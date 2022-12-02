use actix_web::{dev::Payload, web::Data, FromRequest, HttpRequest};
use futures::future;
use futures::future::Ready;
use syncserver_common::{Metrics, Taggable};

use super::ServerState;

pub struct MetricsWrapper(pub Metrics);

impl FromRequest for MetricsWrapper {
    type Config = ();
    type Error = ();
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let client = req
            .app_data::<Data<ServerState>>()
            .map(|state| state.statsd_client.clone());

        if client.is_none() {
            warn!("⚠️ metric error: No App State");
        }

        future::ok(MetricsWrapper(Metrics {
            client: client.as_deref().cloned(),
            tags: req.get_tags(),
            timer: None,
        }))
    }
}
