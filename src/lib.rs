//! # hyper_echo
//! `hyper_echo` provides an async echo server based on tokio, tower and hyper.
//!
//! ## Features
//! - Async and efficient
//! - Always runs on ip `127.0.0.1`
//! - Configurable port
//! - Configurable log level. Could log uri, headers and body of a request.
//! - Supports both HTTP (1 and 2) and WebSocket (WIP)
//! - Two implementations of logging: custom and based on [Trace](https://docs.rs/tower-http/latest/tower_http/trace/struct.Trace.html) from [tower_http](https://docs.rs/tower-http/latest/tower_http/index.html)
//!
//! ## Example
//! ```
//! use hyper_echo::LogLevel;
//!
//! #[tokio::main]
//! async fn main() {
//!   let echo_server = EchoServer::new(LogLevel::Uri, None).await?;
//!   info!("Starting echo server on {}", echo_server.local_addr());
//!   echo_server.run().await
//! }
//! ```
//!
//! ## Logging implementations
//! By default [Trace](https://docs.rs/tower-http/latest/tower_http/trace/struct.Trace.html) based implementation is used.
//! Implementation to use is controlled by crate's features: `tower_trace` (default) and `custom_trace`.
//!
//! There is no real reason to use logging implementation provided by `custom_trace` feature.
//! It was created in educational purposes to practice creating custom tower layers.
//! But if in some case you want to use it, please don't forget to add `default-features = false` if you are using `custom_trace` because
//! it is possible to use only one logging implementation at a time.

#[cfg(all(feature = "custom_trace", feature = "tower_trace"))]
compile_error!("Please use either 'custom_trace' or 'tower_trace' feature");

#[cfg(feature = "custom_trace")]
mod custom_logger;

#[cfg(feature = "tower_trace")]
mod tower_logger;

mod log_utils;
mod service;

pub use log_utils::LogLevel;

use hyper::server::conn::http1::{self};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::warn;

/// Asynchronous echo server supporting HTTP and WebSocket
pub struct EchoServer {
    listener: TcpListener,
    log_level: LogLevel,
}

impl EchoServer {
    /// Create a new [EchoServer] or return an error if the provided port is busy.
    /// - `log_level` - the log level to use for each request
    /// - `port` - the port to run on. If not provided a random free port will be chosen
    pub async fn new(log_level: LogLevel, port: Option<u16>) -> Result<Self, std::io::Error> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port.unwrap_or_default()));

        let listener = TcpListener::bind(addr).await?;
        Ok(Self {
            listener,
            log_level,
        })
    }

    /// Get [std::net::SocketAddr] of the server.
    pub fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr().unwrap()
    }

    /// Run the server.
    pub async fn run(self) -> Result<(), std::io::Error> {
        let mut connection_id = 0_u64;

        loop {
            let (stream, client_addr) = self.listener.accept().await?;
            let io = TokioIo::new(stream);
            let id = connection_id;
            connection_id += 1;
            let svc = service::make_service(self.log_level, client_addr.ip(), id);

            tokio::task::spawn(async move {
                let connection = http1::Builder::new()
                    .serve_connection(io, hyper_util::service::TowerToHyperService::new(svc));
                let connection = connection.with_upgrades();

                if let Err(err) = connection.await {
                    warn!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}
