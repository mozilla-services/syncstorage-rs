use std::collections::{BTreeMap, HashMap};

use actix_web::{
    dev::{Payload, RequestHead},
    http::header::USER_AGENT,
    Error, FromRequest, HttpRequest,
};
use serde::{
    ser::{SerializeMap, Serializer},
    Serialize,
};

use crate::server::user_agent::parse_user_agent;

#[derive(Clone, Debug)]
pub struct Tags {
    pub tags: HashMap<String, String>,
}

impl Default for Tags {
    fn default() -> Tags {
        Tags {
            tags: HashMap::new(),
        }
    }
}

impl Serialize for Tags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_map(Some(self.tags.len()))?;
        for tag in self.tags.clone() {
            seq.serialize_entry(&tag.0, &tag.1)?;
        }
        seq.end()
    }
}

impl Tags {
    pub fn from_request_head(req_head: &RequestHead) -> Tags {
        // Return an Option<> type because the later consumers (ApiErrors) presume that
        // tags are optional and wrapped by an Option<> type.
        let mut tags = HashMap::new();
        if let Some(ua) = req_head.headers().get(USER_AGENT) {
            if let Ok(uas) = ua.to_str() {
                let (ua_result, metrics_os, metrics_browser) = parse_user_agent(uas);

                tags.insert("ua.os.family".to_owned(), metrics_os.to_owned());
                tags.insert("ua.browser.family".to_owned(), metrics_browser.to_owned());
                tags.insert("ua.name".to_owned(), ua_result.name.to_owned());
                tags.insert(
                    "ua.os.ver".to_owned(),
                    ua_result.os_version.to_owned().to_string(),
                );
                tags.insert("ua.browser.ver".to_owned(), ua_result.version.to_owned());
            }
        }
        tags.insert("uri.path".to_owned(), req_head.uri.to_string());
        tags.insert("uri.method".to_owned(), req_head.method.to_string());
        Tags { tags }
    }

    pub fn with_tags(tags: HashMap<String, String>) -> Tags {
        if tags.is_empty() {
            return Tags::default();
        }
        Tags { tags }
    }

    pub fn get(&self, label: &str) -> String {
        let none = "None".to_owned();
        self.tags.get(label).map(String::from).unwrap_or(none)
    }

    pub fn extend(&mut self, tags: HashMap<String, String>) {
        self.tags.extend(tags);
    }
}

impl FromRequest for Tags {
    type Config = ();
    type Error = Error;
    type Future = Result<Self, Self::Error>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let tags = {
            let exts = req.extensions();
            match exts.get::<Tags>() {
                Some(t) => t.clone(),
                None => Tags::from_request_head(req.head()),
            }
        };

        Ok(tags)
    }
}

impl Into<BTreeMap<String, String>> for Tags {
    fn into(self) -> BTreeMap<String, String> {
        let mut result = BTreeMap::new();

        for (k, v) in self.tags {
            result.insert(k.clone(), v.clone());
        }

        result
    }
}
