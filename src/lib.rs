#[cfg(all(feature = "custom_trace", feature = "tower_trace"))]
compile_error!("Please use either 'custom_trace' or 'tower_trace' feature");

#[cfg(feature = "custom_trace")]
mod custom_logger;

#[cfg(feature = "tower_trace")]
mod tower_logger;

mod log_utils;

use log_utils::LogLevel;

use hyper::body::{Body, Bytes};
use hyper::server::conn::http1::{self};
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::future::Future;
use std::net::IpAddr;
use std::{convert::Infallible, net::SocketAddr};
use tokio::net::TcpListener;
use tracing::warn;

pub struct EchoServer {
    listener: TcpListener,
    log_level: LogLevel,
}

impl EchoServer {
    pub async fn new(log_level: u8, port: u16) -> Result<Self, std::io::Error> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        let listener = TcpListener::bind(addr).await?;
        Ok(Self {
            listener,
            log_level: log_level.into(),
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr().unwrap()
    }

    pub async fn run(self) -> Result<(), std::io::Error> {
        let mut connection_id = 0_u64;

        loop {
            let (stream, client_addr) = self.listener.accept().await?;
            let io = TokioIo::new(stream);
            let id = connection_id;
            connection_id += 1;
            let svc = make_service(self.log_level, client_addr.ip(), id);

            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, hyper_util::service::TowerToHyperService::new(svc))
                    .await
                {
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
    B: Body<Data = Bytes>,
{
    use custom_logger::LoggerLayer;

    let svc = tower::ServiceBuilder::new()
        .layer(LoggerLayer::new(log_level, client_ip, id))
        .service_fn(echo);
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
            hyper::body::Incoming,
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
        .service_fn(echo);
    svc
}

async fn echo<B>(request: Request<B>) -> Result<Response<B>, Infallible>
where
    B: Body,
{
    Ok(Response::new(request.into_body()))
}
