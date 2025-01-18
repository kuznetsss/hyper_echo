mod logger;

use logger::LoggerLayer;

use hyper::server::conn::http1::{self};
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::sync::atomic::AtomicU64;
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
        let connection_id = AtomicU64::new(0);

        loop {
            let (stream, client_addr) = self.listener.accept().await?;
            let io = TokioIo::new(stream);
            let id = connection_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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

async fn echo<B>(request: Request<B>) -> Result<Response<B>, Infallible> {
    Ok(Response::new(request.into_body()))
}
