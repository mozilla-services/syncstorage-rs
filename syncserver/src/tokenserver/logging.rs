use actix_web::{
    HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse},
};
use futures::future::Future;

use super::LogItems;

pub fn handle_request_log_line<B, S>(
    request: ServiceRequest,
    service: &S,
) -> impl Future<Output = Result<ServiceResponse<B>, actix_web::Error>> + use<B, S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
{
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
