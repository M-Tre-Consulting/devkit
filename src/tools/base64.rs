// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 MTRE Consulting

//! Base64 Encoder / Decoder module stub.
//!
//! Provides standard and URL-safe Base64 encoding and decoding with optional padding.

use base64::engine::general_purpose::{
    GeneralPurpose, STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD,
};

use base64::Engine as _;

/// Operation mode for Base64 tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Base64Mode {
    #[default]
    Encode,
    Decode,
}

/// Encoding format variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Base64Variant {
    #[default]
    Standard,
    UrlSafe,
}

/// Input parameters for Base64 conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Base64Input {
    pub input_text: String,
    pub mode: Base64Mode,
    pub variant: Base64Variant,
    pub padding: bool,
}

/// Output of Base64 conversion.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Base64Output {
    pub output_text: String,
}

/// Errors that can occur during Base64 decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Base64Error {
    InvalidBase64(String),
    EmptyInput,
}

/// Converts input text between raw and Base64 representation.
///
/// Checks the mode and performs the appropriate encoding or decoding operation.
pub fn convert(_input: &Base64Input) -> Result<Base64Output, Base64Error> {
    match _input.mode {
        Base64Mode::Encode => {
            let output_text = encode(_input);
            Ok(Base64Output { output_text })
        }
        Base64Mode::Decode => {
            let output_text = decode(_input);

            if let Ok(text) = output_text {
                Ok(Base64Output { output_text: text })
            } else {
                Err(output_text.err().unwrap())
            }
        }
    }
}

/// Encodes the input text string into base64 format with proper padding and variant.
fn encode(input: &Base64Input) -> String {
    let engine = select_engine(input.variant, input.padding);
    engine.encode(input.input_text.as_bytes())
}

/// Decodes the input text string from base64 format with proper padding and variant.
fn decode(input: &Base64Input) -> Result<String, Base64Error> {
    let engine = select_engine(input.variant, input.padding);
    let bytes = engine
        .decode(input.input_text.as_bytes())
        .map_err(|e| Base64Error::InvalidBase64(e.to_string()))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| Base64Error::InvalidBase64("Output is not valid UTF-8".to_string()))?;

    Ok(text)
}

/// Dynamically selects the appropriate Base64 engine based on variant and padding.
fn select_engine(variant: Base64Variant, padding: bool) -> GeneralPurpose {
    match (variant, padding) {
        (Base64Variant::Standard, true) => STANDARD,
        (Base64Variant::Standard, false) => STANDARD_NO_PAD,
        (Base64Variant::UrlSafe, true) => URL_SAFE,
        (Base64Variant::UrlSafe, false) => URL_SAFE_NO_PAD,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not implemented yet"]
    fn test_encode_standard() {
        let input = Base64Input {
            input_text: "hello".to_string(),
            mode: Base64Mode::Encode,
            variant: Base64Variant::Standard,
            padding: true,
        };
        let out = convert(&input).unwrap();
        assert_eq!(out.output_text, "aGVsbG8=");
    }

    #[test]
    #[ignore = "not implemented yet"]
    fn test_decode_standard() {
        let input = Base64Input {
            input_text: "aGVsbG8=".to_string(),
            mode: Base64Mode::Decode,
            variant: Base64Variant::Standard,
            padding: true,
        };
        let out = convert(&input).unwrap();
        assert_eq!(out.output_text, "hello");
    }
}
