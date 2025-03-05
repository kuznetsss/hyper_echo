use fastwebsockets::OpCode;
use tokio_util::sync::CancellationToken;

mod common;

#[tokio::test]
async fn ws_echo_test() {
    let port = common::spawn_server(CancellationToken::new()).await;
    let mut ws_client = common::WsClient::connect(port).await;
    let message = "Some message";

    ws_client.send(message).await.unwrap();
    let (op_code, response) = ws_client.receive().await.unwrap();

    assert_eq!(op_code, OpCode::Text);
    assert_eq!(response.as_ref().map(String::as_str), Some(message));
}

#[tokio::test]
async fn ws_echo_multiple_messages_test() {
    let port = common::spawn_server(CancellationToken::new()).await;
    let mut ws_client = common::WsClient::connect(port).await;

    for message in ["some message", "other_message", "message with 🙂"] {
        ws_client.send(message).await.unwrap();
        let (op_code, response) = ws_client.receive().await.unwrap();

        assert_eq!(op_code, OpCode::Text);
        assert_eq!(response.as_ref().map(String::as_str), Some(message));
    }
}

#[tokio::test]
async fn ws_client_is_disconnected_after_cancel() {
    let cancellation_token = CancellationToken::new();
    let port = common::spawn_server(cancellation_token.clone()).await;
    let mut ws_client = common::WsClient::connect(port).await;
    cancellation_token.cancel();

    let (op_code, response) = ws_client.receive().await.unwrap();
    assert_eq!(op_code, OpCode::Close);
    assert_eq!(response, None);

    let error = ws_client.receive().await;
    assert!(error.is_err());
}
