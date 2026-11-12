// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! CRON expression parser.
//!
//! Supports standard 5-field expressions (`min hour day-of-month month day-of-week`),
//! 6-field expressions with a leading seconds field, `@yearly`-style macros, lists,
//! ranges, steps (`*/15`, `10-40/5`, `5/10`), month / weekday names, `?`, and `7` as
//! Sunday. When both day-of-month and day-of-week are restricted, a day matches if
//! *either* matches (Vixie cron semantics). Upcoming runs are computed in local time.

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

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

impl std::fmt::Display for CronError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CronError::InvalidFieldCount => {
                write!(f, "Expected 5 fields (min hour day month weekday) or 6 with seconds first")
            }
            CronError::InvalidExpression(m) => write!(f, "{m}"),
            CronError::EmptyExpression => write!(f, "Enter a CRON expression, e.g. */15 * * * *"),
        }
    }
}

/// One parsed field: the allowed values plus whether it was a wildcard.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    values: Vec<u32>,
    any: bool,
    raw: String,
}

impl Field {
    fn contains(&self, v: u32) -> bool {
        self.values.binary_search(&v).is_ok()
    }
}

/// A fully parsed schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    seconds: Field,
    minutes: Field,
    hours: Field,
    dom: Field,
    months: Field,
    dow: Field,
    has_seconds: bool,
}

const MONTHS: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
const DAYS: [&str; 7] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September", "October",
    "November", "December",
];
const DAY_NAMES: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

fn parse_value(token: &str, min: u32, max: u32, names: Option<&[&str]>, what: &str) -> Result<u32, CronError> {
    let upper = token.to_ascii_uppercase();
    if let Some(names) = names {
        if let Some(i) = names.iter().position(|n| *n == upper) {
            return Ok(i as u32 + min);
        }
    }
    let v: u32 = token
        .parse()
        .map_err(|_| CronError::InvalidExpression(format!("'{token}' is not a valid {what}")))?;
    if v < min || v > max {
        return Err(CronError::InvalidExpression(format!("{what} {v} is out of range {min}-{max}")));
    }
    Ok(v)
}

fn parse_field(raw: &str, min: u32, max: u32, names: Option<&[&str]>, what: &str) -> Result<Field, CronError> {
    let any = raw == "*" || raw == "?";
    let mut values = Vec::new();
    for part in raw.split(',') {
        if part.is_empty() {
            return Err(CronError::InvalidExpression(format!("Empty list item in {what} field '{raw}'")));
        }
        let (range, step) = match part.split_once('/') {
            Some((r, s)) => {
                let step: u32 = s
                    .parse()
                    .ok()
                    .filter(|s| *s > 0)
                    .ok_or_else(|| CronError::InvalidExpression(format!("Invalid step '{s}' in {what} field")))?;
                (r, step)
            }
            None => (part, 1),
        };
        let (lo, hi) = if range == "*" || range == "?" {
            (min, max)
        } else if let Some((a, b)) = range.split_once('-') {
            let a = parse_value(a, min, max, names, what)?;
            let b = parse_value(b, min, max, names, what)?;
            if a > b {
                return Err(CronError::InvalidExpression(format!("Range {range} in {what} field is reversed")));
            }
            (a, b)
        } else {
            let a = parse_value(range, min, max, names, what)?;
            // "5/10" means "from 5 to the max, every 10".
            (a, if part.contains('/') { max } else { a })
        };
        let mut v = lo;
        while v <= hi {
            values.push(v);
            v += step;
        }
    }
    values.sort_unstable();
    values.dedup();
    Ok(Field { values, any, raw: raw.to_string() })
}

