// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 MTRE Consulting

//! Tool execution handlers and Slint UI bridge adapters.
//!
//! Provides thin marshalling wrappers between Slint UI inputs/outputs and the tool backend
//! modules. Each handler reads from the input parameters, calls the corresponding tool function,
//! and returns a Result with either the typed output or a formatted error message.
//!
//! Calls are wrapped in `catch_unwind` to gracefully handle `todo!()` stubs during development
//! without terminating the application process.

use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::tools::base64::{self, Base64Input, Base64Mode, Base64Output, Base64Variant};
use crate::tools::chmod::{self, ChmodInput, ChmodOutput, PermissionBits};
use crate::tools::color::{self, ColorFormat, ColorInput, ColorOutput};
use crate::tools::contrast::{self, ContrastInput, ContrastResult};
use crate::tools::cron::{self, CronInput, CronOutput};
use crate::tools::formatter::{self, FormatterInput, FormatterOutput, SupportedLanguage};
use crate::tools::gzip::{self, GzipInput, GzipMode, GzipOutput};
use crate::tools::hash::{self, HashAlgorithm, HashInput, HashOutput};
use crate::tools::json_yaml::{self, ConversionDirection, JsonYamlInput, JsonYamlOutput};
use crate::tools::jwt::{self, JwtInput, JwtOutput};
use crate::tools::regex::{self, RegexFlags, RegexInput, RegexOutput};
use crate::tools::subnet::{self, SubnetInput, SubnetOutput};
use crate::tools::timestamp::{self, TimestampInput, TimestampMode, TimestampOutput, TimezoneOption};
use crate::tools::uuid::{self, UuidInput, UuidOutput, UuidVersion};

/// Handler for Subnet Calculator.
pub fn handle_subnet(cidr: &str) -> Result<SubnetOutput, String> {
    let input = SubnetInput {
        ip_or_cidr: cidr.to_string(),
        prefix: None,
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| subnet::calculate(&input)));
    match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(format!("Subnet error: {err:?}")),
        Err(_) => Err("Subnet calculation not yet implemented (todo stub)".to_string()),
    }
}

/// Handler for Hash Calculator.
pub fn handle_hash(text: &str, algorithm_idx: i32) -> Result<HashOutput, String> {
    let algorithm = HashAlgorithm::from_index(algorithm_idx);
    let input = HashInput {
        input_text: text.to_string(),
        algorithm,
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| hash::calculate(&input)));
    match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(format!("Hash error: {err:?}")),
        Err(_) => Err("Hash calculation not yet implemented (todo stub)".to_string()),
    }
}

/// Handler for Base64 Encoder/Decoder.
pub fn handle_base64(
    text: &str,
    is_encode: bool,
    is_url_safe: bool,
    padding: bool,
) -> Result<Base64Output, String> {
    let mode = if is_encode {
        Base64Mode::Encode
    } else {
        Base64Mode::Decode
    };
    let variant = if is_url_safe {
        Base64Variant::UrlSafe
    } else {
        Base64Variant::Standard
    };
    let input = Base64Input {
        input_text: text.to_string(),
        mode,
        variant,
        padding,
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| base64::convert(&input)));
    match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(format!("Base64 error: {err:?}")),
        Err(_) => Err("Base64 operation not yet implemented (todo stub)".to_string()),
    }
}

/// Handler for UUID Generator.
pub fn handle_uuid(
    version_idx: i32,
    count: usize,
    uppercase: bool,
    hyphens: bool,
) -> Result<UuidOutput, String> {
    let version = UuidVersion::from_index(version_idx);
    let input = UuidInput {
        version,
        count: count as u32,
        uppercase,
        hyphens,
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| uuid::generate(&input)));
    match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(format!("UUID error: {err:?}")),
        Err(_) => Err("UUID generation not yet implemented (todo stub)".to_string()),
    }
}

/// Handler for Timestamp Converter.
pub fn handle_timestamp(
    timestamp: &str,
    timezone_str: &str,
) -> Result<TimestampOutput, String> {
    let timezone = if timezone_str.eq_ignore_ascii_case("Local") {
        TimezoneOption::Local
    } else {
        TimezoneOption::Utc
    };
    let input = TimestampInput {
        input_value: timestamp.to_string(),
        mode: TimestampMode::EpochToHuman,
        timezone,
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| timestamp::convert(&input)));
    match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(format!("Timestamp error: {err:?}")),
        Err(_) => Err("Timestamp conversion not yet implemented (todo stub)".to_string()),
    }
}

