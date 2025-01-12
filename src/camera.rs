use std::str::FromStr;

use color_eyre::Result;
use configuration::VideoCodec;
use gstreamer::{event, message, prelude::*, Element, MessageType};
use gstreamer::{ElementFactory, Pipeline, State};
use serde::{Deserialize, Serialize};
use time::{format_description, OffsetDateTime};
use tokio::sync::mpsc;
use tracing::info;

mod configuration;
pub use configuration::{Configuration, NullableConfiguration};

struct CameraActor {
    receiver: mpsc::Receiver<CameraActorMessage>,
    pipeline: Option<Pipeline>,
    controls: Option<Controls>,
    state: CameraState,
    /// The current configuration of the camera.
    /// Some fields may be ignored depending on the state of the camera.
    configuration: Configuration,
}

enum Controls {
    Capture {},
    Livefeed {
        left_transform: Element,
        right_transform: Element,
        glviewconvert: Element,
    },
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Default)]
pub enum CameraState {
    #[default]
    Idle,
    Livefeed,
    Capture,
}

enum CameraActorMessage {
    StartCapture(),
    StartLivefeed(),
    GetState(tokio::sync::oneshot::Sender<CameraState>),
    GetConfiguration(tokio::sync::oneshot::Sender<Configuration>),
    SetConfiguration(NullableConfiguration),
    Shutdown(),
}

impl CameraActor {
    fn new(receiver: mpsc::Receiver<CameraActorMessage>) -> Self {
        Self {
            receiver,
            pipeline: None,
            controls: None,
            state: CameraState::Idle,
            configuration: Configuration::default(),
        }
    }

    /// Provides a graceful shutdown of the current pipeline.
    /// Similar to gst-launch-1.0 with the -e flag.
    async fn clear_pipeline(&mut self) -> Result<()> {
        if let Some(previous) = self.pipeline.take() {
            let shutdown = tokio::task::spawn_blocking(|| async move {
                previous.send_event(event::Eos::new());

                if let Some(bus) = previous.bus() {
                    if let Some(_message) =
                        bus.timed_pop_filtered(None, &[MessageType::Eos, MessageType::Error])
                    {
                    }
                }

                previous.set_state(State::Null).unwrap();
            });
            shutdown.await?.await;
        }
        self.controls = None;

        Ok(())
    }

