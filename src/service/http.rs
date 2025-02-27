use std::{convert::Infallible, error::Error};

use http_body_util::BodyExt;
use http_body_util::combinators::BoxBody;
use hyper::{
    Request, Response,
    body::{Body, Bytes},
};

use super::BoxedError;

pub(in crate::service) fn echo<B>(
    request: Request<B>,
) -> Result<Response<BoxBody<Bytes, BoxedError!()>>, Infallible>
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
