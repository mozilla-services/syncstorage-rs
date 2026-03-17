use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::jwk::Jwk;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::json;

/// RSA private key for TEST_JWK.
pub const TEST_PRIVATE_KEY_PEM: &[u8] = b"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEAwDVLbkUbqt74yyEd1d5HQsmT/BVAMrausJCZq1FHyy9Ham+c
6Uobln07+G2Wj/X0PaDUBfKq12M90hPKAsI1gvAIVs9TL6BOvFYQArodTGxya0JL
v2GBSiJjgj8LQHL4DYbmvMsvtW6nHCGWtB9Fa9t9aN5R+/fcqj4mk0EzY5xL+cuz
zxuMRhreMUL+WE74bzA+b4IT2SIJvYy4yYY0stLvC1DSqKNn9Z6jWMP4qIFz1OCk
cJ74/wsUAIoicnVECe0jLEM+6xul6dIXwH2j7t105Kt+FSS+syOHqE+/oenfbrfO
n0v0ILjiBPJFa6f3mN1jO4RwyvYj6vut4f9KfQIDAQABAoIBACFRq8mKA9WPUPgf
fcVLArXmbTeT9HzGE8rKSkU7MhOcFshx4DTFsroX7Asw7hp3E7eSN2bvjeOIEdmm
sgxf37haxUtNJdm586Qs1Bow6q7KltwWkjxzGd9Azliv9pKdy3fG1J1SKKtOKvxS
q0X+rMFZe2gwL+yapz9Axl2c/hxMWRnU/4h2Z5yiAIMZ7fzWLvJcTU8Pd9yHeJDP
N13sTl4o+O8cVGxfFkDWJQYVS11J3XW4jN7OALYBBQi0qhA0+wPZ3rlWoyvEc7nv
TLtTVodNeu9pE8u1cPfaGt/NbWRw1HdgZx/KjHpfv2yd57ubLaeg3FftcWotXmdL
0I9T+ekCgYEA55+/N19DZBybFZ1hGaY6cxVgwbQ1bKKdFDYxdL3n9PKWhQBsK5J0
j6WkePzQV6fZgcGaioyatwghUe0uKC3F/18rB0XlJ0Mczq1USqG3XUF2Jc0nmn1S
tEvJhL6ntBWz33K+33GPFsZbAhZ+tsmObCHs+v/6EygyLRBODNPl4bkCgYEA1G+i
Q+DNO2Nea0ez7Cu68WYnQ3ccnPAJpCLgvvgmmBKWzl9A5MpspBHXv5P4DtdhXrmv
ZjAA0ihbsYIdG6v730wXTTolOnTBs4dNg2u8o/yAmnPlxViqFCd6ZgSjr6K3lQu7
Z+vW8P4tGAEIx2PHdbZwU3vZ1aF8ZqbM+g3vYOUCgYBUtI/6UQVVNDzm77IV7juL
4LKMxDmRa/qj7JmzhsuwQZMYOqpUWO/1pG78rAAJPmIF2OaKapcd/oQo8OMjYHH7
TTNwKnh+HkYHs02TKYbkPM9XTaqBDfnT469js1GjQxiPy+fP0Tix7IJVxiI6+IT0
OIfw1vH+VYHcBw10FX4JSQKBgQCq980X4+xIR4jNvj9Ha0pgzV38JfiZNXYM6yUF
jKFC8nL8VBzeBSu6P8HrJSMWjrCGk9pd23RNrr1c9uKGSrvC0nJObOVZTm42FkaD
5klDkQvPQkdBtEHtRnhzcnhp+gLVqUOCN4QdH/MaxnpSPjNgwRtVlO+Txwtfcg61
kFF/IQKBgQDe+BuvhET3OQtiiLUhZHkmtZTAc35+T1iArGVt/kFLARlhWRzGw4Ib
7+TDE+0KxKloqo+oZGJFMD9qJWhxLEnChrgOFx0lN/2FRvntab97AKfJf0R1wHKj
iXkKR8nsMwDtjLaoGsc8hbMT2nVRoPokfyIYJ5H+xzOfX1U05AZLjg==
-----END RSA PRIVATE KEY-----";

