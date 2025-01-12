use std::{fmt::Debug, future::Future, task::Poll, time::Instant};

use hyper::{body::Body, Request, Response};
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

#[derive(Clone)]
pub struct LoggerService<S> {
    inner: S,
    logger: Logger,
}

impl<S> LoggerService<S> {
    fn new(logging_enabled: bool, inner: S) -> Self {
        let logger: Logger = if logging_enabled {
            Logger::ActualLogger
        } else {
            Logger::NeverLogger
        };
        Self { inner, logger }
    }
}

impl<S, I, O> Service<Request<I>> for LoggerService<S>
where
    S: Service<Request<I>, Response = Response<O>>,
    S::Future: Future<Output = Result<Response<O>, S::Error>>,
    S::Error: Debug,
    I: Body,
    O: Body,
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

    fn call(&mut self, req: Request<I>) -> Self::Future {
        self.logger.log_request(&req);
        let start_time = Instant::now();
        LoggingFuture {
            inner: self.inner.call(req),
            start_time,
            logger: self.logger.clone(),
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
    logger: Logger,
    start_time: Instant,
}

impl<F, O, E> Future for LoggingFuture<F>
where
    F: Future<Output = Result<Response<O>, E>>,
    E: Debug,
    O: Body,
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
                match &result {
                    Ok(r) => {
                        this.logger.log_response(r, this.start_time);
                    }
                    Err(e) => {
                        error!("Error processing request: {e:?}");
                    }
                }
                Poll::Ready(result)
            }
        }
    }
}

#[derive(Clone)]
enum Logger {
    NeverLogger,
    ActualLogger,
}

impl Logger {
    fn log_request<B: Body>(&self, request: &Request<B>) {
        match self {
            Self::NeverLogger => {}
            Self::ActualLogger => {
                info!(
                    "> {} HTTP {:?} {}",
                    request.method(),
                    request.version(),
                    request.uri().path()
                );
            }
        };
    }

    fn log_response<B: Body>(&self, response: &Response<B>, start_time: &Instant) {
        match self {
            Self::NeverLogger => {}
            Self::ActualLogger => {
                let elapsed_time = start_time.elapsed();
                info!("< {} in {:.1?}", response.status(), elapsed_time);
            }
        }
    }
}
