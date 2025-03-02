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

#[cfg(test)]
mod tests {
    use http_body_util::{BodyExt, Full};
    use hyper::{Method, Request, Version, body::Bytes, header::AUTHORIZATION};

    #[tokio::test]
    async fn echo_echoes() {
        let version = Version::HTTP_11;
        let header = (AUTHORIZATION, "some secret");
        let body = Full::new(Bytes::from("some body"));

        let request = Request::builder()
            .method(Method::POST)
            .extension("Some extension")
            .uri("some_uri")
            .version(version)
            .header(header.0.clone(), header.1)
            .body(body.clone())
            .unwrap();

        let response = super::echo(request).unwrap();
        

        assert_eq!(response.status(), 200);
        assert_eq!(response.extensions().len(), 1);
        assert_eq!(response.headers().get(header.0).unwrap(), header.1);
        assert_eq!(response.version(), version);

        let body = body.collect().await.unwrap().to_bytes();
        let response_body = response.into_body().collect().await.unwrap().to_bytes();

        assert_eq!(response_body, body);
    }
}
