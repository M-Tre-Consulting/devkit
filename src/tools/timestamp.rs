// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Timestamp Converter module.
//!
//! Provides bidirectional conversions between Unix epoch timestamps (seconds, milliseconds)
//! and human-readable date formats (ISO 8601, RFC 2822) with UTC and Local timezone support.

use chrono::{DateTime, Local, TimeZone, Utc};

/// Conversion direction mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimestampMode {
    #[default]
    EpochToHuman,
    HumanToEpoch,
}

impl TimestampMode {
    pub fn from_index(index: i32) -> Self {
        match index {
            1 => Self::HumanToEpoch,
            _ => Self::EpochToHuman,
        }
    }
}

/// Timezone display option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimezoneOption {
    #[default]
    Utc,
    Local,
}

impl TimezoneOption {
    pub fn from_str_opt(s: &str) -> Self {
        if s.eq_ignore_ascii_case("Local") {
            Self::Local
        } else {
            Self::Utc
        }
    }
}

/// Input parameters for timestamp conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampInput {
    pub input_value: String,
    pub mode: TimestampMode,
    pub timezone: TimezoneOption,
}

/// Output representations of the converted timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TimestampOutput {
    pub input_value: String,
    pub unix_seconds: String,
    pub unix_millis: String,
    pub iso_8601: String,
    pub rfc_2822: String,
    pub human_readable: String,
}

/// Errors that can occur during timestamp parsing or conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimestampError {
    InvalidFormat(String),
    OutOfBounds(String),
}

