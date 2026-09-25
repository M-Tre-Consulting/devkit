//! JWT Decoder module stub.
//!
//! Provides JSON Web Token header, payload decoding, and signature inspection.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JwtParts {
    pub header_json: String,
    pub payload_json: String,
    pub signature_hex: String,
    pub algorithm: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidTokenFormat,
    Base64DecodeError(String),
    InvalidUtf8,
    InvalidJson(String),
}

/// Decode a JSON Web Token into its header, payload, and signature components.
///
/// TODO: Split on '.', base64url decode parts, parse JSON header and payload, and extract claims.
pub fn decode_jwt(_token: &str) -> Result<JwtParts, Error> {
    // TODO: Implement JWT decoding logic
    todo!("decode JWT parts")
}
