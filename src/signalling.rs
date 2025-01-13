// Abridged from https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs/-/blob/main/net/webrtc/signalling/src/bin/server.rs

use std::time::Duration;

use color_eyre::eyre::Result;
use gst_plugin_webrtc_signalling::server::Server;
use gst_plugin_webrtc_signalling::{handlers::Handler, server::ServerError};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::{fs, task};
use tokio_native_tls::native_tls;
use tracing::{debug, warn};

const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) async fn run_signalling_server(
    addr: &std::net::SocketAddr,
    cert: &Option<String>,
    cert_password: &Option<String>,
) -> Result<()> {
    let server = Server::spawn(Handler::new);

    // Create the event loop and TCP listener we'll accept connections on.
    let listener = TcpListener::bind(&addr).await?;

    let acceptor = match cert {
        Some(cert) => {
            let mut file = fs::File::open(cert).await?;
            let mut identity = vec![];
            file.read_to_end(&mut identity).await?;
            let identity = tokio_native_tls::native_tls::Identity::from_pkcs12(
                &identity,
                cert_password.as_deref().unwrap_or(""),
            )?;
            Some(tokio_native_tls::TlsAcceptor::from(
                native_tls::TlsAcceptor::new(identity)?,
            ))
        }
        None => None,
    };

    debug!("Listening on: {}", addr);

    while let Ok((stream, address)) = listener.accept().await {
        let mut server_clone = server.clone();
        debug!("Accepting connection from {}", address);

        if let Some(acceptor) = acceptor.clone() {
            tokio::spawn(async move {
                match tokio::time::timeout(TLS_HANDSHAKE_TIMEOUT, acceptor.accept(stream)).await {
                    Ok(Ok(stream)) => server_clone.accept_async(stream).await,
                    Ok(Err(err)) => {
                        warn!("Failed to accept TLS connection from {}: {}", address, err);
                        Err(ServerError::TLSHandshake(err))
                    }
                    Err(elapsed) => {
                        warn!("TLS connection timed out {} after {}", address, elapsed);
                        Err(ServerError::TLSHandshakeTimeout(elapsed))
                    }
                }
            });
        } else {
            task::spawn(async move { server_clone.accept_async(stream).await });
        }
    }

    Ok(())
}
