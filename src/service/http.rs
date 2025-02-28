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

    let mut response = Response::builder()
        .status(200)
        .version(parts.version)
        .extension(parts.extensions)
        .body(body)
        .unwrap();
    *response.headers_mut() = parts.headers;
    Ok(to_boxed_body(response))
}

pub(in crate::service) fn to_boxed_body<B>(
    resp: Response<B>,
) -> Response<BoxBody<Bytes, BoxedError!()>>
where
    B: Body<Data = Bytes> + Send + Sync + 'static,
    B::Error: Error + Send + Sync + 'static,
{
    resp.map(|b| {
        let b = b.map_err(Into::into);
        BoxBody::new(b)
    })
}
