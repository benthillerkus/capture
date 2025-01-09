use tokio::task;

use color_eyre::eyre::Result;
use tracing::{info, warn};

pub(crate) trait Hotspot {
    async fn start(&self) -> Result<()>;

    async fn stop(&self) -> Result<()>;
}

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
use linux::HotspotLinux as PlatformHotspot;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
use windows::HotspotWindows as PlatformHotspot;

struct HotspotActor {
    receiver: tokio::sync::mpsc::Receiver<HotspotMessage>,
    ssid: String,
    password: String,
}

enum HotspotMessage {
    Start(tokio::sync::oneshot::Sender<Result<()>>),
    Stop(tokio::sync::oneshot::Sender<Result<()>>),
}

impl HotspotActor {
    async fn handle_message(&mut self, message: HotspotMessage) {
        let hotspot = PlatformHotspot {
            #[cfg(target_os = "linux")]
            name: "Hotspot",
            ssid: &self.ssid,
            password: &self.password,
        };

        match message {
            HotspotMessage::Start(sender) => {
                info!("starting hotspot");
                let res = hotspot.start().await;
                let _ = sender.send(res);
            }
            HotspotMessage::Stop(sender) => {
                info!("stopping hotspot");
                let res = hotspot.stop().await;
                let _ = sender.send(res);
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
pub struct HotspotActorHandle {
    sender: tokio::sync::mpsc::Sender<HotspotMessage>,
}

impl HotspotActorHandle {
    pub fn new(ssid: &str, password: &str) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(4);
        let actor = HotspotActor {
            receiver,
            ssid: ssid.to_string(),
            password: password.to_string(),
        };

        #[cfg(not(target_os = "windows"))]
        {
            tokio::spawn(HotspotActor::run(actor));
        }
        // Unfortunately, the `NetworkOperatorTetheringManager` API on Windows is not
        // thread-safe, so we have to run it in a way
        // where tokio will not try to move it between threads.
        #[cfg(target_os = "windows")]
        {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            std::thread::spawn(move || {
                // Construct a local task set that can run `!Send` futures.
                let local = task::LocalSet::new();

                // Run the local task set.
                let future = local.run_until(async move {
                    // `spawn_local` ensures that the future is spawned on the local
                    // task set.
                    task::spawn_local(
                        HotspotActor::run(actor), // ...
                    )
                    .await
                    .unwrap();
                });

                rt.block_on(future);
            });
        }

        Self { sender }
    }

    pub async fn start(&self) {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let _ = self.sender.send(HotspotMessage::Start(sender)).await;
        let _ = receiver.await;
    }

    pub async fn stop(&self) {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let _ = self.sender.send(HotspotMessage::Stop(sender)).await;
        let _ = receiver.await;
    }
}
