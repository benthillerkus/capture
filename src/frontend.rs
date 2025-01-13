use std::net::SocketAddr;

use axum::{
    extract::{self},
    routing::{get, post},
    Json, Router,
};
use tokio::sync::mpsc;
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::warn;

use crate::camera::{CameraActorHandle, NullableConfiguration};

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
        let camera3 = actor.camera.clone();
        let camera4 = actor.camera.clone();

        let app = Router::new()
            .nest_service("/gallery", ServeDir::new("gallery"))
            .route(
                "/api/gallery",
                get(|| async {
                    let dir = tokio::fs::read_dir("gallery").await;

                    if dir.is_err() {
                        warn!("failed to read gallery directory");
                        return Json(vec![]);
                    }

                    let mut dir = dir.unwrap();
                    let mut result = vec![];

                    while let Ok(Some(entry)) = dir.next_entry().await {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.ends_with(".mkv")
                            || name.ends_with(".mov")
                            || name.ends_with(".mp4")
                        {
                            result.push(name);
                        }
                    }

                    Json(result)
                }),
            )
            .route(
                "/api/state",
                get(|| async move { Json(camera2.get_state().await) }),
            )
            .route(
                "/api/record",
                post(|extract::Json(payload): extract::Json<bool>| async move {
                    if payload {
                        camera4.start_capture().await;
                    } else {
                        camera4.start_livefeed().await;
                    }

                    Json(camera4.get_state().await)
                }),
            )
            .route(
                "/api/configuration",
                get(|| async move { Json(camera.get_configuration().await) }).post(
                    |extract::Json(payload): extract::Json<NullableConfiguration>| async move {
                        camera3.set_configuration(payload).await;
                        Json(camera3.get_configuration().await)
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
