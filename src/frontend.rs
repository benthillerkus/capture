use std::net::SocketAddr;

use axum::{
    extract::{self, Path},
    http::{header, Method, StatusCode, Uri},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use tokio::{fs::File, sync::mpsc};
use tower_http::{
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
};
use tracing::info;

use crate::camera::CameraActorHandle;

#[derive(Debug, Deserialize)]
struct Control {
    convergence: Option<XY>,
    record: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct XY {
    x: f32,
    y: f32,
}

struct WebServerActor {
    address: SocketAddr,
    receiver: mpsc::Receiver<WebServerActorMessage>,
    camera: CameraActorHandle,
}

enum WebServerActorMessage {
    Shutdown,
}

impl WebServerActor {
    async fn handle_message(&mut self, message: WebServerActorMessage) {
        use WebServerActorMessage::*;
        match message {
            Shutdown => {
                self.receiver.close();
            }
        }
    }

    async fn run(mut actor: Self) {
        let camera = actor.camera.clone();
        let camera2 = actor.camera.clone();
        let app = Router::new()
            .route_service("/capture", ServeDir::new("capture"))
            .route("/api/captures", get(|| async { "[]".to_string() }))
            .route(
                "/api/state",
                get(|| async move { Json(camera2.get_state().await) }),
            )
            .route(
                "/api/control",
                get(|| async { "use HTTP POST" }).post(
                    |extract::Json(payload): extract::Json<Control>| async move {
                        info!("received control: {:?}", payload);
                        if let Some(XY { x, y }) = payload.convergence {
                            camera.set_convergence(x, y).await;
                        }
                        if let Some(record) = payload.record {
                            if record {
                                camera.start_capture().await;
                            } else {
                                camera.start_livefeed().await;
                            }
                        }
                        Json(camera.get_state().await)
                    },
                ),
            )
            .layer(CorsLayer::permissive())
            .fallback_service(ServeDir::new("frontend/dist"));

        let listener: tokio::net::TcpListener =
            tokio::net::TcpListener::bind(actor.address).await.unwrap();
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
    pub fn new(address: SocketAddr, camera: CameraActorHandle) -> Self {
        let (sender, receiver) = mpsc::channel(16);
        let actor = WebServerActor {
            receiver,
            address,
            camera,
        };
        tokio::spawn(WebServerActor::run(actor));
        Self { sender }
    }

    pub async fn shutdown(&self) {
        let _ = self.sender.send(WebServerActorMessage::Shutdown).await;
    }
}
