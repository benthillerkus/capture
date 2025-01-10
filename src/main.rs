use std::time::Duration;

use frontend::WebServerActorHandle;
use rand::{prelude::*, rngs};

use camera::{CameraActorHandle, CameraState};
use clap::Parser;

use axum::{response::Html, routing::get, Router};
use color_eyre::eyre::Result;
use tower_http::services::ServeFile;
use tracing::{info, warn};

mod camera;

#[cfg(all(feature = "hotspot", not(target_os = "macos")))]
mod hotspot;

#[cfg(feature = "signalling")]
mod signalling;

mod frontend;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// The network address and port to listen to.
    #[clap(short = 'a', long = "address", default_value = "0.0.0.0:8080")]
    address: std::net::SocketAddr,

    #[cfg(feature = "signalling")]
    #[clap(long)]
    enable_signalling: bool,

    #[cfg(feature = "signalling")]
    #[clap(long, default_value = "0.0.0.0:8443")]
    signalling_address: std::net::SocketAddr,

    /// TLS certificate to use
    #[clap(short, long)]
    cert: Option<String>,
    /// password to TLS certificate
    #[clap(long)]
    cert_password: Option<String>,

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

    let webserver = WebServerActorHandle::new(args.address);

    c3.start_livefeed().await;

    #[cfg(feature = "signalling")]
    {
        if args.enable_signalling {
            tokio::spawn(async move {
                if let Err(err) = signalling::run_signalling_server(
                    &args.signalling_address,
                    &args.cert,
                    &args.cert_password,
                )
                .await
                {
                    warn!("signalling server error: {:?}", err);
                }
            });
        }
    }

    #[cfg(all(feature = "hotspot", not(target_os = "macos")))]
    let mut hotspot_handle = None;
    #[cfg(all(feature = "hotspot", not(target_os = "macos")))]
    {
        if args.enable_hotspot {
            let hotspot = hotspot::HotspotActorHandle::new(&args.ssid, &args.password);
            hotspot.start().await;
            hotspot_handle = Some(hotspot);
        }
    }

    // DEBUG simulate convergence changes
    {
        let camera = camera.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let (x, y) = {
                    let mut rng = rand::thread_rng();
                    let x: f32 = rand::distributions::Open01.sample(&mut rng);
                    let y: f32 = rand::distributions::Open01.sample(&mut rng);
                    (x, y)
                };
                camera.set_convergence(x, y).await;
            }
        });
    }

    tokio::select! {
        _ = shutdown => {
            info!("received shutdown signal");
            camera.shutdown().await;
            webserver.shutdown().await;
        },
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
