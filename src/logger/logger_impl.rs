use std::time::Instant;

use hyper::{body::Body, Request, Response};
use tracing::info;

#[derive(Clone)]
pub enum Logger {
    NeverLogger,
    ActualLogger,
}

impl Logger {
    pub fn log_request<B: Body>(&self, request: &Request<B>) {
        match self {
            Self::NeverLogger => {}
            Self::ActualLogger => {
                info!(
                    "> {} {} {:?}",
                    request.method(),
                    request.uri().path(),
                    request.version(),
                );
            }
        };
    }

    pub fn log_response<B: Body>(&self, response: &Response<B>, start_time: &Instant) {
        match self {
            Self::NeverLogger => {}
            Self::ActualLogger => {
                let elapsed_time = start_time.elapsed();
                info!(
                    "< {:?} {} in {:.1?}",
                    response.version(),
                    response.status(),
                    elapsed_time
                );
            }
        }
    }
}
