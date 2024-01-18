use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse},
    HttpMessage,
};
use futures::future::Future;

use super::LogItems;

pub fn handle_request_log_line<B>(
    request: ServiceRequest,
    service: &impl Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
) -> impl Future<Output = Result<ServiceResponse<B>, actix_web::Error>> {
    let items = LogItems::from(request.head());
    request.extensions_mut().insert(items);
    let fut = service.call(request);

    async move {
        let sresp = fut.await?;

        if let Some(items) = sresp.request().extensions().get::<LogItems>() {
            info!("{}", items);
        }

        Ok(sresp)
    }
}
