use std::{net::IpAddr, time::Instant};

use hyper::{
    body::{Body, Bytes},
    Request, Response,
};
use tracing::{span, Level};

use crate::log_utils::{
    log_body_frame, log_headers, log_latency, log_request_uri, log_response_uri, Direction,
    LogLevel,
};

use super::body::LoggingBody;

#[derive(Clone)]
pub struct Logger {
    log_level: LogLevel,
    span: tracing::Span,
}

impl Logger {
    pub fn new(log_level: LogLevel, client_ip: IpAddr, id: u64) -> Self {
        Self {
            log_level,
            span: span!(Level::INFO, "client", ip = ?client_ip, id = id),
        }
    }

    pub fn wrap_request<B>(&self, request: Request<B>) -> Request<LoggingBody<B>>
    where
        B: Body<Data = Bytes>,
    {
        let span = self.span.clone();
        match self.log_level {
            LogLevel::UriHeadersBody => request.map(|b| LoggingBody::new(b, span, log_body_frame)),
            _ => request.map(|b| LoggingBody::new(b, span, |_, _| {})),
        }
    }

    pub fn log_request<B: Body>(&self, request: &Request<B>) {
        let _enter = self.span.enter();
        match self.log_level {
            LogLevel::None => {}
            LogLevel::Uri => {
                log_request_uri(request);
            }
            LogLevel::UriHeaders | LogLevel::UriHeadersBody => {
                log_request_uri(request);
                log_headers(request.headers(), Direction::Incoming);
                // Body is logged in LoggingBody if needed
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
            LogLevel::UriHeaders | LogLevel::UriHeadersBody => {
                log_response_uri(response);
                log_headers(response.headers(), Direction::Outgoing);
            }
        }
        log_latency(elapsed_time);
    }
}
