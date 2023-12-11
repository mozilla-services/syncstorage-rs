/// gRPC metadata Resource prefix header
///
/// Generic across Google APIs. This "improves routing by the backend" as
/// described by other clients
const PREFIX_KEY: &str = "google-cloud-resource-prefix";

/// gRPC metadata Client information header
///
/// A `User-Agent` like header, likely its main use is for GCP's metrics
const METRICS_KEY: &str = "x-goog-api-client";

/// gRPC metadata Dynamic Routing header:
/// https://google.aip.dev/client-libraries/4222
///
/// See the googleapis protobuf for which routing header params are used for
/// each Spanner operation (under the `google.api.http` option).
///
/// https://github.com/googleapis/googleapis/blob/master/google/spanner/v1/spanner.proto
const ROUTING_KEY: &str = "x-goog-request-params";

/// gRPC metadata Leader Aware Routing header
///
/// Not well documented. Added to clients in early 2023 defaulting to disabled.
/// Clients have began defaulting it to enabled in late 2023.
///
/// "Enabling leader aware routing would route all requests in RW/PDML
/// transactions to the leader region." as described by other Spanner clients
const LEADER_AWARE_KEY: &str = "x-goog-spanner-route-to-leader";

/// The USER_AGENT string is a static value specified by Google.
/// Its meaning is not to be known to the uninitiated.
const USER_AGENT: &str = "gl-external/1.0 gccl/1.0";

/// Builds the [grpcio::Metadata] for all db operations
#[derive(Default)]
pub struct MetadataBuilder<'a> {
    prefix: &'a str,
    routing_params: Vec<(&'a str, &'a str)>,
    route_to_leader: bool,
}

impl<'a> MetadataBuilder<'a> {
    /// Initialize a new builder with a [PREFIX_KEY] header for the given
    /// resource
    pub fn with_prefix(prefix: &'a str) -> Self {
        Self {
            prefix,
            ..Default::default()
        }
    }

    /// Add a [ROUTING_KEY] header
    pub fn routing_param(mut self, key: &'a str, value: &'a str) -> Self {
        self.routing_params.push((key, value));
        self
    }

    /// Toggle the [LEADER_AWARE_KEY] header
    pub fn route_to_leader(mut self, route_to_leader: bool) -> Self {
        self.route_to_leader = route_to_leader;
        self
    }

    /// Build the [grpcio::Metadata]
    pub fn build(self) -> Result<grpcio::Metadata, grpcio::Error> {
        let mut meta = grpcio::MetadataBuilder::new();

        meta.add_str(PREFIX_KEY, self.prefix)?;
        meta.add_str(METRICS_KEY, USER_AGENT)?;
        if self.route_to_leader {
            meta.add_str(LEADER_AWARE_KEY, "true")?;
        }
        if !self.routing_params.is_empty() {
            meta.add_str(ROUTING_KEY, &self.routing_header())?;
        }
        Ok(meta.build())
    }

    fn routing_header(self) -> String {
        let mut ser = form_urlencoded::Serializer::new(String::new());
        for (key, val) in self.routing_params {
            ser.append_pair(key, val);
        }
        // python-spanner (python-api-core) doesn't encode '/':
        // https://github.com/googleapis/python-api-core/blob/6251eab/google/api_core/gapic_v1/routing_header.py#L85
        ser.finish().replace("%2F", "/")
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str};

    use super::{
        MetadataBuilder, LEADER_AWARE_KEY, METRICS_KEY, PREFIX_KEY, ROUTING_KEY, USER_AGENT,
    };

    // Resource paths should not start with a "/"
    pub const DB: &str = "projects/sync/instances/test/databases/sync1";
    pub const SESSION: &str = "projects/sync/instances/test/databases/sync1/sessions/f00B4r_quuX";

    #[test]
    fn metadata_basic() {
        let meta = MetadataBuilder::with_prefix(DB)
            .routing_param("session", SESSION)
            .routing_param("foo", "bar baz")
            .build()
            .unwrap();
        let meta: HashMap<_, _> = meta.into_iter().collect();

        assert_eq!(meta.len(), 3);
        assert_eq!(str::from_utf8(meta.get(PREFIX_KEY).unwrap()).unwrap(), DB);
        assert_eq!(
            str::from_utf8(meta.get(METRICS_KEY).unwrap()).unwrap(),
            USER_AGENT
        );
        assert_eq!(
            str::from_utf8(meta.get(ROUTING_KEY).unwrap()).unwrap(),
            format!("session={SESSION}&foo=bar+baz")
        );
    }

    #[test]
    fn leader_aware() {
        let meta = MetadataBuilder::with_prefix(DB)
            .route_to_leader(true)
            .build()
            .unwrap();
        let meta: HashMap<_, _> = meta.into_iter().collect();

        assert_eq!(meta.len(), 3);
        assert_eq!(
            str::from_utf8(meta.get(LEADER_AWARE_KEY).unwrap()).unwrap(),
            "true"
        );
    }
}
