use std::time::Duration;
use std::{convert::Infallible, time::Instant};

use fastwebsockets::{
    CloseCode, Frame, OpCode, Payload, WebSocket, WebSocketError, upgrade::upgrade,
};
use http_body_util::Full;
use http_body_util::combinators::BoxBody;
use hyper::StatusCode;
use hyper::{Request, Response, body::Bytes, upgrade::Upgraded};
use hyper_util::rt::TokioIo;
use tokio::{select, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::ws_logger::WsLogger;

use super::BoxedError;
use super::http::to_boxed_body;

#[derive(Debug, Clone)]
pub struct SessionData {
    ws_logger: WsLogger,
    ws_ping_interval: Option<Duration>,
    cancellation_token: CancellationToken,
}

impl SessionData {
    pub fn new(
        ws_logger: WsLogger,
        ws_ping_interval: Option<Duration>,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            ws_logger,
            ws_ping_interval,
            cancellation_token,
        }
    }
}

pub(in crate::service) fn run_session<B>(
    mut request: Request<B>,
    session_data: SessionData,
) -> Result<Response<BoxBody<Bytes, BoxedError!()>>, Infallible>
where
    B: Send + Sync + 'static,
{
    match upgrade(&mut request) {
        Ok((response, fut)) => {
            tokio::spawn(async move {
                match fut.await {
                    Ok(mut ws) => {
                        ws.set_auto_close(true);
                        ws.set_auto_pong(true);
                        ws.set_max_message_size(16 * 1024 * 1024); // 16 MB
                        echo_ws(ws, session_data).await;
                    }
                    Err(e) => {
                        warn!("Failed to establish websocket connection: {e}");
                    }
                }
            });
            Ok(to_boxed_body(response))
        }
        Err(e) => Ok(to_response(e)),
    }
}

async fn echo_ws(mut ws: WebSocket<TokioIo<Upgraded>>, session_data: SessionData) {
    let mut ping_interval =
        tokio::time::interval(session_data.ws_ping_interval.unwrap_or(Duration::MAX));
    let mut got_pong: Option<bool> = None;

    session_data.ws_logger.log_connection_established();
    loop {
        let frame = select! {
            biased;
            _ = ping_interval.tick() => {
                if session_data.ws_ping_interval.is_none() {
                    continue;
                }
                if let Some(false) = got_pong {
                    session_data.ws_logger.log("Didn't receive pong from client");
                    break;
                }
                let ping_frame = Frame::new(true, OpCode::Ping, None, Payload::Borrowed(&[]));
                if ws.write_frame(ping_frame).await.is_err() {
                    break;
                }
                got_pong = Some(false);
                continue;
            },
            frame = session_data.cancellation_token.run_until_cancelled(ws.read_frame()) => {
                    let Some(Ok(frame)) = frame else {break;};
                    frame
            },
        };

        let start = Instant::now();
        match frame.opcode {
            OpCode::Text | OpCode::Binary => {
                let payload = String::from_utf8_lossy(&frame.payload);
                session_data.ws_logger.log(&payload);
                let frame = Frame::new(
                    true,
                    frame.opcode,
                    None,
                    Payload::Borrowed(payload.as_bytes()),
                );
                if let Err(e) = ws.write_frame(frame).await {
                    session_data
                        .ws_logger
                        .log(&format!("Error sending ws frame: {e}"));
                    break;
                }
                session_data.ws_logger.log_duration(start.elapsed())
            }
            OpCode::Close => {
                break;
            }
            OpCode::Pong => {
                got_pong = Some(true);
            }
            _ => {}
        }
    }

    // Try to close connection gracefully if it is still alive
    if !ws.is_closed() {
        let close_frame = Frame::close(CloseCode::Normal.into(), &[]);
        select! {
             _ = ws.write_frame(close_frame) => {},
             _ = sleep(std::time::Duration::from_secs(1)) => {},
        };
    }
    session_data.ws_logger.log_connection_closed();
}

fn to_response(e: WebSocketError) -> Response<BoxBody<Bytes, BoxedError!()>> {
    let body = Full::new(Bytes::from(e.to_string()));
    let response = Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(body)
        .unwrap();
    to_boxed_body(response)
}