/// Parse an expression (5 or 6 fields, or a macro) into a [`Schedule`].
pub fn parse_schedule(expr: &str) -> Result<Schedule, CronError> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Err(CronError::EmptyExpression);
    }
    let expanded = match expr.to_ascii_lowercase().as_str() {
        "@yearly" | "@annually" => "0 0 1 1 *".to_string(),
        "@monthly" => "0 0 1 * *".to_string(),
        "@weekly" => "0 0 * * 0".to_string(),
        "@daily" | "@midnight" => "0 0 * * *".to_string(),
        "@hourly" => "0 * * * *".to_string(),
        "@reboot" => {
            return Err(CronError::InvalidExpression("@reboot runs once at startup and has no schedule".into()))
        }
        s if s.starts_with('@') => return Err(CronError::InvalidExpression(format!("Unknown macro '{expr}'"))),
        _ => expr.to_string(),
    };
    let fields: Vec<&str> = expanded.split_whitespace().collect();
    let (sec, rest) = match fields.len() {
        5 => ("0", &fields[..]),
        6 => (fields[0], &fields[1..]),
        _ => return Err(CronError::InvalidFieldCount),
    };
    let mut dow = parse_field(rest[4], 0, 7, Some(&DAYS), "weekday")?;
    // 7 is an alias for Sunday.
    if dow.values.contains(&7) {
        dow.values.retain(|v| *v != 7);
        if !dow.values.contains(&0) {
            dow.values.insert(0, 0);
        }
    }
    Ok(Schedule {
        seconds: parse_field(sec, 0, 59, None, "second")?,
        minutes: parse_field(rest[0], 0, 59, None, "minute")?,
        hours: parse_field(rest[1], 0, 23, None, "hour")?,
        dom: parse_field(rest[2], 1, 31, None, "day of month")?,
        months: parse_field(rest[3], 1, 12, Some(&MONTHS), "month")?,
        dow,
        has_seconds: fields.len() == 6,
    })
}

impl Schedule {
    fn day_matches(&self, d: NaiveDate) -> bool {
        if !self.months.contains(d.month()) {
            return false;
        }
        let dom_ok = self.dom.contains(d.day());
        let dow_ok = self.dow.contains(d.weekday().num_days_from_sunday());
        match (self.dom.any, self.dow.any) {
            (true, true) => true,
            (true, false) => dow_ok,
            (false, true) => dom_ok,
            (false, false) => dom_ok || dow_ok,
        }
    }