/// Private key for testing mismatching signatures.
pub const OTHER_PRIVATE_KEY_PEM: &[u8] = b"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEAyd0B8fYfYJ8lKbkhii838MjGp3VTNnveK7C/sxWHysEcGioH
TUvL9huz6tx41stE8Gi+uIGeXNzfcX3ZX2AFBnsT3z1+ndaPMw9dIhRWghrBp3u3
KB8soG7M81b0fLtTyymBbUVCnA9NRgFjr9A2WJacuJp8CGHLzeNhKMXJLrskUJLa
RktWeW79eyXTYuqiz0LlcDQAVjBeg30WPr80Wo6Scw55gC+c1dZl+P8FTX4/URxp
dimwnyU9mfje21ZPlNiIYHjPFpg5Zim/QasjMHzUAVTOEBCvwWY2KRMWJRFPju0i
goRkSV/MNq3doYKhQXM7cxKrhxsgNEhk9CTu7wIDAQABAoIBAFjx/eJsjWTYmtpo
jYPCzIZXIVk2FCVkrP9ZUQ6KxRusvUI2FKOVa1iU2lD5NnGGfWjk7myECJBobjgm
uLoSqAQ0BQyPnxPTL6PS+DmE9p07RusSUyDlo5dJWxs5zF6NeB2Du1i3dOMoxua6
w/764odkTcf1ogNbfB7LOstpYv0ohf+OmLIndHDws1qqfh58KHZlsRf4nz1TK73s
Pe5XbxpfjeDNp79czKJBr1HsMPKkCXw3VYhUNpoP7vC/UeSwRhuPrI7skO2+bXt7
SXR6rA7SxCgPKR5f3+LOV6GiU4ixc2DRMU/n8BROb+CIGE52+qbcjhNJXN9XYWoY
XDLlg/ECgYEA7yxlZez5hutCtnt/JvRaxaGywEHzcOazhUJNFMazs5INni8TKqZG
MqNUH0aa/GdT8dOE/HFcilSgr2glcchiJb7cIg7O1rDobIP8iZkml40Xs6S20dEW
83/+juX3VknPxxBGYkFo5ywSFKxD1zX9O9OB+u9DGHL+gWRgr4yZCwkCgYEA2BCk
bE0wzyuwNx4qmSsEAdtxh4ZyEgv46pXS18DH0oEUd/jzbAbBgz2vYIrWsHqszBDD
mnJr8YOhpqXMRmmyuPBuxvrNPwX+I9WE55NmVkY5NV1uDv26UgSWcuVMIBH2k9At
72F7F03zfovytcAuW1VbfjNESIZkA41JBOe7EDcCgYEA2nhbRvdoFu3fSoEUbKjY
IZ7KgQO9M2wIn7koX8oBbA4FknC9uT+Y77hxpv//on9gFo139IA4X8Nd49vmGEFK
JeBphFKybTm7lSQbEjVrIxQmilnzBUVRCavpAu7dN1zFBri/EhFdmYyQF4Ijlfoj
DvrsyCK1zyd7gwYFq1VqlsECgYAnK3UzcRb9J9VtWJmuZN74GzlMsXHylZsNpBWy
KW/QWLhGO6qdlef1C/TEUscy/TpgUFW1pTKueQeQN5R922GcJ3JdvlABMevtwSKz
/MPbtiVe6E4wh40Em3JO6ATR94+1IlOBhzGSev4+nc5lZq7Avgu1KEQjxcFR54Yq
TnxaJwKBgQDPDdfixZlqxuKiWewJupuDI9T2HBKp0BnRBzKZjSn+IuvCHUQC8xtv
yyVcq33mCxMSP9YqFrvZwjnoVMWCbVTPU7f6xzP4IGxXpySkVf8QwGp7Fc9q/XG4
8DA43GmyC52UynXzmBzTZfURio3qSQRFPtyZApJDcicR1nUw22KolA==
-----END RSA PRIVATE KEY-----";

/// Public JWK of TEST_PRIVATE_KEY_PEM.
pub const TEST_JWK: &str = r#"{
    "kty": "RSA",
    "alg": "RS256",
    "n": "wDVLbkUbqt74yyEd1d5HQsmT_BVAMrausJCZq1FHyy9Ham-c6Uobln07-G2Wj_X0PaDUBfKq12M90hPKAsI1gvAIVs9TL6BOvFYQArodTGxya0JLv2GBSiJjgj8LQHL4DYbmvMsvtW6nHCGWtB9Fa9t9aN5R-_fcqj4mk0EzY5xL-cuzzxuMRhreMUL-WE74bzA-b4IT2SIJvYy4yYY0stLvC1DSqKNn9Z6jWMP4qIFz1OCkcJ74_wsUAIoicnVECe0jLEM-6xul6dIXwH2j7t105Kt-FSS-syOHqE-_oenfbrfOn0v0ILjiBPJFa6f3mN1jO4RwyvYj6vut4f9KfQ",
    "e": "AQAB"
}"#;

pub fn test_jwk() -> Jwk {
    serde_json::from_str(TEST_JWK).unwrap()
}

/// Sign a SET with the given PEM.
pub fn make_set(
    sub: &str,
    aud: &str,
    events: serde_json::Value,
    exp_offset_secs: i64,
    private_key_pem: &[u8],
) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let claims = json!({
        "iss": "https://accounts.firefox.com/",
        "sub": sub,
        "aud": aud,
        "iat": now,
        "exp": now + exp_offset_secs,
        "jti": "wibble",
        "events": events,
    });
    let key = EncodingKey::from_rsa_pem(private_key_pem).expect("invalid PEM key");
    encode(&Header::new(Algorithm::RS256), &claims, &key).unwrap()
}
