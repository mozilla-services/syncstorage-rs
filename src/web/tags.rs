use core::cell::RefMut;
use std::collections::{BTreeMap, HashMap};

use actix_http::Extensions;
use actix_web::{
    dev::{Payload, RequestHead},
    http::header::USER_AGENT,
    Error, FromRequest, HttpRequest,
};
use futures::future;
use futures::future::Ready;
use serde::{
    ser::{SerializeMap, Serializer},
    Serialize,
};
use serde_json::value::Value;
use slog::{Key, Record, KV};

use crate::server::user_agent::parse_user_agent;

#[derive(Clone, Debug, Default)]
pub struct Tags {
    pub tags: HashMap<String, String>,
    pub extra: HashMap<String, String>,
}

impl Serialize for Tags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_map(Some(self.tags.len()))?;
        for tag in self.tags.clone() {
            if !tag.1.is_empty() {
                seq.serialize_entry(&tag.0, &tag.1)?;
            }
        }
        seq.end()
    }
}

fn insert_if_not_empty(label: &str, val: &str, tags: &mut HashMap<String, String>) {
    if !val.is_empty() {
        tags.insert(label.to_owned(), val.to_owned());
    }
}

/// Tags are extra data to be recorded in metric and logging calls.
///
/// If additional tags are required or desired, you will need to add them to the
/// mutable extensions, e.g.
/// ```compile_fail
///      let mut tags = Tags::default();
///      tags.add_tag("SomeLabel", "whatever");
///      tags.commit(&mut request.extensions_mut());
/// ```
impl Tags {
    pub fn extend(&mut self, new_tags: Self) {
        self.tags.extend(new_tags.tags);
        self.extra.extend(new_tags.extra);
    }

    pub fn with_tags(tags: HashMap<String, String>) -> Tags {
        if tags.is_empty() {
            return Tags::default();
        }
        Tags {
            tags,
            extra: HashMap::new(),
        }
    }

    pub fn with_tag(key: &str, value: &str) -> Self {
        let mut tags = Tags::default();

        tags.tags.insert(key.to_owned(), value.to_owned());

        tags
    }

    pub fn add_extra(&mut self, key: &str, value: &str) {
        if !value.is_empty() {
            self.extra.insert(key.to_owned(), value.to_owned());
        }
    }

    pub fn add_tag(&mut self, key: &str, value: &str) {
        if !value.is_empty() {
            self.tags.insert(key.to_owned(), value.to_owned());
        }
    }

    pub fn get(&self, label: &str) -> String {
        let none = "None".to_owned();
        self.tags.get(label).map(String::from).unwrap_or(none)
    }

    pub fn tag_tree(self) -> BTreeMap<String, String> {
        let mut result = BTreeMap::new();

        for (k, v) in self.tags {
            result.insert(k.clone(), v.clone());
        }
        result
    }

    pub fn extra_tree(self) -> BTreeMap<String, Value> {
        let mut result = BTreeMap::new();

        for (k, v) in self.extra {
            result.insert(k.clone(), Value::from(v));
        }
        result
    }

    pub fn commit(self, exts: &mut RefMut<'_, Extensions>) {
        match exts.get_mut::<Tags>() {
            Some(t) => t.extend(self),
            None => exts.insert(self),
        }
    }
}

impl From<&RequestHead> for Tags {
    fn from(req_head: &RequestHead) -> Self {
        // Return an Option<> type because the later consumers (ApiErrors) presume that
        // tags are optional and wrapped by an Option<> type.
        let mut tags = HashMap::new();
        let mut extra = HashMap::new();
        if let Some(ua) = req_head.headers().get(USER_AGENT) {
            if let Ok(uas) = ua.to_str() {
                let (ua_result, metrics_os, metrics_browser) = parse_user_agent(uas);
                insert_if_not_empty("ua.os.family", metrics_os, &mut tags);
                insert_if_not_empty("ua.browser.family", metrics_browser, &mut tags);
                insert_if_not_empty("ua.name", ua_result.name, &mut tags);
                insert_if_not_empty("ua.os.ver", &ua_result.os_version.to_owned(), &mut tags);
                insert_if_not_empty("ua.browser.ver", ua_result.version, &mut tags);
                extra.insert("ua".to_owned(), uas.to_string());
            }
        }
        tags.insert("uri.method".to_owned(), req_head.method.to_string());
        // `uri.path` causes too much cardinality for influx but keep it in
        // extra for sentry
        extra.insert("uri.path".to_owned(), req_head.uri.to_string());
        Tags { tags, extra }
    }
}

impl FromRequest for Tags {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let tags = {
            let exts = req.extensions();
            match exts.get::<Tags>() {
                Some(t) => t.clone(),
                None => Tags::from(req.head()),
            }
        };

        future::ok(tags)
    }
}

impl From<Tags> for BTreeMap<String, String> {
    fn from(tags: Tags) -> BTreeMap<String, String> {
        let mut result = BTreeMap::new();

        for (k, v) in tags.tags {
            result.insert(k.clone(), v.clone());
        }
        result
    }
}

impl KV for Tags {
    fn serialize(&self, _rec: &Record<'_>, serializer: &mut dyn slog::Serializer) -> slog::Result {
        for (key, val) in &self.tags {
            serializer.emit_str(Key::from(key.clone()), val)?;
        }
        Ok(())
    }
}
