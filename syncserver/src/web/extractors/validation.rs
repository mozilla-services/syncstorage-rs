use validator::ValidationError;

use crate::web::extractors::{
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
