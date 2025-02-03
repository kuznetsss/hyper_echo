use crate::log_utils::LogLevel;
use fastwebsockets::{
    upgrade::{is_upgrade_request, upgrade},
    FragmentCollector, Frame, OpCode, Payload, WebSocket, WebSocketError,
};
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{
    body::{Body, Bytes},
    upgrade::Upgraded,
    Request, Response, StatusCode,
};
use hyper_util::rt::TokioIo;
use std::{convert::Infallible, error::Error, future::Future, net::IpAddr};
use tracing::{info, warn};

macro_rules! BoxedError {
    () => {
        Box<dyn Error + Send + Sync + 'static>
    };
}

#[cfg(feature = "custom_trace")]
pub fn make_service(
    log_level: LogLevel,
    client_ip: IpAddr,
    id: u64,
) -> impl tower::Service<
    Request<hyper::body::Incoming>,
    Response = Response<BoxBody<Bytes, BoxedError!()>>,
    Error = Infallible,
    Future = impl Future,
> + Clone {
    use crate::custom_logger::LoggerLayer;

    let svc = tower::ServiceBuilder::new()
        .layer(LoggerLayer::new(log_level, client_ip, id))
        .service_fn(process_request);
    svc
}

#[cfg(feature = "tower_trace")]
pub fn make_service(
    log_level: LogLevel,
    client_ip: IpAddr,
    id: u64,
) -> impl tower::Service<
    Request<hyper::body::Incoming>,
    Response = Response<
        tower_http::trace::ResponseBody<
            BoxBody<Bytes, BoxedError!()>,
            tower_http::classify::NeverClassifyEos<tower_http::classify::ServerErrorsFailureClass>,
            crate::tower_logger::BodyLogger,
        >,
    >,
    Future = impl Future,
    Error = Infallible,
> + Clone
where
{
    use crate::tower_logger::{BodyLogger, OnRequestLogger, OnResponseLogger, SpanMaker};
    use tower_http::trace::TraceLayer;

    let svc = tower::ServiceBuilder::new()
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(SpanMaker::new(client_ip, id))
                .on_request(OnRequestLogger::new(log_level))
                .on_response(OnResponseLogger::new(log_level))
                .on_body_chunk(BodyLogger::new(log_level)),
        )
        .service_fn(process_request);
    svc
}

async fn process_request<B>(
    request: Request<B>,
) -> Result<Response<BoxBody<Bytes, BoxedError!()>>, Infallible>
where
    B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static,
{
    if is_upgrade_request(&request) {
        websocket_upgrade(request).await
    } else {
        echo(request).await
    }
}

async fn websocket_upgrade<B>(
    mut request: Request<B>,
) -> Result<Response<BoxBody<Bytes, BoxedError!()>>, Infallible>
where
    B: Send + Sync + 'static,
{
    match upgrade(&mut request) {
        Ok((response, fut)) => {
            tokio::spawn(async move {
                match fut.await {
                    Ok(ws) => {
                        echo_ws(ws).await;
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
    let body = request.into_body().map_err(Into::into);
    Ok(Response::new(BoxBody::new(body)))
}

async fn echo_ws(mut ws: WebSocket<TokioIo<Upgraded>>) {
    //let mut ws = FragmentCollector::new(ws);
    while let Ok(frame) = ws.read_frame().await {
        match frame.opcode {
            OpCode::Text | OpCode::Binary => {
                let payload = String::from_utf8(frame.payload.to_vec()).unwrap();
                info!("WS: {} {}", &payload, frame.fin);
                let frame = Frame::new(true, frame.opcode, None, Payload::Owned(payload.into()));
                if let Err(e) = ws.write_frame(frame).await {
                    warn!("Error sending frame: {e}");
                    break;
                }
            }
            OpCode::Close => {
                info!("got close");
                break;
            }
            OpCode::Continuation => {
                info!("Got Continuation");
            }
            _ => {}
        }
    }
}

fn to_response(e: WebSocketError) -> Response<BoxBody<Bytes, BoxedError!()>> {
    let body = Full::new(Bytes::from(e.to_string()));
    let body = BoxBody::new(body.map_err(Into::into));
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(body)
        .unwrap()
}
