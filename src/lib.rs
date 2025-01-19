#[cfg(feature = "custom_trace")]
mod logger;

#[cfg(feature = "custom_trace")]
use logger::LoggerLayer;

#[cfg(all(feature = "custom_trace", feature = "tower_trace"))]
compile_error!("Please use either 'custom_trace' or 'tower_trace' feature");

use hyper::body::Body;

use hyper::server::conn::http1::{self};
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::fmt::Debug;
use std::{convert::Infallible, net::SocketAddr};
use tokio::net::TcpListener;
use tracing::warn;

pub struct EchoServer {
    listener: TcpListener,
    log_level: u8,
}

impl EchoServer {
    pub async fn new(log_level: u8, port: u16) -> Result<Self, std::io::Error> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        let listener = TcpListener::bind(addr).await?;
        Ok(Self {
            listener,
            log_level,
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
            let svc = tower::ServiceBuilder::new()
                .layer(LoggerLayer::new(self.log_level, client_addr.ip(), id))
                .service_fn(echo);

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

async fn echo<B>(request: Request<B>) -> Result<Response<B>, Infallible>
where
    B: Body,
    B::Error: Debug,
{
    Ok(Response::new(request.into_body()))
}
