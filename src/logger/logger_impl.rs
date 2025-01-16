use std::time::Instant;

use hyper::{body::Body, header::HeaderValue, HeaderMap, Request, Response};
use tracing::info;

#[derive(Clone)]
pub enum Logger {
    Never,
    Uri,
    UriHeaders,
    Full,
}

impl Logger {
    pub fn log_request<B: Body>(&self, request: &Request<B>) {
        match self {
            Self::Never => {}
            Self::Uri => {
                log_request_uri(request);
            }
            Self::UriHeaders => {
                log_request_uri(request);
                log_request_headers(request);
            }
            Self::Full => {
                log_request_uri(request);
                log_request_headers(request);
            }
        };
    }

    pub fn log_response<B: Body>(&self, response: &Response<B>, start_time: &Instant) {
        let elapsed_time = start_time.elapsed();
        match self {
            Self::Never => {
                return;
            }
            Self::Uri => {
                log_response_uri(response);
            }
            Self::UriHeaders => {
                log_response_uri(response);
                log_response_headers(response);
            }
            Self::Full => {
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
