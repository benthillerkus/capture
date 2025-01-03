use std::sync::Arc;

use camera::{CameraActorHandle, CameraState};
use clap::Parser;
#[cfg(all(feature = "hotspot", not(target_os = "macos")))]
use hotspot::HotspotActorHandle;
use tokio::{fs, process::Command, sync::Mutex};

use axum::{
    response::Html,
    routing::{get, post},
    Extension, Router,
};
use color_eyre::eyre::Result;
use tower_http::services::ServeFile;
use tracing::{info, warn};

mod camera;
mod hotspot;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// The network address and port to listen to.
    #[clap(short = 'a', long = "address", default_value = "0.0.0.0:8080")]
    address: std::net::SocketAddr,

    #[cfg(all(feature = "hotspot", not(target_os = "macos")))]
    #[clap(long)]
    enable_hotspot: bool,

    #[cfg(all(feature = "hotspot", not(target_os = "macos")))]
    #[clap(long, default_value = "hey monte dein aquarium brennt")]
    ssid: String,

    #[cfg(all(feature = "hotspot", not(target_os = "macos")))]
    #[clap(long, default_value = "jajajajaja")]
    password: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();
    let args = Args::try_parse()?;
    info!("starting up");

    let shutdown = tokio::signal::ctrl_c();

    let camera = CameraActorHandle::new();
    let c1 = camera.clone();
    let c2 = camera.clone();
    let c3 = camera.clone();

    let app = Router::new()
        .route(
            "/",
            get(|| async { Html(render_page(c1).await) }).post({
                move || async {
                    info!("received capture");
                    match c2.get_state().await {
                        CameraState::Livefeed | CameraState::Idle => c2.start_capture().await,
                        CameraState::Capture => c2.start_livefeed().await,
                    }
                    Html(render_page(c2).await)
                }
            }),
        )
        .route_service("/output.mp4", ServeFile::new("output.mp4"));

    // c3.start_livefeed().await;

    #[cfg(all(feature = "hotspot", not(target_os = "macos")))]
    let mut hotspot_handle = None;
    #[cfg(all(feature = "hotspot", not(target_os = "macos")))]
    {
        if args.enable_hotspot {
            let hotspot = HotspotActorHandle::new(&args.ssid, &args.password);
            hotspot.start().await;
            hotspot_handle = Some(hotspot);
        }
    }

    let listener = tokio::net::TcpListener::bind(args.address).await?;
    info!("listening on http://{}", listener.local_addr()?);
    let server = axum::serve(listener, app);

    tokio::select! {
        _ = shutdown => {
            info!("received shutdown signal");
            camera.shutdown().await;
        },
        res = server => {
            if let Err(err) = res {
                warn!("server error: {:?}", err);
            }
            warn!("server closed");
        }
    }

    #[cfg(all(feature = "hotspot", not(target_os = "macos")))]
    {
        if let Some(h) = hotspot_handle {
            h.stop().await;
        }
    }

    info!("shutdown complete");

    Ok(())
}

async fn render_page(camera: CameraActorHandle) -> String {
    let state = camera.get_state().await;
    let has_video_file = std::path::Path::new("output.mp4").exists();
    include_str!("../index.html")
        .replace(
            "{video}",
            if has_video_file {
                r#"<video controls width="200px" src="output.mp4" type="video/mp4"></video>"#
            } else {
                ""
            },
        )
        .replace(
            "{app_state}",
            match state {
                CameraState::Idle | CameraState::Livefeed => "Capture",
                CameraState::Capture => "Stop Capturing",
            },
        )
        .to_string()
}
