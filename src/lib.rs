//! # hyper_echo
//! `hyper_echo` provides an async echo server based on tokio, tower, hyper and fastwebsockets.
//!
//! ## Features
//! - Async and efficient
//! - Always runs on ip `127.0.0.1`
//! - Configurable port
//! - Supports both HTTP (versions 1 and 2) and WebSocket
//! - Configurable HTTP log level. Could log uri, headers and body of a request.
//! - Two implementations of http logging: a custom one and one based on [Trace](https://docs.rs/tower-http/latest/tower_http/trace/struct.Trace.html) from [tower_http](https://docs.rs/tower-http/latest/tower_http/index.html)
//! - Logging of WebSocket messages (if enabled)
//! - Configurable ping interval for WebSocket connections and automatic disconnection of inactive clients
//! - Supports graceful shutdown by cancellation token
//!
//! ## Example
//! ```no_run
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!   let echo_server = hyper_echo::EchoServer::new(None, hyper_echo::HttpLogLevel::None, false).await?;
//!   println!("Starting echo server on {}", echo_server.local_addr());
//!   let cancellation_token = tokio_util::sync::CancellationToken::new();
//!   tokio::spawn({
//!     let cancellation_token = cancellation_token.clone();
//!     async move {
//!         let _guard = cancellation_token.drop_guard();
//!         let _ = tokio::signal::ctrl_c().await;
//!     }
//!   });
//!   echo_server.run(cancellation_token).await.map_err(Into::into)
//! }
//! ```
//! ## HTTP logging implementation
//! There are two crate's features controlling HTTP logging:
//! - `tower_trace` (default) is based on [Trace](https://docs.rs/tower-http/latest/tower_http/trace/struct.Trace.html) from [tower_http](https://docs.rs/tower-http/latest/tower_http/index.html) crate
//! - `custom_trace` written from scratch logging layer for tower service
//!
//! Both implementations are almost identical from a user perspective.
//! There is no real reason to use logging implementation provided by `custom_trace` feature.
//! It was created to learn how to create a custom tower layer and how to handle multiple features in one crate.
//! But if in some case you want to use it, please don't forget to add `default-features = false` if you are using `custom_trace` because
//! it is possible to use only one logging implementation at a time.

#[cfg(all(feature = "custom_trace", feature = "tower_trace"))]
compile_error!("Please use either 'custom_trace' or 'tower_trace' feature");

#[cfg(feature = "custom_trace")]
mod custom_logger;

#[cfg(feature = "tower_trace")]
mod http_loggers;

mod log_utils;
mod service;
mod ws_logger;

pub use log_utils::HttpLogLevel;

use hyper_util::rt::TokioIo;
use std::{net::SocketAddr, pin::pin};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;
use tracing::warn;

/// Asynchronous echo server supporting HTTP and WebSocket
pub struct EchoServer {
    listener: TcpListener,
    http_log_level: HttpLogLevel,
    ws_logging_enabled: bool,
    ws_ping_interval: Option<std::time::Duration>,
}

impl EchoServer {
    /// Create a new [EchoServer] or return an error if the provided port is busy.
    /// - `http_log_level` - the log level for http requests to use for each request
    /// - `port` - the port to run on. If not provided a random free port will be chosen
    /// - `ws_logging_enabled` - whether websocket messages and events should be logged or not
    ///
    /// Returns created [EchoServer] or an error (e.g. if the provided port is already taken).
    pub async fn new(
        port: Option<u16>,
        http_log_level: HttpLogLevel,
        ws_logging_enabled: bool,
    ) -> Result<Self, std::io::Error> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port.unwrap_or_default()));

        let listener = TcpListener::bind(addr).await?;
        Ok(Self {
            listener,
            http_log_level,
            ws_logging_enabled,
            ws_ping_interval: None,
        })
    }

    /// Get [std::net::SocketAddr] of the server.
    pub fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr().unwrap()
    }

    /// Set ping interval for WebSocket connections
    /// - `ping_interval` - duration between pings or none to disable pings
    pub fn set_ws_ping_interval(&mut self, ping_interval: Option<std::time::Duration>) {
        self.ws_ping_interval = ping_interval;
    }

    /// Run the server.
    /// - `cancellation_token` - the cancellation_token to stop the server
    ///
    /// Returns `()` or an error if something went wrong.
    pub async fn run(self, cancellation_token: CancellationToken) -> Result<(), std::io::Error> {
        let mut connection_id = 0_u64;

        loop {
            let Some(conn) = cancellation_token
                .run_until_cancelled(self.listener.accept())
                .await
            else {
                break;
            };

            let (stream, client_addr) = conn?;
            self.process_connection(
                stream,
                client_addr,
                connection_id,
                cancellation_token.clone(),
            );
            connection_id += 1;
        }
        Ok(())
    }

    fn process_connection(
        &self,
        stream: TcpStream,
        client_addr: SocketAddr,
        id: u64,
        cancellation_token: CancellationToken,
    ) {
        let io = TokioIo::new(stream);
        let svc = service::make_service(
            self.http_log_level,
            self.ws_logging_enabled,
            self.ws_ping_interval,
            client_addr.ip(),
            id,
            cancellation_token.clone(),
        );

        tokio::task::spawn(async move {
            let executor = hyper_util::rt::TokioExecutor::new();
            let builder = hyper_util::server::conn::auto::Builder::new(executor);
            let connection = builder.serve_connection_with_upgrades(
                io,
                hyper_util::service::TowerToHyperService::new(svc),
            );
            let mut connection = pin!(connection);

            match cancellation_token
                .run_until_cancelled(connection.as_mut())
                .await
            {
                Some(res) => {
                    if let Err(e) = res {
                        warn!("Error processing connection: {e}");
                    }
                }
                None => {
                    connection.as_mut().graceful_shutdown();
                    let _ = connection.await;
                }
            }
        });
    }
}
