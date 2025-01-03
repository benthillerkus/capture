use tokio::{process::{self, Child, Command}, sync::mpsc};

struct CameraActor {
    receiver: mpsc::Receiver<CameraActorMessage>,
    current_process: Option<process::Child>,
    state: CameraState,
}

#[derive(Copy, Clone)]
pub enum CameraState {
    Idle,
    Livefeed,
    Capture
}

enum CameraActorMessage {
    StartCapture(),
    StartLivefeed(),
    GetState(tokio::sync::oneshot::Sender<CameraState>),
    Shutdown()
}

impl CameraActor {
    fn new(receiver: mpsc::Receiver<CameraActorMessage>) -> Self {
        Self { receiver, current_process: None, state: CameraState::Idle }
    }
    
    async fn handle_message(&mut self, message: CameraActorMessage) {
        match message {
            CameraActorMessage::GetState(sender) => {
                let _ = sender.send(self.state);
            }
            CameraActorMessage::StartCapture() => {
                if let CameraState::Capture = self.state {
                    return;
                }
                
                if let Some(mut previous) = self.current_process.take() {
                    Command::new("kill").args(["-2", &previous.id().unwrap().to_string()]).spawn().unwrap().wait().await.unwrap();
                    let _ = previous.wait().await;
                }
                
                let handle = Command::new("gst-launch-1.0")
                    .args(["-v",
                        "nvarguscamerasrc","sensor_id=0","!","nvvidconv","!",
                        "x264enc","!","h264parse","!","qtmux","!","filesink","location=output.mp4","-e"
                    ]).spawn().unwrap();
                
                let _ = self.current_process.insert(handle);
                
                self.state = CameraState::Capture;
            },
            CameraActorMessage::StartLivefeed() => {
                if let CameraState::Livefeed = self.state {
                    return;
                }
                
                if let Some(mut previous) = self.current_process.take() {
                    Command::new("kill").args(["-15", &previous.id().unwrap().to_string()]).spawn().unwrap().wait().await.unwrap();
                    //let _ = previous.wait().await;
                }
                
                let handle = Command::new("gst-launch-1.0")
                    .args(["-v",
                        "nvarguscamerasrc","sensor_id=0","name=left",
                        "nvarguscamerasrc","sensor_id=1","name=right",
                        "glstereomix", "name=mix",
                        "left.", "!",r#"video/x-raw(memory:NVMM),width=(int)1280,height=(int)720,format=(string)NV12,framerate=(fraction)60/1"#,"!","nvvidconv","flip-method=2","!","glupload","!","mix.",
                        "right.","!",r#"video/x-raw(memory:NVMM),width=(int)1280,height=(int)720,format=(string)NV12,framerate=(fraction)60/1"#,"!","nvvidconv","flip-method=2","!","glupload","!","mix.",
                        "mix.","!",r#"video/x-raw(memory:GLMemory)"#,"!",
                        //"queue","!","glimagesink","output-multiview-mode=top-bottom"
                        "queue","!","glimagesink","output-multiview-downmix-mode=1"                        
                    ]).spawn().unwrap();
                
                    let _ = self.current_process.insert(handle);
                    
                    self.state = CameraState::Livefeed;
            },
            CameraActorMessage::Shutdown() => {
                if let Some(child) = &mut self.current_process {
                    let _ = child.kill().await;
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
    sender: mpsc::Sender<CameraActorMessage>
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