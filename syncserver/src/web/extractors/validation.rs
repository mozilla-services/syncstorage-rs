use validator::ValidationError;

use super::{
    RequestErrorLocation, BSO_MAX_SORTINDEX_VALUE, BSO_MAX_TTL, BSO_MIN_SORTINDEX_VALUE,
    VALID_ID_REGEX,
};

/// Convenience function to create a `ValidationError` with additional context
pub fn request_error(message: &'static str, location: RequestErrorLocation) -> ValidationError {
    let mut err = ValidationError::new(message);
    err.add_param("location".into(), &location);
    err
}

/// Verifies the BSO sortindex is in the valid range
pub fn validate_body_bso_sortindex(sort: i32) -> Result<(), ValidationError> {
    if (BSO_MIN_SORTINDEX_VALUE..=BSO_MAX_SORTINDEX_VALUE).contains(&sort) {
        Ok(())
    } else {
        Err(request_error("invalid value", RequestErrorLocation::Body))
    }
}

/// Verifies the BSO id string is valid
pub fn validate_body_bso_id(id: &str) -> Result<(), ValidationError> {
    if !VALID_ID_REGEX.is_match(id) {
        return Err(request_error("Invalid id", RequestErrorLocation::Body));
    }
    Ok(())
}

/// Verifies the BSO ttl is valid
pub fn validate_body_bso_ttl(ttl: u32) -> Result<(), ValidationError> {
    if ttl > BSO_MAX_TTL {
        return Err(request_error("Invalid TTL", RequestErrorLocation::Body));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::web::extractors::test_utils::{post_collection, USER_ID};

    #[actix_rt::test]
    async fn test_max_ttl() {
        let bso_body = json!([
            {"id": "123", "payload": "xxx", "sortindex": 23, "ttl": 94_608_000},
            {"id": "456", "payload": "xxxasdf", "sortindex": 23, "ttl": 999_999_999},
            {"id": "789", "payload": "xxxfoo", "sortindex": 23, "ttl": 1_000_000_000}
        ]);
        let result = post_collection("", &bso_body)
            .await
            .expect("Could not get result in test_valid_collection_post_request");
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(&result.collection, "tabs");
        assert_eq!(result.bsos.valid.len(), 2);
        assert_eq!(result.bsos.invalid.len(), 1);
        assert!(result.bsos.invalid.contains_key("789"));
    }
}
