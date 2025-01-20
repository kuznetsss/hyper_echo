use hyper::{body::{Body, Bytes, Frame}, Request};
use tower_http::trace::{OnBodyChunk, OnRequest, OnResponse};
use tracing::{info, Span};

use crate::log_utils::{log_body_frame, log_headers, log_latency, log_request_uri, log_response_uri, LogLevel};

#[derive(Debug, Clone)]
pub struct OnRequestLogger {
    log_level: LogLevel,
}

impl OnRequestLogger {
    pub fn new(log_level: LogLevel) -> Self {
        Self { log_level }
    }
}

impl<B> OnRequest<B> for OnRequestLogger
where
    B: Body,
{
    fn on_request(&mut self, request: &Request<B>, _span: &Span) {
        match &self.log_level {
            LogLevel::None => {}
            LogLevel::Uri => {
                log_request_uri(request);
            }
            LogLevel::UriHeaders | LogLevel::UriHeadersBody => {
                log_request_uri(request);
                log_headers(request.headers(), '>');
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct OnResponseLogger {
    log_level: LogLevel,
}

impl OnResponseLogger {
    pub fn new(log_level: LogLevel) -> Self {
        Self { log_level }
    }
}

impl<B> OnResponse<B> for OnResponseLogger
where
    B: Body,
{
    fn on_response(self, response: &hyper::Response<B>, latency: std::time::Duration, _span: &Span) {
        match self.log_level {
            LogLevel::None => {return;},
            LogLevel::Uri => {log_response_uri(response);}
            LogLevel::UriHeaders | LogLevel::UriHeadersBody => {
                log_response_uri(response);
                log_headers(response.headers(), '<');
            }
        }
        log_latency(latency);
    }
}

#[derive(Debug, Clone)]
pub struct BodyLogger {
    log_level: LogLevel
}

impl BodyLogger {
    pub fn new(log_level: LogLevel) -> Self {
        Self { log_level }
    }
}

impl OnBodyChunk<Bytes> for BodyLogger {
    fn on_body_chunk(&mut self, chunk: &Bytes, latency: std::time::Duration, span: &Span) {
        info!("> {:?}", chunk);
    }
}
