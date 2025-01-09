use std::sync::Arc;

use hyper::{body::Incoming, Request};
use tower::{Layer, Service};
use tracing::info;

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
