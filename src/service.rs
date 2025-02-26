use crate::{log_utils::HttpLogLevel, ws_logger::WsLogger};
use fastwebsockets::{
    CloseCode, Frame, OpCode, Payload, WebSocket, WebSocketError,
    upgrade::{is_upgrade_request, upgrade},
};
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::{
    Request, Response, StatusCode,
    body::{Body, Bytes},
    upgrade::Upgraded,
};
use hyper_util::rt::TokioIo;
use std::{
    convert::Infallible, error::Error, future::Future, net::IpAddr, pin::Pin, time::Instant,
};
use tokio::{select, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::warn;

macro_rules! BoxedError {
    () => {
        Box<dyn Error + Send + Sync + 'static>
    };
}

#[cfg(feature = "custom_trace")]
pub fn make_service(
    log_level: HttpLogLevel,
    ws_logging_enabled: bool,
    client_ip: IpAddr,
    id: u64,
    cancellation_token: CancellationToken,
) -> impl tower::Service<
    Request<hyper::body::Incoming>,
    Response = Response<BoxBody<Bytes, BoxedError!()>>,
    Error = Infallible,
    Future = impl Future,
> + Clone {
    use crate::custom_logger::LoggerLayer;

    let svc = EchoService::new(ws_logging_enabled, client_ip, id, cancellation_token);
    tower::ServiceBuilder::new()
        .layer(LoggerLayer::new(log_level, client_ip, id))
        .service(svc)
}

#[cfg(feature = "tower_trace")]
pub fn make_service(
    http_log_level: HttpLogLevel,
    ws_logging_enabled: bool,
    client_ip: IpAddr,
    id: u64,
    cancellation_token: CancellationToken,
) -> impl tower::Service<
    Request<hyper::body::Incoming>,
    Response = Response<
        tower_http::trace::ResponseBody<
            BoxBody<Bytes, BoxedError!()>,
            tower_http::classify::NeverClassifyEos<tower_http::classify::ServerErrorsFailureClass>,
            crate::tower_loggers::BodyLogger,
        >,
    >,
    Future = impl Future,
    Error = Infallible,
> + Clone {
    use crate::tower_loggers::{BodyLogger, OnRequestLogger, OnResponseLogger, SpanMaker};
    use tower_http::trace::TraceLayer;

    let echo_service = EchoService::new(ws_logging_enabled, client_ip, id, cancellation_token);

    let svc = tower::ServiceBuilder::new()
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(SpanMaker::new(client_ip, id))
                .on_request(OnRequestLogger::new(http_log_level))
                .on_response(OnResponseLogger::new(http_log_level))
                .on_body_chunk(BodyLogger::new(http_log_level)),
        )
        .service(echo_service);
    svc
}

#[derive(Debug, Clone)]
struct EchoService {
    ws_logger: WsLogger,
    cancellation_token: CancellationToken,
}

impl<B> tower::Service<Request<B>> for EchoService
where
    B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static,
{
    type Response = Response<BoxBody<Bytes, BoxedError!()>>;

    type Error = Infallible;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Infallible>> + Send>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let response =
            process_request(req, self.ws_logger.clone(), self.cancellation_token.clone());
        Box::pin(response)
    }
}

impl EchoService {
    pub fn new(
        ws_logging_enabled: bool,
        client_ip: IpAddr,
        id: u64,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            ws_logger: WsLogger::new(ws_logging_enabled, client_ip, id),
            cancellation_token,
        }
    }
}

async fn process_request<B>(
    request: Request<B>,
    ws_logger: WsLogger,
    cancellation_token: CancellationToken,
) -> Result<Response<BoxBody<Bytes, BoxedError!()>>, Infallible>
where
    B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static,
{
    if is_upgrade_request(&request) {
        websocket_upgrade(request, ws_logger, cancellation_token).await
    } else {
        echo(request).await
    }
}

async fn websocket_upgrade<B>(
    mut request: Request<B>,
    ws_logger: WsLogger,
    cancellation_token: CancellationToken,
) -> Result<Response<BoxBody<Bytes, BoxedError!()>>, Infallible>
where
    B: Send + Sync + 'static,
{
    match upgrade(&mut request) {
        Ok((response, fut)) => {
            tokio::spawn(async move {
                match fut.await {
                    Ok(ws) => {
                        echo_ws(ws, ws_logger, cancellation_token).await;
                    }
                    Err(e) => {
                        warn!("Failed to establish websocket connection: {e}");
                    }
                }
            });
            let response = response.map(|b| {
                let b = b.map_err(Into::into);
                BoxBody::new(b)
            });
            Ok(response)
        }
        Err(e) => Ok(to_response(e)),
    }
}

async fn echo<B>(request: Request<B>) -> Result<Response<BoxBody<Bytes, BoxedError!()>>, Infallible>
where
    B: Body<Data = Bytes> + Send + Sync + 'static,
    B::Error: Error + Send + Sync + 'static,
{
    let (parts, body) = request.into_parts();
    let body = BoxBody::new(body.map_err(Into::into));

    let mut response = Response::builder()
        .status(200)
        .version(parts.version)
        .extension(parts.extensions)
        .body(body)
        .unwrap();
    *response.headers_mut() = parts.headers;
    Ok(response)
}

async fn echo_ws(
    mut ws: WebSocket<TokioIo<Upgraded>>,
    ws_logger: WsLogger,
    cancellation_token: CancellationToken,
) {
    ws.set_auto_close(true);
    ws.set_auto_pong(true);
    ws.set_max_message_size(16 * 1024 * 1024); // 16 MB

    let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(5));
    let mut got_pong : Option<bool> = None;

    ws_logger.log_connection_established();
    loop {
        let frame = select! {
            biased;
            _ = ping_interval.tick() => {
                if let Some(false) = got_pong {
                    ws_logger.log("Didn't receive pong from client");
                    break;
                }
                let ping_frame = Frame::new(true, OpCode::Ping, None, Payload::Owned(Vec::new()));
                if ws.write_frame(ping_frame).await.is_err() {
                    break;
                }
                got_pong = Some(false);
                continue;
            },
            frame = cancellation_token.run_until_cancelled(ws.read_frame()) => {
                    let Some(Ok(frame)) = frame else {break;};
                    frame
            },
        };

        let start = Instant::now();
        match frame.opcode {
            OpCode::Text | OpCode::Binary => {
                let payload = String::from_utf8(frame.payload.to_vec()).unwrap();
                ws_logger.log(&payload);
                let frame = Frame::new(true, frame.opcode, None, Payload::Owned(payload.into()));
                if let Err(e) = ws.write_frame(frame).await {
                    warn!("Error sending ws frame: {e}");
                    break;
                }
                ws_logger.log_duration(start.elapsed())
            }
            OpCode::Close => {
                break;
            },
            OpCode::Pong => {
                got_pong = Some(true);
            },
            _ => {}
        }
    }

    if !ws.is_closed() {
        let close_frame = Frame::close(
            CloseCode::Normal.into(),
            "Server is shutting down".as_bytes(),
        );
        select! {
             _ = ws.write_frame(close_frame) => {},
             _ = sleep(std::time::Duration::from_secs(1)) => {},
        };
    }
    ws_logger.log_connection_closed();
}

fn to_response(e: WebSocketError) -> Response<BoxBody<Bytes, BoxedError!()>> {
    let body = Full::new(Bytes::from(e.to_string()));
    let body = BoxBody::new(body.map_err(Into::into));
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(body)
        .unwrap()
}