    /// Next `count` run times strictly after `after` (searches up to ~5 years ahead).
    pub fn upcoming(&self, after: NaiveDateTime, count: usize) -> Vec<NaiveDateTime> {
        let mut out = Vec::new();
        let mut day = after.date();
        let end = day + Duration::days(366 * 5);
        while day <= end && out.len() < count {
            if self.day_matches(day) {
                'day: for &h in &self.hours.values {
                    for &m in &self.minutes.values {
                        for &s in &self.seconds.values {
                            let Some(t) = NaiveTime::from_hms_opt(h, m, s) else { continue };
                            let dt = day.and_time(t);
                            if dt > after {
                                out.push(dt);
                                if out.len() >= count {
                                    break 'day;
                                }
                            }
                        }
                    }
                }
            }
            day += Duration::days(1);
        }
        out
    }

    /// Plain-English description.
    pub fn describe(&self) -> String {
        let time = self.describe_time();
        let days = self.describe_days();
        let mut s = time;
        if !days.is_empty() {
            s.push_str(", ");
            s.push_str(&days);
        }
        s
    }

    fn describe_time(&self) -> String {
        let single = |f: &Field| (f.values.len() == 1).then(|| f.values[0]);
        let secs = if self.has_seconds && self.seconds.raw != "0" {
            Some(describe_unit(&self.seconds, "second", 60))
        } else {
            None
        };
        let base = match (single(&self.hours), single(&self.minutes)) {
            (Some(0), Some(0)) if secs.is_none() && self.dom.any && self.dow.any && self.months.any => {
                return "Every day at midnight".into()
            }
            (Some(h), Some(m)) => format!("At {h:02}:{m:02}"),
            (None, Some(m)) if self.hours.any => {
                if m == 0 {
                    "Every hour, on the hour".into()
                } else {
                    format!("At minute {m} past every hour")
                }
            }
            (None, Some(m)) => format!("At minute {m} past {}", describe_unit(&self.hours, "hour", 24)),
            (Some(h), None) => format!("{} during hour {h:02}", capitalize(&describe_unit(&self.minutes, "minute", 60))),
            (None, None) => {
                let mins = capitalize(&describe_unit(&self.minutes, "minute", 60));
                if self.hours.any {
                    mins
                } else {
                    format!("{mins}, {}", describe_unit(&self.hours, "hour", 24))
                }
            }
        };
        match secs {
            Some(s) => format!("{}, {base}", capitalize(&s)).replace(", Every minute", "").replace("Every minute", "every minute"),
            None => base,
        }
    }

    fn describe_days(&self) -> String {
        let mut parts = Vec::new();
        let dow = |v: u32| DAY_NAMES[v as usize % 7].to_string();
        if !self.dom.any {
            parts.push(format!("on day {} of the month", list_or_range(&self.dom.values, |v| v.to_string())));
        }
        if !self.dow.any {
            let joiner = if self.dom.any { "" } else { "or " };
            parts.push(format!("{joiner}on {}", list_or_range(&self.dow.values, dow)));
        }
        if !self.months.any {
            parts.push(format!("in {}", list_or_range(&self.months.values, |v| MONTH_NAMES[v as usize - 1].to_string())));
        }
        parts.join(" ")
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// "every minute", "every 15 minutes", "minutes 5, 10", "minutes 0 through 30".
fn describe_unit(f: &Field, unit: &str, span: u32) -> String {
    if f.any {
        return format!("every {unit}");
    }
    let v = &f.values;
    if v.len() > 1 {
        let step = v[1] - v[0];
        let even = v.windows(2).all(|w| w[1] - w[0] == step);
        if even && v[0] < step && v[v.len() - 1] + step >= span {
            return format!("every {step} {unit}s");
        }
    }
    format!("{unit}s {}", list_or_range(v, |x| x.to_string()))
}

fn list_or_range(values: &[u32], name: impl Fn(u32) -> String) -> String {
    let contiguous = values.len() > 2 && values.windows(2).all(|w| w[1] == w[0] + 1);
    if contiguous {
        return format!("{} through {}", name(values[0]), name(values[values.len() - 1]));
    }
    let names: Vec<String> = values.iter().map(|v| name(*v)).collect();
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
    }
}

fn format_run(dt: NaiveDateTime, now: NaiveDateTime, with_seconds: bool) -> String {
    let delta = dt - now;
    let rel = if delta.num_days() >= 1 {
        format!("in {}d {}h", delta.num_days(), delta.num_hours() % 24)
    } else if delta.num_hours() >= 1 {
        format!("in {}h {}m", delta.num_hours(), delta.num_minutes() % 60)
    } else if delta.num_minutes() >= 1 {
        format!("in {}m", delta.num_minutes())
    } else {
        format!("in {}s", delta.num_seconds().max(1))
    };
    let fmt = if with_seconds { "%a %Y-%m-%d %H:%M:%S" } else { "%a %Y-%m-%d %H:%M" };
    format!("{}  ·  {rel}", dt.format(fmt))
}

/// Parses a CRON expression and calculates human description and upcoming runs.
pub fn parse(input: &CronInput) -> Result<CronOutput, CronError> {
    let schedule = parse_schedule(&input.expression)?;
    let now = Local::now().naive_local();
    let count = input.next_runs_count.clamp(1, 50) as usize;
    let runs = schedule.upcoming(now, count);
    if runs.is_empty() {
        return Err(CronError::InvalidExpression("This schedule never fires (e.g. February 30th)".into()));
    }
    // Skip local times that do not exist (DST gaps) only for display correctness.
    let next_run_times = runs
        .into_iter()
        .filter(|dt| Local.from_local_datetime(dt).earliest().is_some())
        .map(|dt| format_run(dt, now, schedule.has_seconds))
        .collect();
    Ok(CronOutput { description: schedule.describe(), next_run_times })
}

