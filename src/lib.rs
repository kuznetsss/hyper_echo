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

use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty};
use hyper::header::{HeaderName, HeaderValue, CONNECTION, UPGRADE};
pub use log_utils::LogLevel;

use hyper::body::{Body, Bytes};
use hyper::server::conn::http1::{self};
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::error::Error;
use std::future::Future;
use std::net::IpAddr;
use std::{convert::Infallible, net::SocketAddr};
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
            let svc = make_service(self.log_level, client_addr.ip(), id);

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

#[cfg(feature = "custom_trace")]
fn make_service<B>(
    log_level: LogLevel,
    client_ip: IpAddr,
    id: u64,
) -> impl tower::Service<
    Request<B>,
    Response = Response<custom_logger::LoggingBody<B>>,
    Error = Infallible,
    Future = impl Future,
> + Clone
where
    B: Body<Data = hyper::body::Bytes>,
{
    use custom_logger::LoggerLayer;

    let svc = tower::ServiceBuilder::new()
        .layer(LoggerLayer::new(log_level, client_ip, id))
        .service_fn(process_request);
    svc
}

#[cfg(feature = "tower_trace")]
fn make_service(
    log_level: LogLevel,
    client_ip: IpAddr,
    id: u64,
) -> impl tower::Service<
    Request<hyper::body::Incoming>,
    Response = Response<
        tower_http::trace::ResponseBody<
            BoxBody<Bytes, Box<dyn Error + Send + Sync + 'static>>,
            tower_http::classify::NeverClassifyEos<tower_http::classify::ServerErrorsFailureClass>,
            tower_logger::BodyLogger,
        >,
    >,
    Future = impl Future,
    Error = Infallible,
> + Clone
where
{
    use tower_http::trace::TraceLayer;
    use tower_logger::{BodyLogger, OnRequestLogger, OnResponseLogger, SpanMaker};

    let svc = tower::ServiceBuilder::new()
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(SpanMaker::new(client_ip, id))
                .on_request(OnRequestLogger::new(log_level))
                .on_response(OnResponseLogger::new(log_level))
                .on_body_chunk(BodyLogger::new(log_level)),
        )
        .service_fn(process_request);
    svc
}

async fn process_request<B>(
    request: Request<B>,
) -> Result<Response<BoxBody<Bytes, Box<dyn Error + Send + Sync + 'static>>>, Infallible>
where
    B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static,
{
    if is_websocket_upgrade(&request) {
        websocket_upgrade(request).await
    } else {
        echo(request).await
    }
}

async fn websocket_upgrade<B>(
    request: Request<B>,
) -> Result<Response<BoxBody<Bytes, Box<dyn Error + Send + Sync + 'static>>>, Infallible>
where
    B: Send + Sync + 'static,
{
    tokio::task::spawn(async move {
        match hyper::upgrade::on(request).await {
            Ok(_upgraded) => {
                todo!()
            }
            Err(e) => warn!("Error upgrading connection: {e}"),
        }
    });
    let body = Empty::<Bytes>::new().map_err(Into::into);
    let response = Response::builder()
        .header(UPGRADE, HeaderValue::from_static("connection"))
        .header(CONNECTION, HeaderValue::from_static("Upgrade"))
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .body(BoxBody::new(body))
        .unwrap();
    Ok(response)
}

fn is_websocket_upgrade<B>(request: &Request<B>) -> bool {
    let check_header_value = |h: HeaderName, v: &str| {
        request
            .headers()
            .get(h)
            .map_or("", |s| s.to_str().unwrap_or(""))
            == v
    };
    check_header_value(UPGRADE, "websocket") && check_header_value(CONNECTION, "Upgrade")
}

async fn echo<B>(
    request: Request<B>,
) -> Result<Response<BoxBody<Bytes, Box<dyn Error + Send + Sync + 'static>>>, Infallible>
where
    B: Body<Data = Bytes> + Send + Sync + 'static,
    B::Error: Error + Send + Sync + 'static,
{
    let body = request.into_body().map_err(Into::into);
    Ok(Response::new(BoxBody::new(body)))
}
