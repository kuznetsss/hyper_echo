use std::net::IpAddr;

use tracing::{Span, info, span};

#[derive(Debug, Clone)]
pub struct WsLogger {
    span: Option<Span>,
}

impl WsLogger {
    pub fn new(ws_logging_enabled: bool, client_ip: IpAddr, id: u64) -> Self {
        if !ws_logging_enabled {
            Self { span: None }
        } else {
            Self {
                span: Some(span!(
                    tracing::Level::INFO,
                    "ws client",
                    ip = ?client_ip,
                    id = id
                )),
            }
        }
    }

    pub fn log_frame(&self, payload: &str) {
        if self.span.is_none() {
            return;
        }

        let _entered = self.span.as_ref().unwrap().enter();
        info!("WS: {payload}")
    }

    pub fn log_connection_established(&self) {
        if self.span.is_none() {
            return;
        }
        let _entered = self.span.as_ref().unwrap().enter();
        info!("WS: connection established");
    }

    pub fn log_connection_closed(&self) {
        if self.span.is_none() {
            return;
        }
        let _entered = self.span.as_ref().unwrap().enter();
        info!("WS: connection closed");
    }

    pub fn log_duration(&self, elapsed: std::time::Duration) {
        if self.span.is_none() {
            return;
        }
        let _entered = self.span.as_ref().unwrap().enter();
        info!("WS: messaged echoed in {elapsed:.1?}");
    }
}
