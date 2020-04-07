use actix_web::error::BlockingError;
use actix_web::web::block;
use actix_web::HttpResponse;

use futures::future::{Future, TryFutureExt};

use crate::error::ApiError;
use crate::web::extractors::TokenServerRequest;

pub struct TokenServerResult {}

pub fn get(
    request: TokenServerRequest,
) -> impl Future<Output = Result<HttpResponse, BlockingError<ApiError>>> {
    block(move || get_sync(request).map_err(Into::into)).map_ok(move |_result| {
        // TODO turn _result into a json response.
        HttpResponse::Ok()
            .content_type("application/json")
            .body("{}")
    })
}

pub fn get_sync(_request: TokenServerRequest) -> Result<TokenServerResult, ApiError> {
    // TODO Perform any blocking calls needed to respond to the tokenserver request.
    Ok(TokenServerResult {})
}
