use http_body_util::Full;
use hyper::server::conn::http1::{self};
use hyper::{
    body::{Bytes, Incoming},
    Request, Response,
};
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use std::{convert::Infallible, net::SocketAddr};
use tokio::net::TcpListener;
use tower::{Layer, Service};
use tracing::{info, Level};

pub struct EchoServer {
    listener: TcpListener,
    logging_enabled: bool,
}

impl EchoServer {
    pub async fn new(logging_enabled: bool, port: u16) -> Result<Self, std::io::Error> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        let listener = TcpListener::bind(addr).await?;
        Ok(Self {
            listener,
            logging_enabled,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr().unwrap()
    }

    pub async fn run(self) -> Result<(), std::io::Error> {
        loop {
            let (stream, _) = self.listener.accept().await?;
            let io = TokioIo::new(stream);
            let svc = tower::ServiceBuilder::new()
                .layer(LoggerLayer::new(self.logging_enabled))
                .service_fn(echo);

            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, hyper_util::service::TowerToHyperService::new(svc))
                    .await
                {
                    println!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

async fn echo(_request: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::from(Bytes::from("hello"))))
}

struct LoggerLayer {
    logging_enabled: bool,
}

impl LoggerLayer {
    fn new(logging_enabled: bool) -> Self {
        Self { logging_enabled }
    }
}

impl<S> Layer<S> for LoggerLayer {
    type Service = LoggerService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LoggerService::new(self.logging_enabled, inner)
    }
}

#[derive(Clone)]
struct LoggerService<S> {
    inner: S,
    logger_impl: Arc<dyn LoggerImpl + Send + Sync>,
}

impl<S> LoggerService<S> {
    fn new(logging_enabled: bool, inner: S) -> Self {
        let logger_impl: Arc<dyn LoggerImpl + Send + Sync> = if logging_enabled {
            Arc::new(ActualLogger)
        } else {
            Arc::new(NeverLogger)
        };
        Self { inner, logger_impl }
    }
}

impl<S> Service<Request<Incoming>> for LoggerService<S>
where
    S: Service<Request<Incoming>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Incoming>) -> Self::Future {
        self.logger_impl.log_request(&req);
        self.inner.call(req)
    }
}

trait LoggerImpl {
    fn log_request(&self, request: &Request<Incoming>);
    //fn log_response(&self, response: &Response<Full<Bytes>>);
}

#[derive(Clone)]
struct NeverLogger;

impl LoggerImpl for NeverLogger {
    fn log_request(&self, _: &Request<Incoming>) {}

    //fn log_response(&self, _: &Response<Full<Bytes>>) {}
}

#[derive(Clone)]
struct ActualLogger;

impl LoggerImpl for ActualLogger {
    fn log_request(&self, request: &Request<Incoming>) {
        info!(
            "> {} HTTP {:?} {}",
            request.method(),
            request.version(),
            request.uri().path()
        );
    }
}