impl std::fmt::Display for TimestampError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(msg) => write!(f, "{msg}"),
            Self::OutOfBounds(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for TimestampError {}

/// Converts timestamp or datetime string across standard representations.
pub fn convert(input: &TimestampInput) -> Result<TimestampOutput, TimestampError> {
    match input.mode {
        TimestampMode::EpochToHuman => parse_epoch_string(input),
        TimestampMode::HumanToEpoch => parse_human_string(input),
    }
}

/// Returns the current timestamp in all representations for the given mode and timezone.
pub fn now(mode: TimestampMode, timezone: TimezoneOption) -> TimestampOutput {
    let dt_utc = Utc::now();
    let dt_local = Local::now();
    let unix_seconds = dt_utc.timestamp().to_string();
    let unix_millis = dt_utc.timestamp_millis().to_string();

    let rfc_2822 = match timezone {
        TimezoneOption::Utc => dt_utc.to_rfc2822(),
        TimezoneOption::Local => dt_local.to_rfc2822(),
    };
    let iso_8601 = match timezone {
        TimezoneOption::Utc => format!("{}", dt_utc.format("%+")),
        TimezoneOption::Local => format!("{}", dt_local.format("%+")),
    };
    let human_readable = match timezone {
        TimezoneOption::Utc => format!("{}", dt_utc.format("%a, %d %b %Y %H:%M:%S UTC")),
        TimezoneOption::Local => format!("{}", dt_local.format("%a, %d %b %Y %H:%M:%S %Z")),
    };

    let input_value = match mode {
        TimestampMode::EpochToHuman => unix_seconds.clone(),
        TimestampMode::HumanToEpoch => iso_8601.clone(),
    };

    TimestampOutput {
        input_value,
        unix_seconds,
        unix_millis,
        iso_8601,
        rfc_2822,
        human_readable,
    }
}

/// Parses an input timestamp string (seconds or milliseconds) to standard timestamp outputs.
fn parse_epoch_string(input: &TimestampInput) -> Result<TimestampOutput, TimestampError> {
    let trimmed = input.input_value.trim();
    if trimmed.is_empty() {
        return Err(TimestampError::InvalidFormat("Input is empty.".to_string()));
    }

    let (epoch, is_millis) = if let Ok(val) = trimmed.parse::<i64>() {
        // Numbers > 99_999_999_999 (year > 5138) are treated as milliseconds
        let is_ms = val.abs() > 99_999_999_999;
        (val, is_ms)
    } else if let Ok(val) = trimmed.parse::<f64>() {
        let is_ms = val.abs() > 99_999_999_999.0;
        (val.round() as i64, is_ms)
    } else {
        return Err(TimestampError::InvalidFormat(
            "Invalid timestamp Epoch format. Expected numeric seconds or milliseconds.".to_string(),
        ));
    };

    let (dt, unix_seconds, unix_millis) = if is_millis {
        let dt = DateTime::<Utc>::from_timestamp_millis(epoch).ok_or_else(|| {
            TimestampError::OutOfBounds("Timestamp milliseconds out of bounds.".to_string())
        })?;
        (dt, (epoch / 1000).to_string(), epoch.to_string())
    } else {
        let dt = DateTime::<Utc>::from_timestamp_secs(epoch).ok_or_else(|| {
            TimestampError::OutOfBounds("Timestamp seconds out of bounds.".to_string())
        })?;
        (dt, epoch.to_string(), (epoch as i128 * 1000).to_string())
    };

    let dt_local: DateTime<Local> = dt.with_timezone(&Local);

    let rfc_2822 = match input.timezone {
        TimezoneOption::Utc => dt.to_rfc2822(),
        TimezoneOption::Local => dt_local.to_rfc2822(),
    };
    let iso_8601 = match input.timezone {
        TimezoneOption::Utc => format!("{}", dt.format("%+")),
        TimezoneOption::Local => format!("{}", dt_local.format("%+")),
    };
    let human_readable = match input.timezone {
        TimezoneOption::Utc => format!("{}", dt.format("%a, %d %b %Y %H:%M:%S UTC")),
        TimezoneOption::Local => format!("{}", dt_local.format("%a, %d %b %Y %H:%M:%S %Z")),
    };

    Ok(TimestampOutput {
        input_value: input.input_value.clone(),
        unix_seconds,
        unix_millis,
        rfc_2822,
        iso_8601,
        human_readable,
    })
}

/// Parses a human readable date/time string into standard timestamp outputs.
fn parse_human_string(input: &TimestampInput) -> Result<TimestampOutput, TimestampError> {
    let dt_utc = parse_human_datetime(&input.input_value, input.timezone)?;
    let dt_local: DateTime<Local> = dt_utc.with_timezone(&Local);

    let unix_seconds = dt_utc.timestamp().to_string();
    let unix_millis = dt_utc.timestamp_millis().to_string();

    let rfc_2822 = match input.timezone {
        TimezoneOption::Utc => dt_utc.to_rfc2822(),
        TimezoneOption::Local => dt_local.to_rfc2822(),
    };
    let iso_8601 = match input.timezone {
        TimezoneOption::Utc => format!("{}", dt_utc.format("%+")),
        TimezoneOption::Local => format!("{}", dt_local.format("%+")),
    };
    let human_readable = match input.timezone {
        TimezoneOption::Utc => format!("{}", dt_utc.format("%a, %d %b %Y %H:%M:%S UTC")),
        TimezoneOption::Local => format!("{}", dt_local.format("%a, %d %b %Y %H:%M:%S %Z")),
    };

    Ok(TimestampOutput {
        input_value: input.input_value.clone(),
        unix_seconds,
        unix_millis,
        rfc_2822,
        iso_8601,
        human_readable,
    })
}

fn parse_human_datetime(raw: &str, tz: TimezoneOption) -> Result<DateTime<Utc>, TimestampError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(TimestampError::InvalidFormat("Input is empty.".to_string()));
    }

    // 1. Try RFC 3339 / ISO 8601 (e.g. 2023-11-14T22:13:20Z or 2023-11-14T22:13:20+00:00)
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.to_utc());
    }

    // 2. Try RFC 2822 (e.g. Tue, 14 Nov 2023 22:13:20 +0000)
    if let Ok(dt) = DateTime::parse_from_rfc2822(s) {
        return Ok(dt.to_utc());
    }

    // 3. Try common date-time formats without offset (assume selected timezone)
    let dt_formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%dT%H:%M",
        "%Y/%m/%d %H:%M:%S",
        "%Y/%m/%d %H:%M",
    ];

    for fmt in &dt_formats {
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, fmt) {
            return match tz {
                TimezoneOption::Utc => Ok(ndt.and_utc()),
                TimezoneOption::Local => match Local.from_local_datetime(&ndt) {
                    chrono::LocalResult::Single(dt) => Ok(dt.with_timezone(&Utc)),
                    chrono::LocalResult::Ambiguous(dt, _) => Ok(dt.with_timezone(&Utc)),
                    chrono::LocalResult::None => Err(TimestampError::InvalidFormat(
                        "Invalid local datetime (daylight saving time gap).".to_string(),
                    )),
                },
            };
        }
    }

    // 4. Try date-only formats (default to midnight 00:00:00)
    let date_formats = ["%Y-%m-%d", "%Y/%m/%d", "%d-%m-%Y", "%d/%m/%Y"];
    for fmt in &date_formats {
        if let Ok(nd) = chrono::NaiveDate::parse_from_str(s, fmt) {
            if let Some(ndt) = nd.and_hms_opt(0, 0, 0) {
                return match tz {
                    TimezoneOption::Utc => Ok(ndt.and_utc()),
                    TimezoneOption::Local => match Local.from_local_datetime(&ndt) {
                        chrono::LocalResult::Single(dt) => Ok(dt.with_timezone(&Utc)),
                        chrono::LocalResult::Ambiguous(dt, _) => Ok(dt.with_timezone(&Utc)),
                        chrono::LocalResult::None => Err(TimestampError::InvalidFormat(
                            "Invalid local date.".to_string(),
                        )),
                    },
                };
            }
        }
    }

    Err(TimestampError::InvalidFormat(
        "Invalid datetime format. Expected ISO 8601 (e.g. 2023-11-14T22:13:20Z), RFC 2822, or YYYY-MM-DD HH:MM:SS.".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_epoch_to_human() {
        let input = TimestampInput {
            input_value: "1700000000".to_string(),
            mode: TimestampMode::EpochToHuman,
            timezone: TimezoneOption::Utc,
        };
        let res = convert(&input).unwrap();
        assert_eq!(res.unix_seconds, "1700000000");
        assert_eq!(res.unix_millis, "1700000000000");
        assert_eq!(res.rfc_2822, "Tue, 14 Nov 2023 22:13:20 +0000");
        assert_eq!(res.iso_8601, "2023-11-14T22:13:20+00:00");
    }

    #[test]
    fn test_convert_epoch_millis() {
        let input = TimestampInput {
            input_value: "1700000000000".to_string(),
            mode: TimestampMode::EpochToHuman,
            timezone: TimezoneOption::Utc,
        };
        let res = convert(&input).unwrap();
        assert_eq!(res.unix_seconds, "1700000000");
        assert_eq!(res.unix_millis, "1700000000000");
        assert_eq!(res.iso_8601, "2023-11-14T22:13:20+00:00");
    }

    #[test]
    fn test_convert_human_to_epoch_iso8601() {
        let input = TimestampInput {
            input_value: "2023-11-14T22:13:20Z".to_string(),
            mode: TimestampMode::HumanToEpoch,
            timezone: TimezoneOption::Utc,
        };
        let res = convert(&input).unwrap();
        assert_eq!(res.unix_seconds, "1700000000");
        assert_eq!(res.unix_millis, "1700000000000");
    }

    #[test]
    fn test_convert_human_to_epoch_datetime_string() {
        let input = TimestampInput {
            input_value: "2023-11-14 22:13:20".to_string(),
            mode: TimestampMode::HumanToEpoch,
            timezone: TimezoneOption::Utc,
        };
        let res = convert(&input).unwrap();
        assert_eq!(res.unix_seconds, "1700000000");
        assert_eq!(res.unix_millis, "1700000000000");
    }

    #[test]
    fn test_convert_human_to_epoch_date_only() {
        let input = TimestampInput {
            input_value: "2023-11-14".to_string(),
            mode: TimestampMode::HumanToEpoch,
            timezone: TimezoneOption::Utc,
        };
        let res = convert(&input).unwrap();
        assert_eq!(res.unix_seconds, "1699920000");
    }

    #[test]
    fn test_convert_invalid_epoch() {
        let input = TimestampInput {
            input_value: "not_a_number".to_string(),
            mode: TimestampMode::EpochToHuman,
            timezone: TimezoneOption::Utc,
        };
        assert!(convert(&input).is_err());
    }

    #[test]
    fn test_convert_invalid_human() {
        let input = TimestampInput {
            input_value: "invalid-date-string".to_string(),
            mode: TimestampMode::HumanToEpoch,
            timezone: TimezoneOption::Utc,
        };
        assert!(convert(&input).is_err());
    }

    #[test]
    fn test_now() {
        let out = now(TimestampMode::EpochToHuman, TimezoneOption::Utc);
        assert!(!out.input_value.is_empty());
        assert_eq!(out.input_value, out.unix_seconds);
        assert!(!out.unix_seconds.is_empty());
        assert!(!out.unix_millis.is_empty());
        assert!(!out.iso_8601.is_empty());
        assert!(!out.rfc_2822.is_empty());
        assert!(!out.human_readable.is_empty());

        let out_human = now(TimestampMode::HumanToEpoch, TimezoneOption::Utc);
        assert_eq!(out_human.input_value, out_human.iso_8601);
    }
}
