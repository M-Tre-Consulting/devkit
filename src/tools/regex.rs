// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Regex Tester module.
//!
//! Provides regular expression testing, syntax validation, match evaluation,
//! and capture group extraction with case-insensitive, multiline, and dot-matches-newline flags.

use regex::RegexBuilder;

/// Regex compilation flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RegexFlags {
    pub case_insensitive: bool,
    pub multiline: bool,
    pub dot_matches_newline: bool,
}

/// Input parameters for regex testing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexInput {
    pub pattern: String,
    pub test_string: String,
    pub flags: RegexFlags,
}

/// Individual regex match occurrence with span and captures.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RegexMatch {
    pub index: usize,
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub captures: Vec<String>,
}

/// Aggregated output from evaluating regex matches.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RegexOutput {
    pub total_matches: usize,
    pub matches: Vec<RegexMatch>,
}

/// Errors that can occur during regex parsing and evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegexError {
    InvalidPattern(String),
    ExecutionError(String),
}

/// Evaluates regex pattern against test text using specified flags.
pub fn evaluate(input: &RegexInput) -> Result<RegexOutput, RegexError> {
    if input.pattern.is_empty() {
        return Err(RegexError::InvalidPattern("Regex pattern cannot be empty".to_string()));
    }

    let re = RegexBuilder::new(&input.pattern)
        .case_insensitive(input.flags.case_insensitive)
        .multi_line(input.flags.multiline)
        .dot_matches_new_line(input.flags.dot_matches_newline)
        .build()
        .map_err(|err| RegexError::InvalidPattern(err.to_string()))?;

    let mut matches = Vec::new();
    for (i, caps) in re.captures_iter(&input.test_string).enumerate() {
        if let Some(m) = caps.get(0) {
            let capture_groups: Vec<String> = caps
                .iter()
                .skip(1)
                .map(|c| c.map_or("".to_string(), |group| group.as_str().to_string()))
                .collect();

            matches.push(RegexMatch {
                index: i + 1,
                start: m.start(),
                end: m.end(),
                text: m.as_str().to_string(),
                captures: capture_groups,
            });
        }
    }

    let total_matches = matches.len();
    Ok(RegexOutput {
        total_matches,
        matches,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_simple_match() {
        let input = RegexInput {
            pattern: r"\d+".to_string(),
            test_string: "Item 42 and 99".to_string(),
            flags: RegexFlags::default(),
        };
        let res = evaluate(&input).unwrap();
        assert_eq!(res.total_matches, 2);
        assert_eq!(res.matches[0].text, "42");
        assert_eq!(res.matches[0].start, 5);
        assert_eq!(res.matches[0].end, 7);
        assert_eq!(res.matches[0].index, 1);
        assert_eq!(res.matches[1].text, "99");
        assert_eq!(res.matches[1].start, 12);
        assert_eq!(res.matches[1].end, 14);
        assert_eq!(res.matches[1].index, 2);
    }

    #[test]
    fn test_evaluate_invalid_pattern() {
        let input = RegexInput {
            pattern: r"([a-z+".to_string(),
            test_string: "test".to_string(),
            flags: RegexFlags::default(),
        };
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn test_evaluate_empty_pattern() {
        let input = RegexInput {
            pattern: "".to_string(),
            test_string: "test".to_string(),
            flags: RegexFlags::default(),
        };
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn test_evaluate_case_insensitive() {
        let input = RegexInput {
            pattern: "hello".to_string(),
            test_string: "Hello HELLO hello".to_string(),
            flags: RegexFlags {
                case_insensitive: true,
                multiline: false,
                dot_matches_newline: false,
            },
        };
        let res = evaluate(&input).unwrap();
        assert_eq!(res.total_matches, 3);
        assert_eq!(res.matches[0].text, "Hello");
        assert_eq!(res.matches[1].text, "HELLO");
        assert_eq!(res.matches[2].text, "hello");
    }

    #[test]
    fn test_evaluate_multiline() {
        let input = RegexInput {
            pattern: "^line".to_string(),
            test_string: "line 1\nline 2\nother".to_string(),
            flags: RegexFlags {
                case_insensitive: false,
                multiline: true,
                dot_matches_newline: false,
            },
        };
        let res = evaluate(&input).unwrap();
        assert_eq!(res.total_matches, 2);
    }

    #[test]
    fn test_evaluate_dot_matches_newline() {
        let input_without = RegexInput {
            pattern: "a.*b".to_string(),
            test_string: "a\nb".to_string(),
            flags: RegexFlags {
                case_insensitive: false,
                multiline: false,
                dot_matches_newline: false,
            },
        };
        assert_eq!(evaluate(&input_without).unwrap().total_matches, 0);

        let input_with = RegexInput {
            pattern: "a.*b".to_string(),
            test_string: "a\nb".to_string(),
            flags: RegexFlags {
                case_insensitive: false,
                multiline: false,
                dot_matches_newline: true,
            },
        };
        assert_eq!(evaluate(&input_with).unwrap().total_matches, 1);
    }

    #[test]
    fn test_evaluate_capture_groups() {
        let input = RegexInput {
            pattern: r"(\w+):\s*(\d+)".to_string(),
            test_string: "age: 25, year: 2026".to_string(),
            flags: RegexFlags::default(),
        };
        let res = evaluate(&input).unwrap();
        assert_eq!(res.total_matches, 2);
        assert_eq!(res.matches[0].captures, vec!["age".to_string(), "25".to_string()]);
        assert_eq!(res.matches[1].captures, vec!["year".to_string(), "2026".to_string()]);
    }

    #[test]
    fn test_evaluate_no_matches() {
        let input = RegexInput {
            pattern: "xyz".to_string(),
            test_string: "hello world".to_string(),
            flags: RegexFlags::default(),
        };
        let res = evaluate(&input).unwrap();
        assert_eq!(res.total_matches, 0);
        assert!(res.matches.is_empty());
    }
}
