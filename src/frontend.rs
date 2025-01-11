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

#[derive(Debug, Deserialize, PartialEq)]
struct Control {
    convergence: Option<XY>,
    record: Option<bool>,
}

#[derive(Debug, Deserialize, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xy_deserialize() {
        let xy: XY = serde_json::from_str(r#"{"x": 1.0, "y": 2.0}"#).unwrap();
        assert_eq!(xy.x, 1.0);
        assert_eq!(xy.y, 2.0);
    }

    #[test]
    fn test_control_deserialize() {
        let control: Control =
            serde_json::from_str(r#"{"convergence": {"x": 1.0, "y": 2.0}, "record": true}"#)
                .unwrap();
        assert!(matches!(control.convergence, Some(XY { x: 1.0, y: 2.0 })));
        assert!(control.record.unwrap());
    }

    #[test]
    fn test_control_deserialize_no_convergence() {
        let control: Control = serde_json::from_str(r#"{"record": true}"#).unwrap();
        assert_eq!(control.convergence, None);
        assert!(control.record.unwrap());
    }

    #[test]
    fn test_control_deserialize_no_record() {
        let control: Control =
            serde_json::from_str(r#"{"convergence": {"x": 1.0, "y": 2.0}}"#).unwrap();
        assert!(matches!(control.convergence, Some(XY { x: 1.0, y: 2.0 })));
        assert_eq!(control.record, None);
    }

    #[test]
    fn test_control_deserialize_empty() {
        let control: Control = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(control.convergence, None);
        assert_eq!(control.record, None);
    }

    #[test]
    fn test_control_deserialize_empty_record() {
        let control: Control = serde_json::from_str(r#"{"record": null}"#).unwrap();
        assert_eq!(control.convergence, None);
        assert_eq!(control.record, None);
    }
}
