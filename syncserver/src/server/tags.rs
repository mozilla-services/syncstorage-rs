use std::collections::HashMap;

use actix_web::HttpMessage;
use serde_json::Value;

pub trait Taggable {
    /// Adds a tag to be included in any metric or Sentry error emitted from this point in the
    /// request lifecycle onwards. Tags **must** have low cardinality, meaning that the number of
    /// distinct possible values associated with a given tag must be small.
    fn add_tag(&self, key: String, value: String);

    /// Gets all the tags associated with `Self`.
    fn get_tags(&self) -> HashMap<String, String>;

    /// Adds an extra to be included in any Sentry error emitted from this point in the request
    /// lifecycle onwards. Extras are intended to be used to report additional metadata that have
    /// cardinality that is too high for tags. Note that extras will not be included with metrics.
    fn add_extra(&self, key: String, value: String);

    /// Gets all the extras associated with `Self`. This converts the values to `serde_json::Value`
    /// because the only caller / consumer for this function is the Sentry middleware, which uses
    /// `Value` for extras.
    fn get_extras(&self) -> HashMap<String, Value>;
}

impl<T> Taggable for T
where
    T: HttpMessage,
{
    fn add_tag(&self, key: String, value: String) {
        let mut exts = self.extensions_mut();

        match exts.get_mut::<Tags>() {
            Some(tags) => {
                tags.0.insert(key, value);
            }
            None => {
                let mut tags = Tags::default();
                tags.0.insert(key, value);
                exts.insert(tags);
            }
        }
    }

    fn get_tags(&self) -> HashMap<String, String> {
        self.extensions()
            .get::<Tags>()
            .map(|tags_ref| tags_ref.0.clone())
            .unwrap_or_default()
    }

    fn add_extra(&self, key: String, value: String) {
        let mut exts = self.extensions_mut();

        match exts.get_mut::<Extras>() {
            Some(extras) => {
                extras.0.insert(key, value);
            }
            None => {
                let mut extras = Extras::default();
                extras.0.insert(key, value);
                exts.insert(extras);
            }
        }
    }

    fn get_extras(&self) -> HashMap<String, Value> {
        self.extensions()
            .get::<Extras>()
            .map(|extras_ref| {
                extras_ref
                    .0
                    .clone()
                    .into_iter()
                    .map(|(k, v)| (k, Value::from(v)))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Tags are metadata that will be included in both Sentry errors and metric emissions. Given that
/// InfluxDB requires that tags have low cardinality, tags **must** have low cardinality. This
/// means that the number of distinct values for a given tag across every request must be low.
#[derive(Default)]
struct Tags(HashMap<String, String>);

// "Extras" are pieces of metadata with high cardinality to be included in Sentry errors.
#[derive(Default)]
struct Extras(HashMap<String, String>);
