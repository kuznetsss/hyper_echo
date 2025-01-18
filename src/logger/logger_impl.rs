use std::{net::IpAddr, time::Instant};

use hyper::{body::Body, header::HeaderValue, HeaderMap, Request, Response};
use tracing::{info, span, Level};

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

#[derive(Clone)]
pub struct Logger {
    log_level: LogLevel,
    span: tracing::Span,
}

impl Logger {
    pub fn new(log_level: LogLevel, client_addr: IpAddr, id: u64) -> Self {
        let client_addr = format!("{}", &client_addr);
        Self {
            log_level,
            span: span!(Level::INFO, "client", ip = client_addr, id = id),
        }
    }

    pub fn log_request<B: Body>(&self, request: &Request<B>) {
        let _enter = self.span.enter();
        match self.log_level {
            LogLevel::None => {}
            LogLevel::Uri => {
                log_request_uri(request);
            }
            LogLevel::UriHeaders => {
                log_request_uri(request);
                log_request_headers(request);
            }
            LogLevel::UriHeadersBody => {
                log_request_uri(request);
                log_request_headers(request);
            }
        };
    }

    pub fn log_response<B: Body>(&self, response: &Response<B>, start_time: &Instant) {
        let _enter = self.span.enter();
        let elapsed_time = start_time.elapsed();
        match self.log_level {
            LogLevel::None => {
                return;
            }
            LogLevel::Uri => {
                log_response_uri(response);
            }
            LogLevel::UriHeaders => {
                log_response_uri(response);
                log_response_headers(response);
            }
            LogLevel::UriHeadersBody => {
                log_response_uri(response);
                log_response_headers(response);
            }
        }
        info!("Processed in {:.1?}", elapsed_time);
    }
}

fn log_request_uri<B: Body>(request: &Request<B>) {
    info!(
        "> {} {} {:?}",
        request.method(),
        request.uri().path(),
        request.version(),
    );
}

fn log_response_uri<B: Body>(response: &Response<B>) {
    info!("< {:?} {}", response.version(), response.status());
}

fn log_request_headers<B: Body>(request: &Request<B>) {
    log_headers(request.headers(), '>');
}

fn log_response_headers<B: Body>(response: &Response<B>) {
    log_headers(response.headers(), '<');
}

fn log_headers(headers: &HeaderMap<HeaderValue>, direction: char) {
    headers.iter().for_each(|(name, value)| {
        info!(
            "{direction} {name}: {}",
            value.to_str().unwrap_or("<binary or malformed>")
        );
    });
}
