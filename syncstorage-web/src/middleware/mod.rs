mod tokenserver_origin;
mod weave;

// # Web Middleware
//
// Matches the [Sync Storage middleware](https://github.com/mozilla-services/server-syncstorage/blob/master/syncstorage/tweens.py) (tweens).

use actix_web::{
    dev::{HttpResponseBuilder, ServiceResponse},
    http::StatusCode,
    middleware::errhandlers::ErrorHandlerResponse,
};

use crate::error::WeaveError;

pub use tokenserver_origin::EmitTokenserverOrigin;

pub fn render_404<B>(res: ServiceResponse<B>) -> actix_web::Result<ErrorHandlerResponse<B>> {
    if res.request().path().starts_with("/1.0/") {
        // Do not use a custom response for Tokenserver requests.
        Ok(ErrorHandlerResponse::Response(res))
    } else {
        // Replace the outbound error message with our own for Sync requests.
        let resp =
            HttpResponseBuilder::new(StatusCode::NOT_FOUND).json(WeaveError::UnknownError as u32);
        Ok(ErrorHandlerResponse::Response(ServiceResponse::new(
            res.request().clone(),
            resp.into_body(),
        )))
    }
}
