// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! UUID Generator module stub.
//!
//! Provides generation of UUIDs across versions (v1 timestamp, v4 random, v7 Unix epoch time-ordered)
//! with batch generation and custom formatting options.

use uuid::Uuid;

/// Supported UUID versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UuidVersion {
    V1,
    #[default]
    V4,
    V7,
}

impl UuidVersion {
    pub fn from_index(index: i32) -> Self {
        match index {
            0 => UuidVersion::V1,
            1 => UuidVersion::V4,
            2 => UuidVersion::V7,
            _ => UuidVersion::V4,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            UuidVersion::V1 => "UUIDv1 (MAC & Time)",
            UuidVersion::V4 => "UUIDv4 (Random)",
            UuidVersion::V7 => "UUIDv7 (Epoch Time-Ordered)",
        }
    }
}

/// Input parameters for UUID generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UuidInput {
    pub version: UuidVersion,
    pub count: u32,
    pub uppercase: bool,
    pub hyphens: bool,
}

/// Output list of generated UUIDs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UuidOutput {
    pub uuids: Vec<String>,
}

/// Errors that can occur during UUID generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UuidError {
    InvalidCount(String),
    GenerationFailed(String),
}

/// Generates a batch of UUIDs matching the requested version and format.
///
/// Accepts a UuidInput structure.
pub fn generate(input: &UuidInput) -> Result<UuidOutput, UuidError> {
    let uuids = (0..input.count)
        .map(|_| {
            let uuid = generate_uuid(&input.version)?;
            let uuid = if input.hyphens {
                uuid
            } else {
                uuid.replace('-', "")
            };

            Ok(if input.uppercase {
                uuid.to_uppercase()
            } else {
                uuid
            })
        })
        .collect::<Result<Vec<_>, UuidError>>()?;

    Ok(UuidOutput { uuids })
}

/// Generates a single UUID given a version.
///
/// Accepts a UuidVersion enum.
fn generate_uuid(version: &UuidVersion) -> Result<String, UuidError> {
    // Generate UUID according to version
    let generated_uuid = match version {
        UuidVersion::V1 => {
            let mac = get_mac_address()?;
            let mac_begin: &[u8; 6] = mac
                .as_bytes()
                .get(..6)
                .and_then(|bytes| <&[u8; 6]>::try_from(bytes).ok())
                .ok_or_else(|| {
                    UuidError::GenerationFailed("MAC address retrieval failed.".to_string())
                })?;

            Uuid::now_v1(mac_begin)
        }
        UuidVersion::V4 => Uuid::new_v4(),
        UuidVersion::V7 => Uuid::now_v7(),
    };

    Ok(generated_uuid.to_string())
}

/// Returns the system MAC address as a string.
fn get_mac_address() -> Result<String, UuidError> {
    let address = match mac_address2::get_mac_address() {
        Ok(Some(mac)) => mac,
        Ok(None) => {
            return Err(UuidError::GenerationFailed(
                "Cannot read MAC address from system".to_string(),
            ))
        }
        Err(err) => return Err(UuidError::GenerationFailed(err.to_string())),
    };

    Ok(address.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not implemented yet"]
    fn test_generate_v4() {
        let input = UuidInput {
            version: UuidVersion::V4,
            count: 3,
            uppercase: false,
            hyphens: true,
        };
        let res = generate(&input).unwrap();
        assert_eq!(res.uuids.len(), 3);
    }

    #[test]
    #[ignore = "not implemented yet"]
    fn test_generate_v7() {
        let input = UuidInput {
            version: UuidVersion::V7,
            count: 1,
            uppercase: true,
            hyphens: false,
        };
        let res = generate(&input).unwrap();
        assert_eq!(res.uuids.len(), 1);
    }
}
