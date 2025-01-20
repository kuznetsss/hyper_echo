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

pub fn log_request_uri<B: Body>(request: &Request<B>) {
    info!(
        "> {} {} {:?}",
        request.method(),
        request.uri().path(),
        request.version(),
    );
}

pub fn log_response_uri<B: Body>(response: &Response<B>) {
    info!("< {:?} {}", response.version(), response.status());
}

pub fn log_headers(headers: &HeaderMap<HeaderValue>, direction: char) {
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
        info!("> {:?}", data);
    }
}

pub fn log_latency(latency: Duration) {
    info!("Processed in {:.1?}", latency);
}
