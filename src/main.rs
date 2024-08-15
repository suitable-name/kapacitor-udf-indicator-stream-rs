//! Main executable for the Kapacitor UDF server using Unix sockets.
//!
//! This module sets up and runs a server that listens on a Unix socket,
//! processes UDF (User Defined Function) requests for EMA calculation,
//! and handles graceful shutdown on termination signals.

use async_std::{fs, io, main, os::unix::net::UnixListener, stream::StreamExt, sync::Arc, task};
use clap::Parser;
use kapacitor_multi_indicator_stream_udf::handler::accepter::Accepter;
use kapacitor_udf::socket_server::SocketServer;
use libc::{SIGINT, SIGTERM};
use signal_hook_async_std::Signals;
use std::path::PathBuf;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// Command-line arguments for the UDF server.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the Unix socket file.
    #[clap(short, long, default_value = "/tmp/indicator-stream.sock")]
    socket: PathBuf,
}

#[main]
async fn main() -> io::Result<()> {
    // Initialize the tracing subscriber with a maximum log level
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    // Parse command-line arguments
    let args = Args::parse();
    info!("Main() started");

    // Define the path for the Unix socket
    let socket_path = args.socket;

    // Attempt to remove any existing socket file
    match fs::remove_file(&socket_path).await {
        Ok(_) => info!("Removed existing socket file: {:?}", socket_path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            debug!("No existing socket file to delete at: {:?}", socket_path);
        }
        Err(e) => {
            warn!("Error removing socket file: {:?}: {}", socket_path, e);
        }
    }

    // Create a new Unix socket listener
    let listener = UnixListener::bind(&socket_path).await?;

    // Create a new server instance
    let server = Arc::new(SocketServer::new(listener, Accepter::new()));
    let server_clone = Arc::clone(&server);

    // Task for serving requests
    let _serve_handle = task::spawn(async move {
        if let Err(e) = server_clone.serve().await {
            error!("Server error: {}", e);
            std::process::exit(1);
        }
    });

    // Setup signal handler to stop Server on termination signals
    let signal_handle = task::spawn({
        let server = Arc::clone(&server);
        async move {
            let mut signals = Signals::new([SIGINT, SIGTERM]).unwrap();
            while let Some(signal) = signals.next().await {
                match signal {
                    SIGINT | SIGTERM => {
                        info!("Received termination signal: {}", signal);
                        server.stop().await;
                        break;
                    }
                    _ => debug!("Received unhandled signal: {}", signal), // Log unhandled signals
                }
            }
        }
    });

    info!("EMA UDF Server listening on {}", socket_path.display());

    // Wait for either the serve task or the signal handling task to complete
    let _ = futures::join!(signal_handle);

    info!("Server stopped");

    // Ensure the socket file is removed when the server stops
    if let Err(e) = fs::remove_file(&socket_path).await {
        error!("Error removing socket file: {:?}", e);
    }

    Ok(())
}
