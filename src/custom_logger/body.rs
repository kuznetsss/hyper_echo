use std::task::Poll;

use hyper::body::{Body, Frame};
use pin_project::pin_project;
use tracing::Span;

#[pin_project]
pub struct LoggingBody<B: Body> {
    #[pin]
    inner: B,
    span: Span,
    logger: fn(&B::Data, &Span),
}

impl<B: Body> LoggingBody<B> {
    pub fn new(inner: B, span: Span, logger: fn(&B::Data, &Span)) -> Self {
        LoggingBody {
            inner,
            span,
            logger,
        }
    }
}

impl<B> Body for LoggingBody<B>
where
    B: Body,
{
    type Data = B::Data;

    type Error = B::Error;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.project();
        match this.inner.poll_frame(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                if let Some(Ok(frame)) = &result {
                    if let Some(data) = frame.data_ref() {
                        (this.logger)(data, this.span);
                    }
                }
                Poll::Ready(result)
            }
        }
    }
}
