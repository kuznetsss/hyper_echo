use std::{fmt::Debug, future::Future, task::Poll, time::Instant};

use hyper::{Response, body::Body};
use pin_project::pin_project;
use tracing::error;

use super::logger_impl::Logger;

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

impl<F> LoggingFuture<F>
where
    F: Future,
{
    pub fn new(inner: F, logger: Logger, start_time: Instant) -> Self {
        LoggingFuture {
            inner,
            logger,
            start_time,
        }
    }
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
