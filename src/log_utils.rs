use std::time::Duration;

use hyper::{
    body::{Body, Bytes, Frame},
    header::HeaderValue,
    HeaderMap, Request, Response,
};
use tracing::{info, Span};

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    None,
    Uri,
    UriHeaders,
    UriHeadersBody,
}

impl From<u8> for LogLevel {
    fn from(value: u8) -> Self {
        match value {
            0 => LogLevel::None,
            1 => LogLevel::Uri,
            2 => LogLevel::UriHeaders,
            3 => LogLevel::UriHeadersBody,
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
                write!(f, "IN")
            }
            Direction::Outgoing => {
                write!(f, "OUT")
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

pub fn log_body_frame(frame: &Frame<Bytes>, span: &Span) {
    let _enter = span.enter();
    if let Some(data) = frame.data_ref() {
        info!("{} {:?}", Direction::Incoming, data);
    }
}

pub fn log_latency(latency: Duration) {
    info!("Processed in {:.1?}", latency);
}
