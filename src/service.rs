use crate::{log_utils::HttpLogLevel, ws_logger::WsLogger};
use fastwebsockets::upgrade::is_upgrade_request;
use http_body_util::combinators::BoxBody;
use hyper::{
    Request, Response,
    body::{Body, Bytes},
};
use std::{convert::Infallible, future::Future, net::IpAddr, pin::Pin};
use tokio_util::sync::CancellationToken;

mod http;
mod ws;

macro_rules! BoxedError {
    () => {
        Box<dyn std::error::Error + Send + Sync + 'static>
    };
}

pub(in crate::service) use BoxedError;

#[cfg(feature = "custom_trace")]
pub fn make_service<B>(
    log_level: HttpLogLevel,
    ws_logging_enabled: bool,
    client_ip: IpAddr,
    id: u64,
    cancellation_token: CancellationToken,
) -> impl tower::Service<
    Request<B>,
    Response = Response<BoxBody<Bytes, BoxedError!()>>,
    Error = Infallible,
    Future = impl Future,
> + Clone
where
    B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static,
{
    use crate::custom_logger::LoggerLayer;

    let svc = EchoService::new(ws_logging_enabled, client_ip, id, cancellation_token);
    tower::ServiceBuilder::new()
        .layer(LoggerLayer::new(log_level, client_ip, id))
        .service(svc)
}

#[cfg(feature = "tower_trace")]
pub fn make_service<B>(
    http_log_level: HttpLogLevel,
    ws_logging_enabled: bool,
    client_ip: IpAddr,
    id: u64,
    cancellation_token: CancellationToken,
) -> impl tower::Service<
    Request<B>,
    Response = Response<
        tower_http::trace::ResponseBody<
            BoxBody<Bytes, BoxedError!()>,
            tower_http::classify::NeverClassifyEos<tower_http::classify::ServerErrorsFailureClass>,
            crate::http_loggers::BodyLogger,
        >,
    >,
    Future = impl Future,
    Error = Infallible,
> + Clone
where
    B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static,
{
    use crate::http_loggers::{BodyLogger, OnRequestLogger, OnResponseLogger, SpanMaker};
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
        let ws_logger = self.ws_logger.clone();
        let cancellation_token = self.cancellation_token.clone();
        Box::pin(async move { process_request(req, ws_logger, cancellation_token) })
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

fn process_request<B>(
    request: Request<B>,
    ws_logger: WsLogger,
    cancellation_token: CancellationToken,
) -> Result<Response<BoxBody<Bytes, BoxedError!()>>, Infallible>
where
    B: Body<Data = Bytes, Error = hyper::Error> + Send + Sync + 'static,
{
    if is_upgrade_request(&request) {
        ws::websocket_upgrade(request, ws_logger, cancellation_token)
    } else {
        http::echo(request)
    }
}
