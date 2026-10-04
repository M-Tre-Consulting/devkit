// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! JSON ↔ YAML Converter module stub.
//!
//! Provides bidirectional conversion between JSON and YAML structured data.

use serde::Serialize;
use serde_json::{ser::PrettyFormatter, Value};

/// Conversion direction for the translator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConversionDirection {
    #[default]
    JsonToYaml,
    YamlToJson,
}

/// Input parameters for JSON/YAML translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonYamlInput {
    pub source_text: String,
    pub direction: ConversionDirection,
    pub indent_size: u8,
}

/// Output formatted result string.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JsonYamlOutput {
    pub converted_text: String,
}

/// Errors that can occur during format conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonYamlError {
    InvalidJson(String),
    InvalidYaml(String),
    EmptyInput,
}

/// Converts source text bidirectionally between JSON and YAML.
///
/// TODO: Implement parsing and serialization logic when serde_json / serde_yaml are added.
pub fn convert(input: &JsonYamlInput) -> Result<JsonYamlOutput, JsonYamlError> {
    match input.direction {
        ConversionDirection::YamlToJson => {
            let result = convert_yaml_to_json(&input.source_text, input.indent_size)?;
            Ok(JsonYamlOutput {
                converted_text: result,
            })
        }
        ConversionDirection::JsonToYaml => {
            let result = convert_json_to_yaml(&input.source_text)?;
            Ok(JsonYamlOutput {
                converted_text: result,
            })
        }
    }
}

/// Convenience helper to convert JSON string to YAML.
///
/// Accepts an input string and parses it into raw JSON deserialized data.
/// If the deserialization succeeds, then the inverse process is triggered
/// for YAML generation.
fn convert_json_to_yaml(input: &str) -> Result<String, JsonYamlError> {
    // Check if the input is empty and raise proper error
    if input.trim().is_empty() {
        return Err(JsonYamlError::EmptyInput);
    }

    // Parse the JSON with error handling
    let parsed_json: Value =
        serde_json::from_str(input).map_err(|e| JsonYamlError::InvalidJson(e.to_string()))?;

    // Serialize into YAML
    serde_yaml::to_string(&parsed_json).map_err(|e| JsonYamlError::InvalidYaml(e.to_string()))
}

/// Convenience helper to convert YAML string to JSON.
///
/// Accepts an input string and parses it into raw YAML deserialized data.
/// If the deserialization succeeds, then the inverse process is triggered
/// for JSON generation.
fn convert_yaml_to_json(input: &str, indent_size: u8) -> Result<String, JsonYamlError> {
    // Setup custom indent
    let indent = " ".repeat(indent_size as usize);
    let formatter = PrettyFormatter::with_indent(indent.as_bytes());

    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);

    // Check for empty input
    if input.trim().is_empty() {
        return Err(JsonYamlError::EmptyInput);
    }

    // Parse YAML with error handling
    let parsed_yaml: Value =
        serde_yaml::from_str(input).map_err(|e| JsonYamlError::InvalidYaml(e.to_string()))?;

    // Serialize into JSON
    parsed_yaml
        .serialize(&mut ser)
        .map_err(|e| JsonYamlError::InvalidJson(e.to_string()))?;

    String::from_utf8(buf).map_err(|e| JsonYamlError::InvalidJson(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_yaml() {
        let input = JsonYamlInput {
            source_text: r#"{"name": "DevKit"}"#.to_string(),
            direction: ConversionDirection::JsonToYaml,
            indent_size: 2,
        };
        let res = convert(&input).unwrap();
        assert!(res.converted_text.contains("name: DevKit"));
    }

    #[test]
    fn test_yaml_to_json() {
        let input = JsonYamlInput {
            source_text: "name: DevKit\n".to_string(),
            direction: ConversionDirection::YamlToJson,
            indent_size: 2,
        };
        let res = convert(&input).unwrap();
        assert!(res.converted_text.contains(r#""name": "DevKit""#));
    }
}
