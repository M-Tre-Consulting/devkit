// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! JWT Decoder module stub.
//!
//! Provides JSON Web Token header and payload decoding, signature extraction,
//! and expiration claim inspection.

use base64::prelude::{BASE64_URL_SAFE, BASE64_URL_SAFE_NO_PAD};
use base64::Engine as _;

/// Input parameters for JWT decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JwtInput {
    pub token: String,
}

/// Token validity and expiration status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExpirationStatus {
    #[default]
    Unknown,
    Active,
    Expired,
    NoExpirationClaim,
}

impl ExpirationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Active => "Active",
            Self::Expired => "Expired",
            Self::NoExpirationClaim => "No Expiration Claim",
        }
    }
}

/// Decoded JSON Web Token components and metadata.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JwtOutput {
    pub header_json: String,
    pub payload_json: String,
    pub signature: String,
    pub algorithm: String,
    pub expiration_status: ExpirationStatus,
    pub expiration_time: String,
}

/// Errors that can occur during JWT parsing and decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JwtError {
    EmptyToken,
    InvalidSegmentCount(usize),
    InvalidBase64(String),
    InvalidUtf8,
    InvalidJson(String),
}

impl std::fmt::Display for JwtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyToken => write!(f, "JWT token cannot be empty"),
            Self::InvalidSegmentCount(count) => {
                write!(f, "Expected 3 segments separated by dots, found {count}")
            }
            Self::InvalidBase64(val) => write!(f, "Base64URL decode failed: '{val}'"),
            Self::InvalidUtf8 => write!(f, "Payload or header is not valid UTF-8"),
            Self::InvalidJson(err) => write!(f, "Failed to parse segment JSON: '{err}'"),
        }
    }
}

impl std::error::Error for JwtError {}

/// Decodes a JSON Web Token into its header, payload, and signature components.
///
/// Accepts as input a JwtInput structure.
pub fn decode_jwt(input: &JwtInput) -> Result<JwtOutput, JwtError> {
    decode_token(&input.token)
}

/// Convenience function to decode a raw JWT token string.
fn decode_token(token: &str) -> Result<JwtOutput, JwtError> {
    // Trim spaces in token
    let token = token.trim();

    // Check for emptyness
    if token.is_empty() {
        return Err(JwtError::EmptyToken);
    }

    // Split segments
    let segments: Vec<&str> = token.split('.').collect();

    // Segments count must always be 3
    if segments.len() != 3 {
        return Err(JwtError::InvalidSegmentCount(segments.len()));
    }

    // Decode and parse the JSON value
    let header_str = decode_base64url_segment(segments[0])?;
    let header_val: serde_json::Value =
        serde_json::from_str(&header_str).map_err(|e| JwtError::InvalidJson(e.to_string()))?;
    let header_json = serde_json::to_string_pretty(&header_val)
        .map_err(|e| JwtError::InvalidJson(e.to_string()))?;

    // Inspect algorithm
    let algo = header_val
        .get("alg")
        .and_then(|v| v.as_str())
        .unwrap_or("None")
        .to_string();

    // Parse the JSON payload
    let payload_str = decode_base64url_segment(segments[1])?;
    let payload_val: serde_json::Value =
        serde_json::from_str(&payload_str).map_err(|e| JwtError::InvalidJson(e.to_string()))?;
    let payload_json = serde_json::to_string_pretty(&payload_val)
        .map_err(|e| JwtError::InvalidJson(e.to_string()))?;

    // Check for expiration
    let exp_timestamp = payload_val.get("exp").and_then(|v| v.as_i64());
    let (exp_status, exp_time) = match exp_timestamp {
        Some(exp) => {
            let now = chrono::Utc::now().timestamp();
            let status = if now > exp {
                ExpirationStatus::Expired
            } else {
                ExpirationStatus::Active
            };

            let time_str = chrono::DateTime::from_timestamp(exp, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| exp.to_string());

            (status, time_str)
        }
        None => (ExpirationStatus::NoExpirationClaim, "None".to_string()),
    };

    Ok(JwtOutput {
        header_json,
        payload_json,
        signature: segments[2].to_string(),
        algorithm: algo,
        expiration_status: exp_status,
        expiration_time: exp_time,
    })
}

/// Decodes a Base64 URL segment extracted from the JWT.
fn decode_base64url_segment(segment: &str) -> Result<String, JwtError> {
    // Decode the URL without padding
    let bytes = BASE64_URL_SAFE_NO_PAD
        .decode(segment)
        .or_else(|_| BASE64_URL_SAFE.decode(segment))
        .map_err(|e| JwtError::InvalidBase64(e.to_string()))?;

    Ok(String::from_utf8(bytes).map_err(|_| JwtError::InvalidUtf8)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JWT: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    #[test]
    #[ignore = "not implemented yet"]
    fn test_decode_sample_jwt() {
        let input = JwtInput {
            token: SAMPLE_JWT.to_string(),
        };
        let out = decode_jwt(&input).unwrap();
        assert!(out.header_json.contains("HS256"));
        assert!(out.payload_json.contains("John Doe"));
    }

    #[test]
    #[ignore = "not implemented yet"]
    fn test_decode_empty() {
        let input = JwtInput {
            token: "".to_string(),
        };
        assert!(decode_jwt(&input).is_err());
    }
}
