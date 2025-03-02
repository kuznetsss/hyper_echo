use hyper::header::{HeaderName, HeaderValue, ACCEPT};
use hyper_echo::{EchoServer, HttpLogLevel};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn http_echo_test() {
    let echo_server = EchoServer::new(None, HttpLogLevel::None, false)
        .await
        .unwrap();
    let port = echo_server.local_addr().port();
    tokio::spawn({
        async move {
            echo_server.run(CancellationToken::new()).await.unwrap();
        }
    });

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
    assert_eq!(response.headers().get(header_name), Some(header_value).as_ref());
    assert_eq!(response.text().await.unwrap(), "some body");
}