    async fn start_capture(&mut self) -> Result<()> {
        if let CameraState::Capture = self.state {
            return Ok(());
        }

        info!("starting capture");

        self.clear_pipeline().await?;

        let pipeline = Pipeline::new();

        let left_src: Element;
        let right_src: Element;
        let caps: gstreamer::Caps;
        let left_conv: Element;
        let right_conv: Element;
        let left_enc: Element;
        let right_enc: Element;

        #[cfg(target_os = "linux")]
        {
            left_src = ElementFactory::make("nvarguscamerasrc")
                .name("left_src")
                .property("sensor_id", 0)
                .property_from_str("tnr-mode", "0")
                .property_from_str("ee-mode", "0")
                .property("awblock", true)
                .build()?;

            right_src = ElementFactory::make("nvarguscamerasrc")
                .name("right_src")
                .property("sensor_id", 1)
                .property_from_str("tnr-mode", "0")
                .property_from_str("ee-mode", "0")
                .property("awblock", true)
                .build()?;

            caps = gstreamer::Caps::from_str(&format!("video/x-raw(memory:NVMM),width=(int){},height=(int){},format=(string){},framerate=(fraction){}/1", self.configuration.width, self.configuration.height, self.configuration.format, self.configuration.fps))?;

            left_conv = ElementFactory::make("nvvidconv")
                .property_from_str("flip-method", "2")
                .build()?;
            right_conv = ElementFactory::make("nvvidconv")
                .property_from_str("flip-method", "2")
                .build()?;

            match self.configuration.codec {
                VideoCodec::Prores => {
                    left_enc = ElementFactory::make("avenc_prores")
                        .build()?;
                    right_enc = ElementFactory::make("avenc_prores")
                        .build()?;
                }
                VideoCodec::MotionJpeg => {
                    left_enc = ElementFactory::make("jpegenc")
                        .property("quality", 95)
                        .build()?;
                    right_enc = ElementFactory::make("jpegenc")
                        .property("quality", 95)
                        .build()?;
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            left_src = ElementFactory::make("videotestsrc").build()?;
            right_src = ElementFactory::make("videotestsrc").build()?;
            caps = gstreamer::Caps::builder("video/x-raw")
                .field("width", self.configuration.width as i32)
                .field("height", self.configuration.height as i32)
                .field("format", self.configuration.format.to_string())
                .field(
                    "framerate",
                    gstreamer::Fraction::new(self.configuration.fps as i32, 1),
                )
                .build();
            left_conv = ElementFactory::make("identity").build()?;
            right_conv = ElementFactory::make("identity").build()?;
            match self.configuration.codec {
                VideoCodec::Prores => {
                    left_enc = ElementFactory::make("avenc_prores").build()?;
                    right_enc = ElementFactory::make("avenc_prores").build()?;
                }
                VideoCodec::MotionJpeg => {
                    left_enc = ElementFactory::make("jpegenc")
                        .property("quality", 95)
                        .build()?;
                    right_enc = ElementFactory::make("jpegenc")
                        .property("quality", 95)
                        .build()?;
                }
            }
        }

        let left_queue = ElementFactory::make("queue").build()?;
        let right_queue = ElementFactory::make("queue").build()?;

        let left_videoconvert = ElementFactory::make("videoconvert").build()?;
        let right_videoconvert = ElementFactory::make("videoconvert").build()?;

        let left_mux: Element;
        let right_mux: Element;
        match self.configuration.codec {
            VideoCodec::Prores => {
                left_mux = ElementFactory::make("qtmux").build()?;
                right_mux = ElementFactory::make("qtmux").build()?;
            }
            VideoCodec::MotionJpeg => {
                left_mux = ElementFactory::make("matroskamux").build()?;
                right_mux = ElementFactory::make("matroskamux").build()?;
            }
        }

        let left_sink = ElementFactory::make("filesink").build()?;
        let right_sink = ElementFactory::make("filesink").build()?;

        pipeline.add_many([
            &left_src,
            &left_conv,
            &left_queue,
            &left_videoconvert,
            &left_enc,
            &right_src,
            &right_conv,
            &right_queue,
            &right_videoconvert,
            &right_enc,
            &left_mux,
            &right_mux,
            &left_sink,
            &right_sink,
        ])?;

        left_src.link_filtered(&left_conv, &caps)?;
        left_conv.link(&left_queue)?;
        left_queue.link(&left_videoconvert)?;
        left_videoconvert.link(&left_enc)?;
        left_enc.link(&left_mux)?;

        right_src.link_filtered(&right_conv, &caps)?;
        right_conv.link(&right_queue)?;
        right_queue.link(&right_videoconvert)?;
        right_videoconvert.link(&right_enc)?;
        right_enc.link(&right_mux)?;

        left_mux.link(&left_sink)?;
        right_mux.link(&right_sink)?;

        let format = format_description::parse("[year]-[month]-[day] [hour]-[minute]-[second]")?;
        let now = OffsetDateTime::now_utc().format(&format)?;

        let ext = match self.configuration.codec {
            VideoCodec::Prores => "mov",
            VideoCodec::MotionJpeg => "mkv",
        };
        left_sink.set_property("location", format!("gallery/{now} left.{ext}"));
        right_sink.set_property("location", format!("gallery/{now} right.{ext}"));

        pipeline.set_state(State::Playing)?;

        self.pipeline = Some(pipeline);
        self.state = CameraState::Capture;

        info!("capture started");

        Ok(())
    }

    async fn start_livefeed(&mut self) -> Result<()> {
        info!("starting livefeed");

        self.clear_pipeline().await?;

        let pipeline = Pipeline::new();

        let left_src: Element;
        let right_src: Element;
        let caps: gstreamer::Caps;
        let left_conv: Element;
        let right_conv: Element;
        #[cfg(target_os = "linux")]
        {
            left_src = ElementFactory::make("nvarguscamerasrc")
                .name("left_src")
                .property_from_str("sensor_id", "0")
                .build()?;

            right_src = ElementFactory::make("nvarguscamerasrc")
                .name("right_src")
                .property_from_str("sensor_id", "1")
                .build()?;

            caps = gstreamer::Caps::from_str(&format!("video/x-raw(memory:NVMM),width=(int){},height=(int){},format=(string){},framerate=(fraction){}/1", self.configuration.width, self.configuration.height, self.configuration.format, self.configuration.fps))?;

            left_conv = ElementFactory::make("nvvidconv")
                .property_from_str("flip-method", "2")
                .build()?;
            right_conv = ElementFactory::make("nvvidconv")
                .property_from_str("flip-method", "2")
                .build()?;
        }
        #[cfg(not(target_os = "linux"))]
        {
            left_src = ElementFactory::make("videotestsrc")
                .property_from_str("pattern", "ball")
                .property_from_str("motion", "hsweep")
                .build()?;
            right_src = ElementFactory::make("videotestsrc")
                .property_from_str("pattern", "ball")
                .property_from_str("motion", "sweep")
                .build()?;
            caps = gstreamer::Caps::builder("video/x-raw")
                .field("width", self.configuration.width as i32)
                .field("height", self.configuration.height as i32)
                .field("format", self.configuration.format.to_string())
                .field(
                    "framerate",
                    gstreamer::Fraction::new(self.configuration.fps as i32, 1),
                )
                .build();
            left_conv = ElementFactory::make("videoconvert").build()?;
            right_conv = ElementFactory::make("videoconvert").build()?;
        }
        let mix_caps = gstreamer::Caps::from_str("video/x-raw(memory:GLMemory)")?;

        let mix = ElementFactory::make("glstereomix").name("mix").build()?;

        let left_glupload = ElementFactory::make("glupload").build()?;
        let right_glupload = ElementFactory::make("glupload").build()?;

        let left_transform = ElementFactory::make("gltransformation")
            .property("translation-x", self.configuration.convergence.0 / 2f32)
            .property("translation-y", self.configuration.convergence.1 / 2f32)
            .build()?;
        let right_transform = ElementFactory::make("gltransformation")
            .property("translation-x", -self.configuration.convergence.0 / 2f32)
            .property("translation-y", -self.configuration.convergence.1 / 2f32)
            .build()?;

        let glviewconvert = ElementFactory::make("glviewconvert")
            .property(
                "output-mode-override",
                self.configuration.multiview_mode.as_gst(),
            )
            .property_from_str(
                "downmix-mode",
                self.configuration.anaglyph_format.as_gst_str(),
            )
            .build()?;

        let queue = ElementFactory::make("queue").name("name").build()?;
        let gldownload = ElementFactory::make("gldownload")
            .name("gldownload")
            .build()?;
        let sink = ElementFactory::make("webrtcsink")
            .name("sink")
            .property_from_str("meta", "meta")
            .property_from_str("congestion-control", "0")
            .property_from_str("stun-server", "")
            .build()?;

        pipeline.add_many([
            &left_src,
            &left_conv,
            &left_glupload,
            &left_transform,
            &right_src,
            &right_conv,
            &right_glupload,
            &right_transform,
            &mix,
            &queue,
            &glviewconvert,
            &gldownload,
            &sink,
        ])?;

        left_src.link_filtered(&left_conv, &caps)?;

        left_conv.link(&left_glupload)?;
        left_glupload.link(&left_transform)?;
        left_transform.link(&mix)?;

        right_src.link_filtered(&right_conv, &caps)?;

        right_conv.link(&right_glupload)?;
        right_glupload.link(&right_transform)?;
        right_transform.link(&mix)?;

        mix.link_filtered(&glviewconvert, &mix_caps)?;
        glviewconvert.link(&queue)?;
        queue.link(&gldownload)?;
        gldownload.link(&sink)?;

        pipeline.set_state(State::Playing)?;

        self.controls = Some(Controls::Livefeed {
            left_transform,
            right_transform,
            glviewconvert,
        });

        self.pipeline = Some(pipeline);
        self.state = CameraState::Livefeed;

        info!("livefeed started");

        Ok(())
    }

    async fn handle_message(&mut self, message: CameraActorMessage) {
        gstreamer::init().unwrap();

        match message {
            CameraActorMessage::GetState(sender) => {
                let _ = sender.send(self.state);
            }
            CameraActorMessage::StartCapture() => {
                self.start_capture().await.unwrap();
            }
            CameraActorMessage::StartLivefeed() => {
                if let CameraState::Livefeed = self.state {
                    return;
                }

                self.start_livefeed().await.unwrap();
            }
            CameraActorMessage::SetConfiguration(configuration) => {
                if let Some(Controls::Livefeed {
                    left_transform,
                    right_transform,
                    glviewconvert,
                }) = &self.controls
                {
                    let mut needs_restarting = false;
                    info!(
                        "updating configuration to {configuration:?} from {:?}",
                        self.configuration
                    );
                    if let Some(convergence) = configuration.convergence {
                        if convergence != self.configuration.convergence {
                            let (x, y) = convergence;
                            left_transform.set_property("translation-x", x / 2f32);
                            left_transform.set_property("translation-y", y / 2f32);
                            right_transform.set_property("translation-x", -x / 2f32);
                            right_transform.set_property("translation-y", -y / 2f32);
                        }
                    }
                    if let Some(multiview_mode) = configuration.multiview_mode {
                        if multiview_mode != self.configuration.multiview_mode {
                            needs_restarting = true;
                            // glviewconvert
                            // .set_property("output-mode-override", multiview_mode.as_gst());
                        }
                    }
                    if let Some(anaglyph_format) = configuration.anaglyph_format {
                        if anaglyph_format != self.configuration.anaglyph_format {
                            glviewconvert.set_property_from_str(
                                "downmix-mode",
                                anaglyph_format.as_gst_str(),
                            );
                        }
                    }
                    if let Some(width) = configuration.width {
                        if width != self.configuration.width {
                            needs_restarting = true;
                        }
                    }
                    if let Some(height) = configuration.height {
                        if height != self.configuration.height {
                            needs_restarting = true;
                        }
                    }
                    if let Some(fps) = configuration.fps {
                        if fps != self.configuration.fps {
                            needs_restarting = true;
                        }
                    }
                    if let Some(format) = configuration.format {
                        if format != self.configuration.format {
                            needs_restarting = true;
                        }
                    }
                    self.configuration = self.configuration.merge(&configuration);

                    if needs_restarting && self.state == CameraState::Livefeed {
                        self.start_livefeed().await.unwrap();
                    }
                }
            }
            CameraActorMessage::GetConfiguration(sender) => {
                let _ = sender.send(self.configuration);
            }
            CameraActorMessage::Shutdown() => {
                self.receiver.close();
                self.clear_pipeline().await.unwrap();
                self.state = CameraState::Idle;
            }
        }
    }

    async fn run(mut actor: Self) {
        while let Some(message) = actor.receiver.recv().await {
            actor.handle_message(message).await;
        }
    }
}

#[derive(Clone)]
pub struct CameraActorHandle {
    sender: mpsc::Sender<CameraActorMessage>,
}

impl CameraActorHandle {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(4);
        let actor = CameraActor::new(receiver);
        tokio::spawn(CameraActor::run(actor));
        Self { sender }
    }

    pub async fn get_state(&self) -> CameraState {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let _ = self.sender.send(CameraActorMessage::GetState(sender)).await;
        receiver.await.unwrap()
    }

    pub async fn get_configuration(&self) -> Configuration {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let _ = self
            .sender
            .send(CameraActorMessage::GetConfiguration(sender))
            .await;
        receiver.await.unwrap()
    }

    pub async fn start_capture(&self) {
        let _ = self.sender.send(CameraActorMessage::StartCapture()).await;
    }

    pub async fn start_livefeed(&self) {
        let _ = self.sender.send(CameraActorMessage::StartLivefeed()).await;
    }

    pub async fn set_configuration(&self, configuration: NullableConfiguration) {
        let _ = self
            .sender
            .send(CameraActorMessage::SetConfiguration(configuration))
            .await;
    }

    pub async fn shutdown(&self) {
        let _ = self.sender.send(CameraActorMessage::Shutdown()).await;
    }
}

impl Default for CameraActorHandle {
    fn default() -> Self {
        Self::new()
    }
}
