use std::{fmt::Debug, future::Future, time::Instant};

use hyper::{body::Body, Request, Response};
use tower::{Layer, Service};

use super::{future::LoggingFuture, logger_impl::Logger};

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
            Logger::Uri
        } else {
            Logger::Never
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
        LoggingFuture::new(self.inner.call(req), self.logger.clone(), start_time)
    }
}
