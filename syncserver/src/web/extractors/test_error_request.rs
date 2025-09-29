use actix_web::{dev::Payload, http::header::HeaderMap, Error, FromRequest, HttpRequest};
use futures::future::{self, LocalBoxFuture};

#[derive(Debug)]
pub struct TestErrorRequest {
    pub headers: HeaderMap,
}

impl FromRequest for TestErrorRequest {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let headers = req.headers().clone();

        Box::pin(future::ok(TestErrorRequest { headers }))
    }
}
