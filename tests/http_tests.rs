use hyper::header::{ACCEPT, HeaderValue};
use tokio_util::sync::CancellationToken;

mod common;

#[tokio::test]
async fn http_echo() {
    let port = common::spawn_server(CancellationToken::new()).await;

    let url = format!("http://localhost:{port}/");
    let header_name = ACCEPT;
    let header_value = HeaderValue::from_str("some value").unwrap();
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header(header_name.clone(), header_value.clone())
        .body("some body")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get(header_name),
        Some(header_value).as_ref()
    );
    assert_eq!(response.text().await.unwrap(), "some body");
}

#[tokio::test]
async fn request_fails_after_cancel() {
    let cancellation_token = CancellationToken::new();
    let port = common::spawn_server(cancellation_token.clone()).await;
    cancellation_token.cancel();

    let url = format!("http://localhost:{port}/");
    let response = reqwest::get(url).await;
    assert!(response.is_err());
}
