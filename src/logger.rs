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

type LoggerFn = Arc<dyn Fn(&Request<Incoming>) + Send + Sync>;

#[derive(Clone)]
pub struct LoggerService<S> {
    inner: S,
    logger: LoggerFn,
}

impl<S> LoggerService<S> {
    fn new(logging_enabled: bool, inner: S) -> Self {
        let logger: LoggerFn = if logging_enabled {
            Arc::new(log_request)
        } else {
            Arc::new(|_: &Request<Incoming>| {})
        };
        Self { inner, logger }
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
        (self.logger)(&req);
        self.inner.call(req)
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