/// Handler for Regex Tester.
pub fn handle_regex(
    pattern: &str,
    text: &str,
    case_insensitive: bool,
    multiline: bool,
    dot_matches_all: bool,
) -> Result<RegexOutput, String> {
    let input = RegexInput {
        pattern: pattern.to_string(),
        test_string: text.to_string(),
        flags: RegexFlags {
            case_insensitive,
            multiline,
            dot_matches_newline: dot_matches_all,
        },
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| regex::evaluate(&input)));
    match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(format!("Regex error: {err:?}")),
        Err(_) => Err("Regex evaluation not yet implemented (todo stub)".to_string()),
    }
}

/// Handler for JSON ↔ YAML Converter.
pub fn handle_json_yaml(
    source: &str,
    is_json_to_yaml: bool,
    indent: usize,
) -> Result<JsonYamlOutput, String> {
    let direction = if is_json_to_yaml {
        ConversionDirection::JsonToYaml
    } else {
        ConversionDirection::YamlToJson
    };
    let input = JsonYamlInput {
        source_text: source.to_string(),
        direction,
        indent_size: indent as u8,
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| json_yaml::convert(&input)));
    match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(format!("JSON/YAML error: {err:?}")),
        Err(_) => Err("JSON/YAML conversion not yet implemented (todo stub)".to_string()),
    }
}

/// Handler for CRON Parser.
pub fn handle_cron(expression: &str) -> Result<CronOutput, String> {
    let input = CronInput {
        expression: expression.to_string(),
        next_runs_count: 5,
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| cron::parse(&input)));
    match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(format!("CRON error: {err:?}")),
        Err(_) => Err("CRON parsing not yet implemented (todo stub)".to_string()),
    }
}

/// Handler for GZip Compressor.
pub fn handle_gzip(data: &str, is_compress: bool) -> Result<GzipOutput, String> {
    let mode = if is_compress {
        GzipMode::Compress
    } else {
        GzipMode::Decompress
    };
    let input = GzipInput {
        input_data: data.to_string(),
        mode,
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| gzip::process(&input)));
    match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(format!("GZip error: {err:?}")),
        Err(_) => Err("GZip compression not yet implemented (todo stub)".to_string()),
    }
}

/// Handler for Code Formatter.
pub fn handle_formatter(
    code: &str,
    language_idx: i32,
    indent: u8,
) -> Result<FormatterOutput, String> {
    let language = SupportedLanguage::from_index(language_idx);
    let input = FormatterInput {
        source_code: code.to_string(),
        language,
        indent_size: indent,
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| formatter::format(&input)));
    match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(format!("Formatter error: {err:?}")),
        Err(_) => Err("Code formatting not yet implemented (todo stub)".to_string()),
    }
}

/// Handler for Chmod Calculator.
pub fn handle_chmod(
    owner_r: bool,
    owner_w: bool,
    owner_x: bool,
    group_r: bool,
    group_w: bool,
    group_x: bool,
    other_r: bool,
    other_w: bool,
    other_x: bool,
) -> Result<ChmodOutput, String> {
    let input = ChmodInput {
        owner: PermissionBits {
            read: owner_r,
            write: owner_w,
            execute: owner_x,
        },
        group: PermissionBits {
            read: group_r,
            write: group_w,
            execute: group_x,
        },
        other: PermissionBits {
            read: other_r,
            write: other_w,
            execute: other_x,
        },
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| chmod::calculate_permissions(&input)));
    match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(format!("Chmod error: {err}")),
        Err(_) => Err("Chmod calculation not yet implemented (todo stub)".to_string()),
    }
}

/// Handler for Color Converter.
pub fn handle_color(value: &str, format_idx: i32) -> Result<ColorOutput, String> {
    let format = ColorFormat::from_index(format_idx);
    let input = ColorInput {
        value: value.to_string(),
        format,
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| color::convert_color(&input)));
    match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(format!("Color error: {err}")),
        Err(_) => Err("Color conversion not yet implemented (todo stub)".to_string()),
    }
}

/// Handler for Contrast Checker.
pub fn handle_contrast(foreground: &str, background: &str) -> Result<ContrastResult, String> {
    let input = ContrastInput {
        foreground: foreground.to_string(),
        background: background.to_string(),
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| contrast::check_contrast(&input)));
    match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(format!("Contrast error: {err}")),
        Err(_) => Err("Contrast calculation not yet implemented (todo stub)".to_string()),
    }
}

/// Handler for JWT Decoder.
pub fn handle_jwt(token: &str) -> Result<JwtOutput, String> {
    let input = JwtInput {
        token: token.to_string(),
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| jwt::decode_jwt(&input)));
    match outcome {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(format!("JWT error: {err}")),
        Err(_) => Err("JWT decoding not yet implemented (todo stub)".to_string()),
    }
}
