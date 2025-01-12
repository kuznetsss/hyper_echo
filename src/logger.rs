use std::{future::Future, sync::Arc, task::Poll, time::Instant};

use hyper::{body::Incoming, Request, Response};
use pin_project::pin_project;
use tower::{Layer, Service};
use tracing::{error, info};

pub struct LoggerLayer {
    logging_enabled: bool,
}

impl LoggerLayer {
    pub fn new(logging_enabled: bool) -> Self {
        Self { logging_enabled }
    }
}

impl<S> Layer<S> for LoggerLayer {
    type Service = LoggerService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LoggerService::new(self.logging_enabled, inner)
    }
}

type RequestLogger = Arc<dyn Fn(&Request<Incoming>) + Send + Sync>;

#[derive(Clone)]
pub struct LoggerService<S> {
    inner: S,
    logger: RequestLogger,
}

impl<S> LoggerService<S> {
    fn new(logging_enabled: bool, inner: S) -> Self {
        let logger: RequestLogger = if logging_enabled {
            Arc::new(log_request)
        } else {
            Arc::new(|_: &Request<Incoming>| {})
        };
        Self { inner, logger }
    }
}

impl<S, B> Service<Request<Incoming>> for LoggerService<S>
where
    S: Service<Request<Incoming>, Response = Response<B>>,
    S::Future: Future<Output = Result<Response<B>, S::Error>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = LoggingFuture<S::Future>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Incoming>) -> Self::Future {
        (self.logger)(&req);
        let start_time = Instant::now();
        LoggingFuture {
            inner: self.inner.call(req),
            start_time,
        }
    }
}

#[pin_project]
pub struct LoggingFuture<F>
where
    F: Future,
{
    #[pin]
    inner: F,
    start_time: Instant,
}

impl<F, B, E> Future for LoggingFuture<F>
where
    F: Future<Output = Result<Response<B>, E>>,
{
    type Output = F::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        match this.inner.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                let elapsed_time = this.start_time.elapsed();
                match &result {
                    Ok(r) => {
                        info!("< {} in {:.1?}", r.status(), elapsed_time);
                    }
                    Err(_) => {
                        error!("Unexpected error");
                    }
                }
                Poll::Ready(result)
            }
        }
    }
}

fn log_request(request: &Request<Incoming>) {
    info!(
        "> {} HTTP {:?} {}",
        request.method(),
        request.version(),
        request.uri().path()
    );
}