/// Convenience helper to parse a CRON expression string with default next runs.
pub fn parse_cron(expr: &str) -> Result<CronOutput, CronError> {
    parse(&CronInput { expression: expr.to_string(), next_runs_count: 5 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn test_parse_standard_cron() {
        let input = CronInput { expression: "0 0 * * *".to_string(), next_runs_count: 5 };
        let res = parse(&input).unwrap();
        assert_eq!(res.description, "Every day at midnight");
        assert_eq!(res.next_run_times.len(), 5);
    }

    #[test]
    fn test_parse_invalid_cron() {
        let input = CronInput { expression: "invalid cron".to_string(), next_runs_count: 5 };
        assert!(parse(&input).is_err());
        assert_eq!(parse_schedule("").unwrap_err(), CronError::EmptyExpression);
        assert_eq!(parse_schedule("* * *").unwrap_err(), CronError::InvalidFieldCount);
        assert!(parse_schedule("61 * * * *").is_err());
        assert!(parse_schedule("*/0 * * * *").is_err());
        assert!(parse_schedule("30-10 * * * *").is_err());
    }

    #[test]
    fn computes_next_runs() {
        let s = parse_schedule("*/15 * * * *").unwrap();
        let runs = s.upcoming(at("2026-01-01 10:07:00"), 3);
        assert_eq!(runs, vec![at("2026-01-01 10:15:00"), at("2026-01-01 10:30:00"), at("2026-01-01 10:45:00")]);

        // Weekdays at 09:30. 2026-01-02 is a Friday.
        let s = parse_schedule("30 9 * * MON-FRI").unwrap();
        let runs = s.upcoming(at("2026-01-02 10:00:00"), 2);
        assert_eq!(runs, vec![at("2026-01-05 09:30:00"), at("2026-01-06 09:30:00")]);

        // Feb 29th only exists in leap years.
        let s = parse_schedule("0 12 29 2 *").unwrap();
        assert_eq!(s.upcoming(at("2026-01-01 00:00:00"), 1), vec![at("2028-02-29 12:00:00")]);

        // 6 fields: seconds first.
        let s = parse_schedule("*/20 * * * * *").unwrap();
        assert_eq!(s.upcoming(at("2026-01-01 00:00:05"), 2), vec![at("2026-01-01 00:00:20"), at("2026-01-01 00:00:40")]);
    }

    #[test]
    fn dom_and_dow_are_ored() {
        // 1st of the month OR any Monday. 2026-06-01 is a Monday, 2026-06-08 too.
        let s = parse_schedule("0 0 1 * 1").unwrap();
        let runs = s.upcoming(at("2026-06-02 00:00:00"), 2);
        assert_eq!(runs, vec![at("2026-06-08 00:00:00"), at("2026-06-15 00:00:00")]);
    }

    #[test]
    fn names_macros_and_sunday_alias() {
        assert_eq!(parse_schedule("@daily").unwrap(), parse_schedule("0 0 * * *").unwrap());
        assert_eq!(parse_schedule("0 0 * * 7").unwrap().dow.values, vec![0]);
        assert_eq!(parse_schedule("0 0 * JAN,dec *").unwrap().months.values, vec![1, 12]);
        assert!(parse_schedule("@reboot").is_err());
    }

    #[test]
    fn describes_common_schedules() {
        assert_eq!(parse_schedule("*/15 * * * *").unwrap().describe(), "Every 15 minutes");
        assert_eq!(parse_schedule("30 9 * * 1-5").unwrap().describe(), "At 09:30, on Monday through Friday");
        assert_eq!(parse_schedule("0 * * * *").unwrap().describe(), "Every hour, on the hour");
        assert_eq!(parse_schedule("5 * * * *").unwrap().describe(), "At minute 5 past every hour");
        assert_eq!(parse_schedule("0 0 1 1 *").unwrap().describe(), "At 00:00, on day 1 of the month in January");
        assert_eq!(parse_schedule("* * * * *").unwrap().describe(), "Every minute");
    }
}
