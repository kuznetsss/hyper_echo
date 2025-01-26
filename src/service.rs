use crate::log_utils::LogLevel;
use std::{convert::Infallible, error::Error, future::Future, net::IpAddr};
use http_body_util::{combinators::BoxBody, BodyExt, Empty};
use hyper::{body::{Body, Bytes}, header::{HeaderName, HeaderValue, CONNECTION, UPGRADE}, Request, Response, StatusCode};
use tracing::warn;

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
> + Clone
{
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
    use tower_http::trace::TraceLayer;
    use crate::tower_logger::{BodyLogger, OnRequestLogger, OnResponseLogger, SpanMaker};

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
    if is_websocket_upgrade(&request) {
        websocket_upgrade(request).await
    } else {
        echo(request).await
    }
}

async fn websocket_upgrade<B>(
    request: Request<B>,
) -> Result<Response<BoxBody<Bytes, BoxedError!()>>, Infallible>
where
    B: Send + Sync + 'static,
{
    tokio::task::spawn(async move {
        match hyper::upgrade::on(request).await {
            Ok(_upgraded) => {
                todo!()
            }
            Err(e) => warn!("Error upgrading connection: {e}"),
        }
    });
    let body = Empty::<Bytes>::new().map_err(Into::into);
    let response = Response::builder()
        .header(UPGRADE, HeaderValue::from_static("connection"))
        .header(CONNECTION, HeaderValue::from_static("Upgrade"))
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .body(BoxBody::new(body))
        .unwrap();
    Ok(response)
}

fn is_websocket_upgrade<B>(request: &Request<B>) -> bool {
    let check_header_value = |h: HeaderName, v: &str| {
        request
            .headers()
            .get(h)
            .map_or("", |s| s.to_str().unwrap_or(""))
            == v
    };
    check_header_value(UPGRADE, "websocket") && check_header_value(CONNECTION, "Upgrade")
}

async fn echo<B>(
    request: Request<B>,
) -> Result<Response<BoxBody<Bytes, BoxedError!()>>, Infallible>
where
    B: Body<Data = Bytes> + Send + Sync + 'static,
    B::Error: Error + Send + Sync + 'static,
{
    let body = request.into_body().map_err(Into::into);
    Ok(Response::new(BoxBody::new(body)))
}
