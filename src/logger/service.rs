use std::{fmt::Debug, future::Future, net::IpAddr, time::Instant};

use hyper::{
    body::{Body, Bytes},
    Request, Response,
};
use tower::{Layer, Service};

use crate::log_utils::LogLevel;
use super::{body::LoggingBody, future::LoggingFuture, logger_impl::Logger};

pub struct LoggerLayer {
    log_level: LogLevel,
    client_addr: IpAddr,
    id: u64,
}

impl LoggerLayer {
    pub fn new(log_level: LogLevel, client_addr: IpAddr, id: u64) -> Self {
        Self {
            log_level,
            client_addr,
            id,
        }
    }
}

impl<S> Layer<S> for LoggerLayer {
    type Service = LoggerService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        let logger = Logger::new(self.log_level, self.client_addr, self.id);
        LoggerService::new(logger, inner)
    }
}

#[derive(Clone)]
pub struct LoggerService<S> {
    inner: S,
    logger: Logger,
}

impl<S> LoggerService<S> {
    fn new(logger: Logger, inner: S) -> Self {
        Self { inner, logger }
    }
}

impl<S, I, O> Service<Request<I>> for LoggerService<S>
where
    S: Service<Request<LoggingBody<I>>, Response = Response<O>>,
    S::Future: Future<Output = Result<Response<O>, S::Error>>,
    S::Error: Debug,
    I: Body<Data = Bytes>,
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
        let start_time = Instant::now();
        let req = self.logger.wrap_request(req);
        self.logger.log_request(&req);
        LoggingFuture::new(self.inner.call(req), self.logger.clone(), start_time)
    }
}
