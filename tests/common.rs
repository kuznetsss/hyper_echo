#[allow(dead_code)]
use std::time::Duration;

use fastwebsockets::{FragmentCollector, Frame, OpCode, Payload, WebSocketError};
use http_body_util::Empty;
use hyper::{
    Request,
    body::Bytes,
    header::{CONNECTION, UPGRADE},
    upgrade::Upgraded,
};
use hyper_echo::{EchoServer, HttpLogLevel};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

pub async fn spawn_server(cancellation_token: CancellationToken) -> u16 {
    spawn_server_impl(cancellation_token, None, HttpLogLevel::None, false).await
}

pub async fn spawn_server_with_ws_pings(
    cancellation_token: CancellationToken,
    ws_ping_interval: Duration,
) -> u16 {
    spawn_server_impl(
        cancellation_token,
        Some(ws_ping_interval),
        HttpLogLevel::None,
        false,
    )
    .await
}

pub async fn spawn_server_with_log_level(
    cancellation_token: CancellationToken,
    http_log_level: HttpLogLevel,
    ws_logging_enabled: bool,
) -> u16 {
    spawn_server_impl(cancellation_token, None, http_log_level, ws_logging_enabled).await
}

async fn spawn_server_impl(
    cancellation_token: CancellationToken,
    ws_ping_interval: Option<Duration>,
    http_log_level: HttpLogLevel,
    ws_logging_enabled: bool,
) -> u16 {
    let mut echo_server = EchoServer::new(None, http_log_level, ws_logging_enabled)
        .await
        .unwrap();
    echo_server.set_ws_ping_interval(ws_ping_interval);
    let port = echo_server.local_addr().port();
    tokio::spawn({
        async move {
            echo_server.run(cancellation_token).await.unwrap();
        }
    });
    port
}

pub struct WsClient {
    ws: FragmentCollector<TokioIo<Upgraded>>,
}

impl WsClient {
    pub async fn connect(port: u16) -> Self {
        let host = format!("localhost:{port}");
        let stream = TcpStream::connect(&host).await.unwrap();

        let req = Request::builder()
            .method("GET")
            .uri(format!("http://{}/", &host))
            .header("Host", host)
            .header(UPGRADE, "websocket")
            .header(CONNECTION, "upgrade")
            .header(
                "Sec-WebSocket-Key",
                fastwebsockets::handshake::generate_key(),
            )
            .header("Sec-WebSocket-Version", "13")
            .body(Empty::<Bytes>::new())
            .unwrap();

        let (mut ws, _) = fastwebsockets::handshake::client(&TokioExecutor::new(), req, stream)
            .await
            .unwrap();
        ws.set_auto_pong(false);
        Self {
            ws: FragmentCollector::new(ws),
        }
    }

    pub async fn send_message(&mut self, data: &str) -> Result<(), WebSocketError> {
        let frame = Frame::text(Payload::Borrowed(data.as_bytes()));
        self.ws.write_frame(frame).await
    }

    pub async fn send_pong(&mut self) -> Result<(), WebSocketError> {
        let frame = Frame::pong(Payload::Borrowed(&[]));
        self.ws.write_frame(frame).await
    }

    pub async fn receive(&mut self) -> Result<(OpCode, Option<String>), WebSocketError> {
        let frame = self.ws.read_frame().await?;
        let payload = match &frame.opcode {
            OpCode::Text | OpCode::Binary => {
                Some(String::from_utf8_lossy(&frame.payload).to_string())
            }
            _ => None,
        };

        Ok((frame.opcode, payload))
    }
}
