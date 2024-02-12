use crate::{
    crypto::{Crypto, CryptoImpl},
    MakeTokenPlaintext,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use tokenserver_common::TokenserverError;
// Those two constants were pulled directly from
// https://github.com/mozilla-services/tokenlib/blob/91ec9e2c922e55306eddba1394590a88f3b10602/tokenlib/__init__.py#L43-L45
// We could change them, but we'd want to make sure that we also change them syncstorage, however
// that would cause temporary auth issues for anyone with an old pre-new-value token
const HKDF_SIGNING_INFO: &[u8] = b"services.mozilla.com/tokenlib/v1/signing";
const HKDF_INFO_DERIVE: &[u8] = b"services.mozilla.com/tokenlib/v1/derive/";

pub struct Tokenlib {}

#[derive(Debug, Serialize, Deserialize)]
struct Token<'a> {
    #[serde(flatten)]
    plaintext: MakeTokenPlaintext,
    salt: &'a str,
}

impl Tokenlib {
    pub fn get_token_and_derived_secret(
        plaintext: MakeTokenPlaintext,
        shared_secret: &str,
    ) -> Result<(String, String), TokenserverError> {
        // First we make the token itself, the code blow was ported from:
        // https://github.com/mozilla-services/tokenlib/blob/91ec9e2c922e55306eddba1394590a88f3b10602/tokenlib/__init__.py#L96-L97
        let crypto_lib = CryptoImpl {};
        let mut salt_bytes = [0u8; 3];
        crypto_lib.rand_bytes(&mut salt_bytes)?;
        let salt = hex::encode(salt_bytes);
        let token_str = serde_json::to_string(&Token {
            plaintext,
            salt: &salt,
        })
        .map_err(|_| TokenserverError::internal_error())?;
        let hmac_key = crypto_lib.hkdf(shared_secret, None, HKDF_SIGNING_INFO)?;
        let signature = crypto_lib.hmac_sign(&hmac_key, token_str.as_bytes())?;
        let mut token_bytes = Vec::with_capacity(token_str.len() + signature.len());
        token_bytes.extend_from_slice(token_str.as_bytes());
        token_bytes.extend_from_slice(&signature);
        let token = base64::engine::general_purpose::URL_SAFE.encode(token_bytes);
        // Now that we finialized the token, lets generate our per token secret
        // The code below was ported from:
        // https://github.com/mozilla-services/tokenlib/blob/91ec9e2c922e55306eddba1394590a88f3b10602/tokenlib/__init__.py#L158-L159
        let mut info = Vec::with_capacity(HKDF_INFO_DERIVE.len() + token.as_bytes().len());
        info.extend_from_slice(HKDF_INFO_DERIVE);
        info.extend_from_slice(token.as_bytes());

        let per_token_secret = crypto_lib.hkdf(shared_secret, Some(salt.as_bytes()), &info)?;
        let per_token_secret = base64::engine::general_purpose::URL_SAFE.encode(per_token_secret);
        Ok((token, per_token_secret))
    }
}

#[cfg(test)]
mod tests {
    use crate::{crypto::SHA256_OUTPUT_LEN, TokenserverOrigin};

    use super::*;

    #[test]
    fn test_generate_valid_token_and_per_token_secret() -> Result<(), TokenserverError> {
        // First we verify that the token we generated has a valid
        // and correct HMAC signature if signed using the same key
        let plaintext = MakeTokenPlaintext {
            node: "https://www.example.com".to_string(),
            fxa_kid: "kid".to_string(),
            fxa_uid: "user uid".to_string(),
            hashed_fxa_uid: "hased uid".to_string(),
            hashed_device_id: "hashed device id".to_string(),
            expires: 1031,
            uid: 13,
            tokenserver_origin: TokenserverOrigin::Rust,
        };
        let secret = "foobar";
        let crypto_impl = CryptoImpl {};
        let hmac_key = crypto_impl.hkdf(secret, None, HKDF_SIGNING_INFO).unwrap();
        let (b64_token, per_token_secret) =
            Tokenlib::get_token_and_derived_secret(plaintext.clone(), secret).unwrap();
        let token = base64::engine::general_purpose::URL_SAFE
            .decode(&b64_token)
            .unwrap();
        let token_size = token.len();
        let signature = &token[token_size - SHA256_OUTPUT_LEN..];
        let payload = &token[..token_size - SHA256_OUTPUT_LEN];
        crypto_impl
            .hmac_verify(&hmac_key, payload, signature)
            .unwrap();
        // Then we verify that the payload value we signed, is a valid
        // Token represented by our Token struct, and has exactly the same
        // plain_text values
        let token_data = serde_json::from_slice::<Token<'_>>(payload).unwrap();
        assert_eq!(token_data.plaintext, plaintext);
        // Finally, we verify that the same per_token_secret can be derived given the payload
        // and the shared secret
        let mut info = Vec::with_capacity(HKDF_INFO_DERIVE.len() + b64_token.as_bytes().len());
        info.extend_from_slice(HKDF_INFO_DERIVE);
        info.extend_from_slice(b64_token.as_bytes());

        let expected_per_token_secret =
            crypto_impl.hkdf(secret, Some(token_data.salt.as_bytes()), &info)?;
        let expected_per_token_secret =
            base64::engine::general_purpose::URL_SAFE.encode(expected_per_token_secret);

        assert_eq!(expected_per_token_secret, per_token_secret);

        Ok(())
    }
}
