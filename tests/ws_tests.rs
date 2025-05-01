use std::time::Duration;

use fastwebsockets::OpCode;
use hyper_echo::HttpLogLevel;
use tokio_util::sync::CancellationToken;
use tracing_test::traced_test;

mod common;

#[tokio::test]
async fn ws_echo_test() {
    let port = common::spawn_server(CancellationToken::new()).await;
    let mut ws_client = common::WsClient::connect(port).await;
    let message = "Some message";

    ws_client.send_message(message).await.unwrap();
    let (opcode, response) = ws_client.receive().await.unwrap();

    assert_eq!(opcode, OpCode::Text);
    assert_eq!(response.as_ref().map(String::as_str), Some(message));
}

#[tokio::test]
async fn ws_echo_multiple_messages_test() {
    let port = common::spawn_server(CancellationToken::new()).await;
    let mut ws_client = common::WsClient::connect(port).await;

    for message in ["some message", "other_message", "message with 🙂"] {
        ws_client.send_message(message).await.unwrap();
        let (opcode, response) = ws_client.receive().await.unwrap();

        assert_eq!(opcode, OpCode::Text);
        assert_eq!(response.as_ref().map(String::as_str), Some(message));
    }
}

#[tokio::test]
async fn ws_client_is_disconnected_after_cancel() {
    let cancellation_token = CancellationToken::new();
    let port = common::spawn_server(cancellation_token.clone()).await;
    let mut ws_client = common::WsClient::connect(port).await;
    cancellation_token.cancel();

    let (opcode, response) = ws_client.receive().await.unwrap();
    assert_eq!(opcode, OpCode::Close);
    assert_eq!(response, None);

    let error = ws_client.receive().await;
    assert!(error.is_err());
}

#[tokio::test]
async fn ws_echo_with_pings() {
    let port =
        common::spawn_server_with_ws_pings(CancellationToken::new(), Duration::from_millis(100))
            .await;
    let mut ws_client = common::WsClient::connect(port).await;

    let message = "Some message";

    let (opcode, data) = ws_client.receive().await.unwrap();
    assert_eq!(opcode, OpCode::Ping);
    assert_eq!(data, None);
    ws_client.send_pong().await.unwrap();

    ws_client.send_message(message).await.unwrap();
    let (opcode, response) = ws_client.receive().await.unwrap();

    assert_eq!(opcode, OpCode::Text);
    assert_eq!(response.as_ref().map(String::as_str), Some(message));
}

#[tokio::test]
async fn ws_client_is_disconnected_when_doesnt_send_pongs() {
    let port =
        common::spawn_server_with_ws_pings(CancellationToken::new(), Duration::from_millis(10))
            .await;
    let mut ws_client = common::WsClient::connect(port).await;

    let (opcode, data) = ws_client.receive().await.unwrap();
    assert_eq!(opcode, OpCode::Ping);
    assert_eq!(data, None);

    let _ = tokio::time::sleep(Duration::from_millis(11));

    let (opcode, data) = ws_client.receive().await.unwrap();
    assert_eq!(opcode, OpCode::Close);
    assert_eq!(data, None);
}

async fn send_request(ws_logging_enabled: bool) {
    let port = common::spawn_server_with_log_level(
        CancellationToken::new(),
        HttpLogLevel::None,
        ws_logging_enabled,
    )
    .await;
    let mut ws_client = common::WsClient::connect(port).await;
    let message = "Some message";

    ws_client.send_message(message).await.unwrap();
    let _ = ws_client.receive().await.unwrap();
}

#[tokio::test]
#[traced_test]
async fn ws_echo_logging_disabled() {
    send_request(false).await;

    logs_assert(|all_logs: &[&str]| {
        let logs_count = all_logs
            .iter()
            .filter(|s| !s.contains("TRACE") && !s.contains("DEBUG"))
            .count();
        assert_eq!(logs_count, 0);
        Ok(())
    });
}

#[tokio::test]
#[traced_test]
async fn ws_echo_logging_enabled() {
    send_request(true).await;
    let expected_logs = [
        "client{ip=127.0.0.1 id=0}: hyper_echo::ws_logger: WS: connection established",
        "client{ip=127.0.0.1 id=0}: hyper_echo::ws_logger: WS: Some message",
        "client{ip=127.0.0.1 id=0}: hyper_echo::ws_logger: WS: message echoed in",
    ];

    logs_assert(|all_logs: &[&str]| {
        let logs: Vec<&str> = all_logs
            .iter()
            .filter(|s| !s.contains("TRACE") && !s.contains("DEBUG"))
            .map(|&s| s)
            .collect();

        assert_eq!(logs.len(), expected_logs.len());

        expected_logs.iter().for_each(|expected| {
            let found_num = logs.iter().filter(|s| s.contains(expected)).count();
            assert_eq!(found_num, 1);
        });
        Ok(())
    });
}
