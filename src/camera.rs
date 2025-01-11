use std::str::FromStr;

use gstreamer::{prelude::*, Element};
use gstreamer::{ElementFactory, Pipeline, State};
use serde::Serialize;
use time::{format_description, OffsetDateTime};
use tokio::sync::mpsc;
use tracing::info;

struct CameraActor {
    receiver: mpsc::Receiver<CameraActorMessage>,
    pipeline: Option<Pipeline>,
    controls: Option<Controls>,
    state: CameraState,
}

enum Controls {
    Capture {},
    Livefeed {
        left_transform: Element,
        right_transform: Element,
    },
}

#[derive(Copy, Clone, Serialize)]
pub enum CameraState {
    Idle,
    Livefeed,
    Capture,
}

enum CameraActorMessage {
    StartCapture(),
    StartLivefeed(),
    GetState(tokio::sync::oneshot::Sender<CameraState>),
    SetConvergence((f32, f32)),
    Shutdown(),
}

impl CameraActor {
    fn new(receiver: mpsc::Receiver<CameraActorMessage>) -> Self {
        Self {
            receiver,
            pipeline: None,
            controls: None,
            state: CameraState::Idle,
        }
    }

    async fn handle_message(&mut self, message: CameraActorMessage) {
        gstreamer::init().unwrap();

        match message {
            CameraActorMessage::GetState(sender) => {
                let _ = sender.send(self.state);
            }
            CameraActorMessage::StartCapture() => {
                if let CameraState::Capture = self.state {
                    return;
                }

                info!("starting capture");

                if let Some(previous) = self.pipeline.take() {
                    previous.set_state(State::Null).unwrap();
                }
                self.controls = None;

                let pipeline = Pipeline::new();

                let left_src: Element;
                let right_src: Element;
                let caps: gstreamer::Caps;
                let left_enc: Element;
                let right_enc: Element;

                #[cfg(target_os = "linux")]
                {
                    left_src = ElementFactory::make("nvarguscamerasrc")
                        .name("left_src")
                        .property_from_str("sensor_id", "0")
                        .build()
                        .unwrap();

                    right_src = ElementFactory::make("nvarguscamerasrc")
                        .name("right_src")
                        .property_from_str("sensor_id", "1")
                        .build()
                        .unwrap();

                    caps = gstreamer::Caps::from_str("video/x-raw(memory:NVMM),width=(int)1920,height=(int)1080,format=(string)RGB,framerate=(fraction)60/1").unwrap();

                    left_enc = ElementFactory::make("nvjpegenc")
                        .property_from_str("quality", "95")
                        .build()
                        .unwrap();
                    right_enc = ElementFactory::make("nvjpegenc")
                        .property_from_str("quality", "95")
                        .build()
                        .unwrap();
                }
                #[cfg(not(target_os = "linux"))]
                {
                    left_src = ElementFactory::make("videotestsrc").build().unwrap();
                    right_src = ElementFactory::make("videotestsrc").build().unwrap();
                    caps = gstreamer::Caps::builder("video/x-raw")
                        .field("width", 1280)
                        .field("height", 720)
                        .field("format", "NV12")
                        .field("framerate", gstreamer::Fraction::new(30, 1))
                        .build();
                    left_enc = ElementFactory::make("jpegenc").build().unwrap();
                    right_enc = ElementFactory::make("jpegenc").build().unwrap();
                }

                let left_queue = ElementFactory::make("queue").build().unwrap();
                let right_queue = ElementFactory::make("queue").build().unwrap();

                let left_mux = ElementFactory::make("matroskamux").build().unwrap();
                let right_mux = ElementFactory::make("matroskamux").build().unwrap();

                let left_sink = ElementFactory::make("filesink").build().unwrap();
                let right_sink = ElementFactory::make("filesink").build().unwrap();

                pipeline
                    .add_many([
                        &left_src,
                        &left_enc,
                        &left_queue,
                        &right_src,
                        &right_enc,
                        &right_queue,
                        &left_mux,
                        &right_mux,
                        &left_sink,
                        &right_sink,
                    ])
                    .unwrap();

                left_src.link_filtered(&left_enc, &caps).unwrap();
                right_src.link_filtered(&right_enc, &caps).unwrap();

                right_enc.link(&right_queue).unwrap();
                left_enc.link(&left_queue).unwrap();
                right_queue.link(&right_mux).unwrap();
                left_queue.link(&left_mux).unwrap();

                left_mux.link(&left_sink).unwrap();
                right_mux.link(&right_sink).unwrap();

                let format =
                    format_description::parse("[year]-[month]-[day] [hour]-[minute]-[second]")
                        .unwrap();
                let now = OffsetDateTime::now_utc().format(&format).unwrap();

                left_sink.set_property("location", format!("capture/{now} left.mkv"));
                right_sink.set_property("location", format!("capture/{now} right.mkv"));

                pipeline.set_state(State::Playing).unwrap();

                self.pipeline = Some(pipeline);
                self.state = CameraState::Capture;
                
                info!("capture started");
            }
            CameraActorMessage::StartLivefeed() => {
                if let CameraState::Livefeed = self.state {
                    return;
                }

                info!("starting livefeed");

                if let Some(previous) = self.pipeline.take() {
                    previous.set_state(State::Null).unwrap();
                }
                self.controls = None;

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
                        .build()
                        .unwrap();

                    right_src = ElementFactory::make("nvarguscamerasrc")
                        .name("right_src")
                        .property_from_str("sensor_id", "1")
                        .build()
                        .unwrap();

                    caps = gstreamer::Caps::from_str("video/x-raw(memory:NVMM),width=(int)1280,height=(int)720,format=(string)NV12,framerate=(fraction)60/1").unwrap();

                    left_conv = ElementFactory::make("nvvidconv")
                        .property_from_str("flip-method", "2")
                        .build()
                        .unwrap();
                    right_conv = ElementFactory::make("nvvidconv")
                        .property_from_str("flip-method", "2")
                        .build()
                        .unwrap();
                }
                #[cfg(not(target_os = "linux"))]
                {
                    left_src = ElementFactory::make("videotestsrc").build().unwrap();
                    right_src = ElementFactory::make("videotestsrc").build().unwrap();
                    caps = gstreamer::Caps::builder("video/x-raw")
                        .field("width", 1280)
                        .field("height", 720)
                        .field("format", "NV12")
                        .field("framerate", gstreamer::Fraction::new(60, 1))
                        .build();
                    left_conv = ElementFactory::make("videoconvert").build().unwrap();
                    right_conv = ElementFactory::make("videoconvert").build().unwrap();
                }
                let mix_caps = gstreamer::Caps::from_str(
                    "video/x-raw(memory:GLMemory),multiview-mode=top-bottom",
                )
                .unwrap();
                //let mix_caps = gstreamer::Caps::from_str("video/x-raw(memory:GLMemory),downmix-mode=0").unwrap();

                let mix = ElementFactory::make("glstereomix")
                    .name("mix")
                    .build()
                    .unwrap();

                let left_glupload = ElementFactory::make("glupload").build().unwrap();
                let right_glupload = ElementFactory::make("glupload").build().unwrap();

                let left_transform = ElementFactory::make("gltransformation")
                    .property("translation-x", 0.0f32)
                    .build()
                    .unwrap();
                let right_transform = ElementFactory::make("gltransformation")
                    .property("translation-x", -0.1f32)
                    .build()
                    .unwrap();

                let glviewconvert = ElementFactory::make("glviewconvert")
                    .property_from_str("output-mode-override", "mono")
                    .property_from_str("downmix-mode", "1")
                    .build()
                    .unwrap();

                let queue = ElementFactory::make("queue").name("name").build().unwrap();
                let gldownload = ElementFactory::make("gldownload")
                    .name("gldownload")
                    .build()
                    .unwrap();
                let sink = ElementFactory::make("webrtcsink")
                    .name("sink")
                    .property_from_str("meta", "meta")
                    .build()
                    .unwrap();

                pipeline
                    .add_many([
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
                    ])
                    .unwrap();

                left_src.link_filtered(&left_conv, &caps).unwrap();

                left_conv.link(&left_glupload).unwrap();
                left_glupload.link(&left_transform).unwrap();
                left_transform.link(&mix).unwrap();

                right_src.link_filtered(&right_conv, &caps).unwrap();

                right_conv.link(&right_glupload).unwrap();
                right_glupload.link(&right_transform).unwrap();
                right_transform.link(&mix).unwrap();

                mix.link_filtered(&glviewconvert, &mix_caps).unwrap();
                glviewconvert.link(&queue).unwrap();
                queue.link(&gldownload).unwrap();
                gldownload.link(&sink).unwrap();

                pipeline.set_state(State::Playing).unwrap();

                self.controls = Some(Controls::Livefeed {
                    left_transform,
                    right_transform,
                });

                self.pipeline = Some(pipeline);
                self.state = CameraState::Livefeed;

                info!("livefeed started");
            }
            CameraActorMessage::SetConvergence((x, y)) => {
                if let Some(Controls::Livefeed {
                    left_transform,
                    right_transform,
                }) = &self.controls
                {
                    left_transform.set_property("translation-x", x / 2f32);
                    left_transform.set_property("translation-y", y / 2f32);
                    right_transform.set_property("translation-x", -x / 2f32);
                    right_transform.set_property("translation-y", -y / 2f32);
                }
            }
            CameraActorMessage::Shutdown() => {
                self.receiver.close();
                if let Some(pipeline) = &mut self.pipeline {
                    pipeline.set_state(State::Null).unwrap();
                    self.pipeline = None;
                }
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

    pub async fn start_capture(&self) {
        let _ = self.sender.send(CameraActorMessage::StartCapture()).await;
    }

    pub async fn start_livefeed(&self) {
        let _ = self.sender.send(CameraActorMessage::StartLivefeed()).await;
    }

    pub async fn set_convergence(&self, x: f32, y: f32) {
        let _ = self
            .sender
            .send(CameraActorMessage::SetConvergence((x, y)))
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
