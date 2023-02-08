use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse},
    HttpMessage,
};
use futures::future::Future;

use super::LogItems;

pub fn handle_request_log_line(
    request: ServiceRequest,
    service: &mut impl Service<
        Request = ServiceRequest,
        Response = ServiceResponse,
        Error = actix_web::Error,
    >,
) -> impl Future<Output = Result<ServiceResponse, actix_web::Error>> {
    let items = LogItems::from(request.head());
    request.extensions_mut().insert(items);
    let fut = service.call(request);

    Box::pin(async move {
        let sresp = fut.await?;

        if let Some(items) = sresp.request().extensions().get::<LogItems>() {
            info!("{}", items);
        }

        Ok(sresp)
    })
}
