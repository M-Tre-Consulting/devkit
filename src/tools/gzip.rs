// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! GZip Compressor / Decompressor module stub.
//!
//! Provides gzip compression and decompression on strings and byte sequences
//! with size and compression ratio analysis.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
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
            let compressed_bytes = compress(input.input_data.as_bytes())?;
            let processed_size = compressed_bytes.len();
            let result_data = STANDARD.encode(&compressed_bytes);
            let original_size = input.input_data.len();
            let ratio_percentage = if original_size > 0 {
                ((original_size as f32 - processed_size as f32) / original_size as f32) * 100f32
            } else {
                0.0
            };

            Ok(GzipOutput {
                result_data,
                original_size,
                processed_size,
                ratio_percentage,
            })
        }
        GzipMode::Decompress => {
            let trimmed = input.input_data.trim();
            let compressed_bytes = match STANDARD.decode(trimmed) {
                Ok(bytes) => bytes,
                Err(_) => input.input_data.as_bytes().to_vec(),
            };
            let decompressed_bytes = decompress(&compressed_bytes)?;
            let result_data = String::from_utf8(decompressed_bytes).map_err(|e| {
                GzipError::DecompressionFailed(format!("Decompressed output is not valid UTF-8: {e}"))
            })?;
            let original_size = input.input_data.len();
            let processed_size = result_data.len();
            let ratio_percentage = if original_size > 0 {
                ((original_size as f32 - processed_size as f32) / original_size as f32) * 100f32
            } else {
                0.0
            };

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
    fn test_gzip_compression() {
        let input = GzipInput {
            input_data: "Lorem ipsum dolor sit amet".to_string(),
            mode: GzipMode::Compress,
        };
        let res = process(&input).unwrap();
        assert_eq!(res.original_size, 26);
        assert!(res.processed_size > 0);
        // Base64-encoded gzip stream must start with "H4sI"
        assert!(res.result_data.starts_with("H4sI"));
    }

    #[test]
    fn test_process_sample_phrase_roundtrip() {
        let sample = "DevKit: A native Rust + Slint developer toolbox for Android.".to_string();
        let compress_input = GzipInput {
            input_data: sample.clone(),
            mode: GzipMode::Compress,
        };
        let compress_output = process(&compress_input).expect("compression should succeed");
        assert!(compress_output.result_data.starts_with("H4sI"));
        assert_eq!(compress_output.original_size, sample.len());
        assert!(compress_output.processed_size > 0);

        // Decompress the resulting Base64 string back to the original text
        let decompress_input = GzipInput {
            input_data: compress_output.result_data,
            mode: GzipMode::Decompress,
        };
        let decompress_output = process(&decompress_input).expect("decompression should succeed");
        assert_eq!(decompress_output.result_data, sample);
    }

    #[test]
    fn test_gzip_empty_input() {
        let input = GzipInput {
            input_data: "".to_string(),
            mode: GzipMode::Compress,
        };
        assert_eq!(process(&input), Err(GzipError::EmptyInput));

        let input_decomp = GzipInput {
            input_data: "".to_string(),
            mode: GzipMode::Decompress,
        };
        assert_eq!(process(&input_decomp), Err(GzipError::EmptyInput));
    }

    #[test]
    fn test_compress_magic_header() {
        let data = b"Hello world!";
        let compressed = compress(data).expect("compression should succeed");
        // Gzip streams always start with ID1 = 0x1F, ID2 = 0x8B
        assert!(compressed.len() >= 10);
        assert_eq!(compressed[0], 0x1f);
        assert_eq!(compressed[1], 0x8b);
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let original = b"The quick brown fox jumps over the lazy dog";
        let compressed = compress(original).expect("compression should succeed");
        let decompressed = decompress(&compressed).expect("decompression should succeed");
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_compress_decompress_empty_slice() {
        let original = b"";
        let compressed = compress(original).expect("compressing empty slice should succeed");
        assert!(!compressed.is_empty()); // Gzip has header + footer metadata even for empty payload
        let decompressed = decompress(&compressed).expect("decompression should succeed");
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_compress_decompress_large_repetitive() {
        let original = "A".repeat(10_000);
        let compressed = compress(original.as_bytes()).expect("compression should succeed");
        assert!(compressed.len() < original.len()); // Repetitive text must compress significantly
        let decompressed = decompress(&compressed).expect("decompression should succeed");
        assert_eq!(decompressed, original.as_bytes());
    }

    #[test]
    fn test_compress_decompress_unicode() {
        let original = "Hello 🦀 世界! Sonderzeichen: äöüß, emojis: 🚀✨🎉".as_bytes();
        let compressed = compress(original).expect("compression should succeed");
        let decompressed = decompress(&compressed).expect("decompression should succeed");
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_decompress_invalid_data() {
        let invalid_data = b"This is definitely not a gzip stream";
        let result = decompress(invalid_data);
        assert!(matches!(result, Err(GzipError::DecompressionFailed(_))));
    }

    #[test]
    fn test_decompress_truncated_stream() {
        let original = b"Sample text to compress and then truncate";
        let compressed = compress(original).expect("compression should succeed");
        // Truncate stream halfway
        let truncated = &compressed[..compressed.len() / 2];
        let result = decompress(truncated);
        assert!(matches!(result, Err(GzipError::DecompressionFailed(_))));
    }

    #[test]
    fn test_decompress_empty_bytes() {
        let result = decompress(b"");
        assert!(matches!(result, Err(GzipError::DecompressionFailed(_))));
    }

    #[test]
    fn test_process_decompress_invalid_input() {
        let input = GzipInput {
            input_data: "corrupted or non-gzip payload".to_string(),
            mode: GzipMode::Decompress,
        };
        let result = process(&input);
        assert!(matches!(result, Err(GzipError::DecompressionFailed(_))));
    }

    #[test]
    fn test_gzip_types_and_defaults() {
        assert_eq!(GzipMode::default(), GzipMode::Compress);

        let default_output = GzipOutput::default();
        assert_eq!(default_output.result_data, "");
        assert_eq!(default_output.original_size, 0);
        assert_eq!(default_output.processed_size, 0);
        assert_eq!(default_output.ratio_percentage, 0.0);

        let err1 = GzipError::EmptyInput;
        let err2 = GzipError::EmptyInput;
        assert_eq!(err1, err2);
        assert_ne!(
            GzipError::CompressionFailed("a".to_string()),
            GzipError::CompressionFailed("b".to_string())
        );
    }

    #[test]
    fn test_ratio_percentage_calculation() {
        let input = GzipInput {
            input_data: "Repeat repeat repeat repeat repeat repeat repeat".to_string(),
            mode: GzipMode::Compress,
        };
        let out = process(&input).unwrap();
        let expected_ratio = ((out.original_size as f32 - out.processed_size as f32)
            / out.original_size as f32)
            * 100.0;
        assert!((out.ratio_percentage - expected_ratio).abs() < f32::EPSILON);
    }
}
