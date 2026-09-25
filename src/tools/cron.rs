// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 MTRE Consulting

//! CRON Expression Parser module stub.
//!
//! Provides parsing, human-readable explanations, and upcoming run time calculations
//! for standard 5-field and 6-field CRON schedule expressions.

/// Input parameters for CRON expression parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronInput {
    pub expression: String,
    pub next_runs_count: u32,
}

/// Parsed schedule description and projected upcoming executions.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CronOutput {
    pub description: String,
    pub next_run_times: Vec<String>,
}

/// Errors that can occur during CRON syntax validation and parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CronError {
    InvalidFieldCount,
    InvalidExpression(String),
    EmptyExpression,
}

/// Parses a CRON expression and calculates human description and upcoming runs.
///
/// TODO: Implement CRON parsing logic when cron / cronjob parsing crate is added.
pub fn parse(_input: &CronInput) -> Result<CronOutput, CronError> {
    todo!("parse CRON expression")
}

/// Convenience helper to parse a CRON expression string with default next runs.
pub fn parse_cron(_expr: &str) -> Result<CronOutput, CronError> {
    todo!("parse CRON expression")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not implemented yet"]
    fn test_parse_standard_cron() {
        let input = CronInput {
            expression: "0 0 * * *".to_string(),
            next_runs_count: 5,
        };
        let res = parse(&input).unwrap();
        assert_eq!(res.description, "Every day at midnight");
        assert_eq!(res.next_run_times.len(), 5);
    }

    #[test]
    #[ignore = "not implemented yet"]
    fn test_parse_invalid_cron() {
        let input = CronInput {
            expression: "invalid cron".to_string(),
            next_runs_count: 5,
        };
        assert!(parse(&input).is_err());
    }
}
