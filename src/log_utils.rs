use std::time::Duration;

use hyper::{
    HeaderMap, Request, Response,
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

#[derive(Debug)]
pub enum Direction {
    Incoming,
    Outgoing,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Incoming => {
                write!(f, "IN:")
            }
            Direction::Outgoing => {
                write!(f, "OUT:")
            }
        }
    }
}

pub fn log_request_uri<B: Body>(request: &Request<B>) {
    info!(
        "{} {} {} {:?}",
        Direction::Incoming,
        request.method(),
        request.uri().path(),
        request.version(),
    );
}

pub fn log_response_uri<B: Body>(response: &Response<B>) {
    info!(
        "{} {:?} {}",
        Direction::Outgoing,
        response.version(),
        response.status()
    );
}

pub fn log_headers(headers: &HeaderMap<HeaderValue>, direction: Direction) {
    headers.iter().for_each(|(name, value)| {
        info!(
            "{direction} {name}: {}",
            value.to_str().unwrap_or("<binary or malformed>")
        );
    });
}

pub fn log_body_frame(frame: &Bytes, span: &Span) {
    let _enter = span.enter();
    info!("{} {:?}", Direction::Incoming, frame);
}

pub fn log_latency(latency: Duration) {
    info!("Processed in {:.1?}", latency);
}
