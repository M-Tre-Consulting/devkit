// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! GZip Compressor / Decompressor module stub.
//!
//! Provides gzip compression and decompression on strings and byte sequences
//! with size and compression ratio analysis.

use flate2::write::{GzDecoder, GzEncoder};
use flate2::Compression;
use std::io::Write;

/// Operation mode for GZip processor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GzipMode {
    #[default]
    Compress,
    Decompress,
}

/// Input parameters for gzip compression/decompression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GzipInput {
    pub input_data: String,
    pub mode: GzipMode,
}

/// Result of compression/decompression with size statistics.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GzipOutput {
    pub result_data: String,
    pub original_size: usize,
    pub processed_size: usize,
    pub ratio_percentage: f32,
}

/// Errors that can occur during compression or decompression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GzipError {
    CompressionFailed(String),
    DecompressionFailed(String),
    EmptyInput,
}

/// Processes input by compressing to Base64-encoded GZip or decompressing back to text.
///
/// Implements compression and decompression using flate2 crate.
pub fn process(input: &GzipInput) -> Result<GzipOutput, GzipError> {
    // Check input before proceeding
    if input.input_data.is_empty() {
        return Err(GzipError::EmptyInput);
    }

    // Match mode and run operations
    match input.mode {
        GzipMode::Compress => {
            let result_data = compress(input.input_data.as_bytes())?;
            let result_data: String = String::from_utf8_lossy(&result_data).into_owned();
            let original_size = input.input_data.len();
            let processed_size = result_data.len();
            let ratio_percentage = (processed_size as f32 / original_size as f32) * 100f32;

            Ok(GzipOutput {
                result_data,
                original_size,
                processed_size,
                ratio_percentage,
            })
        }
        GzipMode::Decompress => {
            let result_data = decompress(input.input_data.as_bytes())?;
            let result_data: String = String::from_utf8_lossy(&result_data).into_owned();
            let original_size = input.input_data.len();
            let processed_size = result_data.len();
            let ratio_percentage = (processed_size as f32 / original_size as f32) * 100f32;

            Ok(GzipOutput {
                result_data,
                original_size,
                processed_size,
                ratio_percentage,
            })
        }
    }
}

/// Compress a byte slice using the gzip algorithm.
pub fn compress(input: &[u8]) -> Result<Vec<u8>, GzipError> {
    // Compression buffer
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());

    // Write all data into the encoder
    encoder
        .write_all(input)
        .map_err(|e| GzipError::CompressionFailed(e.to_string()))?;

    let compressed = encoder
        .finish()
        .map_err(|e| GzipError::CompressionFailed(e.to_string()))?;

    Ok(compressed)
}

/// Decompress a gzip-compressed byte slice.
pub fn decompress(input: &[u8]) -> Result<Vec<u8>, GzipError> {
    // Decompression buffer
    let mut decoder = GzDecoder::new(Vec::new());

    // Write compressed data to buffer
    decoder
        .write_all(input)
        .map_err(|e| GzipError::DecompressionFailed(e.to_string()))?;

    // Decompress data
    let decompressed = decoder
        .finish()
        .map_err(|e| GzipError::DecompressionFailed(e.to_string()))?;

    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not implemented yet"]
    fn test_gzip_compression() {
        let input = GzipInput {
            input_data: "Lorem ipsum dolor sit amet".to_string(),
            mode: GzipMode::Compress,
        };
        let res = process(&input).unwrap();
        assert!(res.processed_size > 0);
    }

    #[test]
    #[ignore = "not implemented yet"]
    fn test_gzip_empty_input() {
        let input = GzipInput {
            input_data: "".to_string(),
            mode: GzipMode::Compress,
        };
        assert!(process(&input).is_err());
    }
}
