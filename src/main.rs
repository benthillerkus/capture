use std::sync::Arc;

use clap::Parser;
use tokio::{process::Command, sync::Mutex};

use axum::{
    response::Html,
    routing::{get, post},
    Extension, Router,
};
use color_eyre::eyre::Result;
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// The network address and port to listen to.
    #[clap(short = 'a', long = "address", default_value = "127.0.0.1:80")]
    address: std::net::SocketAddr,
}

enum AppState {
    Idle,
    Capturing,
    Processing,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();
    let args = Args::try_parse()?;
    info!("starting up");

    let shutdown = tokio::signal::ctrl_c();

    let app_state = Arc::new(Mutex::new(AppState::Idle));

    let app = Router::new().route(
        "/",
        get({
            let state = app_state.clone();
            move || async { Html(render_page(state).await) }
        })
        .post({
            let state = app_state.clone();
            move || async {
                info!("received capture");
                {
                    let mut state = state.lock().await;
                    *state = AppState::Capturing;
                }
                Html(render_page(state).await)
            }
        }),
    );

    // Start hoptspot *before* binding the address
    let hotspot = HotspotActorHandle::new();
    hotspot.start().await;

    let listener = tokio::net::TcpListener::bind(args.address).await?;
    info!("listening on http://{}", listener.local_addr()?);
    let server = axum::serve(listener, app);

    tokio::select! {
        _ = shutdown => {
            info!("received shutdown signal");
        },
        res = server => {
            if let Err(err) = res {
                warn!("server error: {:?}", err);
            }
            warn!("server closed");
        }
    }

    hotspot.stop().await;

    info!("shutdown complete");

    Ok(())
}

async fn render_page(state: Arc<Mutex<AppState>>) -> String {
    let state = state.lock().await;

    include_str!("../index.html")
        .replace(
            "{app_state}",
            match *state {
                AppState::Idle => "Capture",
                AppState::Capturing => "Stop Capturing",
                AppState::Processing => "Processing...",
            },
        )
        .to_string()
}

struct HotspotActor {
    receiver: tokio::sync::mpsc::Receiver<HotspotMessage>,
}

enum HotspotMessage {
    Start(tokio::sync::oneshot::Sender<()>),
    Stop(tokio::sync::oneshot::Sender<()>),
}

impl HotspotActor {
    fn new(receiver: tokio::sync::mpsc::Receiver<HotspotMessage>) -> Self {
        Self { receiver }
    }

    async fn handle_message(&mut self, message: HotspotMessage) {
        match message {
            HotspotMessage::Start(sender) => {
                info!("starting hotspot");
                Command::new("nmcli")
                    .arg("connection")
                    .arg("up")
                    .arg("Hotspot")
                    .spawn()
                    .unwrap()
                    .wait()
                    .await
                    .unwrap();
                let _ = sender.send(());
            }
            HotspotMessage::Stop(sender) => {
                info!("stopping hotspot");
                Command::new("nmcli")
                    .arg("connection")
                    .arg("down")
                    .arg("Hotspot")
                    .spawn()
                    .unwrap()
                    .wait()
                    .await
                    .unwrap();
                let _ = sender.send(());
            }
        }
    }
}

async fn run_hotspot_actor(mut actor: HotspotActor) {
    while let Some(message) = actor.receiver.recv().await {
        actor.handle_message(message).await;
    }
}

#[derive(Clone)]
pub struct HotspotActorHandle {
    sender: tokio::sync::mpsc::Sender<HotspotMessage>,
}

impl HotspotActorHandle {
    pub fn new() -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(8);
        let actor = HotspotActor::new(receiver);
        tokio::spawn(run_hotspot_actor(actor));
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

impl Default for HotspotActorHandle {
    fn default() -> Self {
        Self::new()
    }
}
