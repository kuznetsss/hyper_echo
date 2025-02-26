use std::time::Duration;

use hyper::{
    HeaderMap, Request,
    body::{Body, Bytes},
    header::HeaderValue,
};
use tracing::{Span, info};

/// Level of logging requests and responses
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpLogLevel {
    /// No logging
    None,
    /// Log URI only
    Uri,
    /// Log URI and headers
    UriHeaders,
    /// Log URI, headers and body.
    /// <div class="warning"> Body is passed to requests as a stream so it might be logged after processing is finished.</div>
    UriHeadersBody,
}

impl From<u8> for HttpLogLevel {
    fn from(value: u8) -> Self {
        match value {
            0 => HttpLogLevel::None,
            1 => HttpLogLevel::Uri,
            2 => HttpLogLevel::UriHeaders,
            3 => HttpLogLevel::UriHeadersBody,
            _ => panic!("Invalid log level {value}"),
        }
    }
}

const HTTP_PREFIX: &str = "HTTP:";

pub fn log_request_uri<B: Body>(request: &Request<B>) {
    info!(
        "{HTTP_PREFIX} {} {} {:?}",
        request.method(),
        request.uri().path(),
        request.version(),
    );
}

pub fn log_headers(headers: &HeaderMap<HeaderValue>) {
    headers.iter().for_each(|(name, value)| {
        info!(
            "{HTTP_PREFIX} {name}: {}",
            value.to_str().unwrap_or("<binary or malformed>")
        );
    });
}

pub fn log_body_frame(frame: &Bytes, span: &Span) {
    let _enter = span.enter();
    info!("{HTTP_PREFIX} {:?}", frame);
}

pub fn log_latency(latency: Duration) {
    info!("{HTTP_PREFIX} Processed in {:.1?}", latency);
}
