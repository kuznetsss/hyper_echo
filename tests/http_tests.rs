use hyper::header::{ACCEPT, HeaderValue};
use hyper_echo::HttpLogLevel;
use tokio_util::sync::CancellationToken;
use tracing_test::traced_test;

mod common;

#[tokio::test]
async fn http_echo() {
    let port = common::spawn_server(CancellationToken::new()).await;

    let header_name = ACCEPT;
    let header_value = HeaderValue::from_str("some value").unwrap();
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://localhost:{port}/"))
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
async fn http_request_fails_after_cancel() {
    let cancellation_token = CancellationToken::new();
    let port = common::spawn_server(cancellation_token.clone()).await;
    cancellation_token.cancel();

    let url = format!("http://localhost:{port}/");
    let response = reqwest::get(url).await;
    assert!(response.is_err());
}

async fn make_request(http_log_level: HttpLogLevel) {
    let port =
        common::spawn_server_with_log_level(CancellationToken::new(), http_log_level, false).await;

    let url = format!("http://127.0.0.1:{port}");

    let _ = reqwest::Client::new()
        .get(url)
        .header(ACCEPT, HeaderValue::from_str("some value").unwrap())
        .body("some body")
        .send()
        .await
        .unwrap();
}

#[tokio::test]
#[traced_test]
async fn http_request_logging_disabled() {
    make_request(HttpLogLevel::None).await;

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
async fn http_request_log_uri() {
    make_request(HttpLogLevel::Uri).await;

    let expected_logs = [
        "client{ip=127.0.0.1 id=0}: hyper_echo::log_utils: HTTP: GET / HTTP/1.1",
        "client{ip=127.0.0.1 id=0}: hyper_echo::log_utils: HTTP: Processed in",
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

#[tokio::test]
#[traced_test]
async fn http_request_log_uri_headers() {
    make_request(HttpLogLevel::UriHeaders).await;

    let expected_logs = [
        "client{ip=127.0.0.1 id=0}: hyper_echo::log_utils: HTTP: GET / HTTP/1.1",
        "client{ip=127.0.0.1 id=0}: hyper_echo::log_utils: HTTP: accept: some value",
        "client{ip=127.0.0.1 id=0}: hyper_echo::log_utils: HTTP: host: 127.0.0.1:",
        "client{ip=127.0.0.1 id=0}: hyper_echo::log_utils: HTTP: content-length: 9",
        "client{ip=127.0.0.1 id=0}: hyper_echo::log_utils: HTTP: Processed in",
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

#[tokio::test]
#[traced_test]
async fn http_request_log_uri_headers_body() {
    make_request(HttpLogLevel::UriHeadersBody).await;

    let expected_logs = [
        "client{ip=127.0.0.1 id=0}: hyper_echo::log_utils: HTTP: GET / HTTP/1.1",
        "client{ip=127.0.0.1 id=0}: hyper_echo::log_utils: HTTP: accept: some value",
        "client{ip=127.0.0.1 id=0}: hyper_echo::log_utils: HTTP: host: 127.0.0.1:",
        "client{ip=127.0.0.1 id=0}: hyper_echo::log_utils: HTTP: content-length: 9",
        "client{ip=127.0.0.1 id=0}: hyper_echo::log_utils: HTTP: Processed in",
        "client{ip=127.0.0.1 id=0}: hyper_echo::log_utils: HTTP: b\"some body\""
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
