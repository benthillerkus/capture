use std::net::SocketAddr;

use axum::{
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use tokio::sync::mpsc;
use tower_http::services::ServeFile;
use tracing::info;

#[derive(vite_rs::Embed)]
#[root = "./frontend"]
#[dev_server_port = 5173]
struct Assets;

struct WebServerActor {
    address: SocketAddr,
    receiver: mpsc::Receiver<WebServerActorMessage>,
}

enum WebServerActorMessage {
    Shutdown,
}

impl WebServerActor {
    async fn handle_message(&mut self, message: WebServerActorMessage) {
        use WebServerActorMessage::*;
        match message {
            Shutdown => {}
        }
    }

    async fn run(mut actor: Self) {
        let app = Router::new()
            .route_service("/output.mp4", ServeFile::new("output.mp4"))
            .fallback(|uri: Uri| async move {
                let asset = Assets::get(&uri.path()[1..]);

                if let Some(asset) = asset {
                    return (
                        StatusCode::OK,
                        [
                            (header::CONTENT_TYPE, asset.content_type),
                            (header::CONTENT_LENGTH, asset.content_length.to_string()),
                        ],
                        asset.bytes,
                    )
                        .into_response();
                }
                (StatusCode::NOT_FOUND, "Not Found").into_response()
            });

        let listener: tokio::net::TcpListener =
            tokio::net::TcpListener::bind(actor.address).await.unwrap();
        info!("listening on http://{}", listener.local_addr().unwrap());
        let server = axum::serve(listener, app);

        tokio::select! {
            _ = server => {}
            _ = async {
                while let Some(message) = actor.receiver.recv().await {
                    actor.handle_message(message).await;
                }
            } => {}
        }
    }
}

#[derive(Clone)]
pub struct WebServerActorHandle {
    sender: mpsc::Sender<WebServerActorMessage>,
}

impl WebServerActorHandle {
    pub fn new(address: SocketAddr) -> Self {
        let (sender, receiver) = mpsc::channel(16);
        let actor = WebServerActor { receiver, address };
        tokio::spawn(WebServerActor::run(actor));
        Self { sender }
    }

    pub async fn shutdown(&self) {
        let _ = self.sender.send(WebServerActorMessage::Shutdown).await;
    }
}
