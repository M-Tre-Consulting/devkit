// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Hash Calculator module stub.
//!
//! Provides cryptographic and checksum hashing digests across multiple
//! algorithms: MD5, SHA-1, SHA-256, SHA-512, BLAKE2b, and BLAKE3.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use blake2::Blake2b512;
use sha1::{Digest, Sha1};
use sha2::{Sha256, Sha512};

/// Supported hashing algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HashAlgorithm {
    Md5,
    Sha1,
    #[default]
    Sha256,
    Sha512,
    Blake2b,
    Blake3,
}

impl HashAlgorithm {
    pub fn from_index(index: i32) -> Self {
        match index {
            0 => HashAlgorithm::Md5,
            1 => HashAlgorithm::Sha1,
            2 => HashAlgorithm::Sha256,
            3 => HashAlgorithm::Sha512,
            4 => HashAlgorithm::Blake2b,
            5 => HashAlgorithm::Blake3,
            _ => HashAlgorithm::Sha256,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HashAlgorithm::Md5 => "MD5",
            HashAlgorithm::Sha1 => "SHA-1",
            HashAlgorithm::Sha256 => "SHA-256",
            HashAlgorithm::Sha512 => "SHA-512",
            HashAlgorithm::Blake2b => "BLAKE2b",
            HashAlgorithm::Blake3 => "BLAKE3",
        }
    }
}

/// Input parameters for hashing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashInput {
    pub input_text: String,
    pub algorithm: HashAlgorithm,
}

/// Output digests from hashing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HashOutput {
    pub hex_digest: String,
    pub base64_digest: String,
}

/// Errors that can occur during hashing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashError {
    EmptyInput,
    HashingFailed(String),
}

/// Computes cryptographic hash digests for the given text and algorithm.
///
/// TODO: Implement hash digest computation when cryptography crate is added.
pub fn calculate(_input: &HashInput) -> Result<HashOutput, HashError> {
    if _input.input_text.is_empty() {
        return Err(HashError::EmptyInput);
    }

    let bytes = match _input.algorithm {
        HashAlgorithm::Md5 => calculate_md5(&_input.input_text),
        HashAlgorithm::Sha1 => calculate_sha1(&_input.input_text),
        HashAlgorithm::Sha256 => calculate_sha256(&_input.input_text),
        HashAlgorithm::Sha512 => calculate_sha512(&_input.input_text),
        HashAlgorithm::Blake2b => calculate_blake2b(&_input.input_text),
        HashAlgorithm::Blake3 => calculate_blake3(&_input.input_text),
    };

    let hex_digest = hex::encode(&bytes);
    let base64_digest = STANDARD.encode(&bytes);

    Ok(HashOutput {
        hex_digest,
        base64_digest,
    })
}

/// Calculates the MD5 hash and returns a lowercase hex string.
fn calculate_md5(input_text: &String) -> Vec<u8> {
    md5::compute(input_text.as_bytes()).to_vec()
}

/// Calculates the SHA1 hash and returns a lowercase hex string.
fn calculate_sha1(input_text: &String) -> Vec<u8> {
    Sha1::digest(input_text.as_bytes()).to_vec()
}

/// Calculates the SHA256 hash and returns a lowercase hex string.
fn calculate_sha256(input_text: &String) -> Vec<u8> {
    Sha256::digest(input_text.as_bytes()).to_vec()
}

/// Calculates the SHA512 hash and returns a lowercase hex string.
fn calculate_sha512(input_text: &String) -> Vec<u8> {
    Sha512::digest(input_text.as_bytes()).to_vec()
}

/// Calculates the BLAKE2B hash and returns a lowercase hex string.
fn calculate_blake2b(input_text: &String) -> Vec<u8> {
    Blake2b512::digest(input_text.as_bytes()).to_vec()
}

/// Calculates the BLAKE3 hash and returns a lowercase hex string.
fn calculate_blake3(input_text: &String) -> Vec<u8> {
    blake3::hash(input_text.as_bytes()).as_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not implemented yet"]
    fn test_calculate_sha256() {
        let input = HashInput {
            input_text: "hello world".to_string(),
            algorithm: HashAlgorithm::Sha256,
        };
        let res = calculate(&input).unwrap();
        assert!(!res.hex_digest.is_empty());
    }

    #[test]
    #[ignore = "not implemented yet"]
    fn test_calculate_empty_input() {
        let input = HashInput {
            input_text: "".to_string(),
            algorithm: HashAlgorithm::Sha256,
        };
        assert!(calculate(&input).is_err());
    }
}
