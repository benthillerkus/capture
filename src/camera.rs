use gstreamer::prelude::*;
use gstreamer::{ElementFactory, Pipeline, State};
use tokio::{process, sync::mpsc};

struct CameraActor {
    receiver: mpsc::Receiver<CameraActorMessage>,
    current_process: Option<Pipeline>,
    state: CameraState,
}

#[derive(Copy, Clone)]
pub enum CameraState {
    Idle,
    Livefeed,
    Capture,
}

enum CameraActorMessage {
    StartCapture(),
    StartLivefeed(),
    GetState(tokio::sync::oneshot::Sender<CameraState>),
    Shutdown(),
}

impl CameraActor {
    fn new(receiver: mpsc::Receiver<CameraActorMessage>) -> Self {
        Self {
            receiver,
            current_process: None,
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

                if let Some(mut previous) = self.current_process.take() {
                    previous.set_state(State::Null).unwrap();
                }

                let pipeline = Pipeline::new();
                let src = ElementFactory::make("nvarguscamerasrc").build().unwrap();
                let conv = ElementFactory::make("nvvidconv").build().unwrap();
                let enc = ElementFactory::make("x264enc").build().unwrap();
                let parse = ElementFactory::make("h264parse").build().unwrap();
                let mux = ElementFactory::make("qtmux").build().unwrap();
                let sink = ElementFactory::make("filesink").build().unwrap();

                pipeline
                    .add_many([&src, &conv, &enc, &parse, &mux, &sink])
                    .unwrap();
                src.link(&conv).unwrap();
                conv.link(&enc).unwrap();
                enc.link(&parse).unwrap();
                parse.link(&mux).unwrap();
                mux.link(&sink).unwrap();

                sink.set_property("location", "output.mp4");

                pipeline.set_state(State::Playing).unwrap();

                self.current_process = Some(pipeline);
                self.state = CameraState::Capture;
            }
            CameraActorMessage::StartLivefeed() => {
                if let CameraState::Livefeed = self.state {
                    return;
                }

                if let Some(mut previous) = self.current_process.take() {
                    previous.set_state(State::Null).unwrap();
                }

                let pipeline = Pipeline::new();

                #[cfg(target_os = "linux")]
                {
                    let src_left = ElementFactory::make("nvarguscamerasrc")
                        .name("left")
                        .build()
                        .unwrap();
                    let src_right = ElementFactory::make("nvarguscamerasrc")
                        .name("right")
                        .build()
                        .unwrap();
                    let mix = ElementFactory::make("glstereomix")
                        .name("mix")
                        .build()
                        .unwrap();
                    let queue = ElementFactory::make("queue").build().unwrap();
                    let sink = ElementFactory::make("glimagesink").build().unwrap();
                    sink.set_property("output-multiview-downmix-mode", 1);

                    pipeline
                        .add_many([&src_left, &src_right, &mix, &queue, &sink])
                        .unwrap();
                    src_left.link(&mix).unwrap();
                    src_right.link(&mix).unwrap();
                    mix.link(&queue).unwrap();
                    queue.link(&sink).unwrap();

                    pipeline.set_state(State::Playing).unwrap();
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let video = ElementFactory::make("videotestsrc").build().unwrap();
                    let audio = ElementFactory::make("audiotestsrc").build().unwrap();
                    let sink = ElementFactory::make("webrtcsink")
                        .name("ws")
                        .property_from_str("meta", "meta")
                        .build()
                        .unwrap();

                    pipeline.add_many([&video, &audio, &sink]).unwrap();
                    video.link(&sink).unwrap();
                    audio.link(&sink).unwrap();

                    pipeline.set_state(State::Playing).unwrap();
                }

                self.current_process = Some(pipeline);
                self.state = CameraState::Livefeed;
            }
            CameraActorMessage::Shutdown() => {
                if let Some(pipeline) = &mut self.current_process {
                    pipeline.set_state(State::Null).unwrap();
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

    pub async fn shutdown(&self) {
        let _ = self.sender.send(CameraActorMessage::Shutdown()).await;
    }
}

impl Default for CameraActorHandle {
    fn default() -> Self {
        Self::new()
    }
}
