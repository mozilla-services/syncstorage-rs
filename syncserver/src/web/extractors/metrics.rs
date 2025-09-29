use super::{BsoPutRequest, BsoRequest, CollectionPostRequest, CollectionRequest, MetaRequest};

pub trait EmitApiMetric {
    fn emit_api_metric(&self, label: &str);
}

macro_rules! impl_emit_api_metric {
    ($type:ty) => {
        impl EmitApiMetric for $type {
            fn emit_api_metric(&self, label: &str) {
                self.metrics.incr_with_tag(
                    label,
                    "tokenserver_origin",
                    &self.tokenserver_origin.to_string(),
                );
            }
        }
    };
}

impl_emit_api_metric!(MetaRequest);
impl_emit_api_metric!(CollectionRequest);
impl_emit_api_metric!(CollectionPostRequest);
impl_emit_api_metric!(BsoRequest);
impl_emit_api_metric!(BsoPutRequest);
