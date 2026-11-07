// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Visual text and code diff engine using Myers difference algorithm.
//!
//! Provides line-by-line comparison with additions, deletions, unchanged lines,
//! optional whitespace ignoring, and unified patch generation.

use similar::{ChangeTag, TextDiff};

/// Type of change for an individual diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffChangeTag {
    Equal,
    Delete,
    Insert,
}

/// A single rendered line within a diff view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    /// Equal, Delete, or Insert.
    pub tag: DiffChangeTag,
    /// 1-based line number in the original text (None if inserted).
    pub old_line_num: Option<usize>,
    /// 1-based line number in the modified text (None if deleted).
    pub new_line_num: Option<usize>,
    /// Content of the line without trailing newline.
    pub text: String,
}

/// Aggregated diff calculation output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffResult {
    /// Detailed list of lines with tags and numbering.
    pub lines: Vec<DiffLine>,
    /// Total count of added lines.
    pub additions: usize,
    /// Total count of removed lines.
    pub deletions: usize,
    /// Total count of unchanged lines.
    pub unchanged: usize,
    /// Formatted standard unified diff patch string.
    pub unified_patch: String,
}

/// Compute differences between original (`old_text`) and modified (`new_text`).
pub fn compute_diff(old_text: &str, new_text: &str, ignore_whitespace: bool) -> DiffResult {
    let mut clean_old = if ignore_whitespace {
        strip_line_trailing_whitespace(old_text)
    } else {
        old_text.to_string()
    };
    let mut clean_new = if ignore_whitespace {
        strip_line_trailing_whitespace(new_text)
    } else {
        new_text.to_string()
    };

    // Normalize final newline to avoid false mismatch on the last line
    if !clean_old.is_empty() && !clean_old.ends_with('\n') {
        clean_old.push('\n');
    }
    if !clean_new.is_empty() && !clean_new.ends_with('\n') {
        clean_new.push('\n');
    }

    let text_diff = TextDiff::from_lines(&clean_old, &clean_new);
    let mut lines = Vec::new();
    let mut additions = 0;
    let mut deletions = 0;
    let mut unchanged = 0;

    let mut old_idx = 1;
    let mut new_idx = 1;

    for change in text_diff.iter_all_changes() {
        let tag = match change.tag() {
            ChangeTag::Equal => {
                unchanged += 1;
                DiffChangeTag::Equal
            }
            ChangeTag::Delete => {
                deletions += 1;
                DiffChangeTag::Delete
            }
            ChangeTag::Insert => {
                additions += 1;
                DiffChangeTag::Insert
            }
        };

        let (old_num, new_num) = match tag {
            DiffChangeTag::Equal => {
                let nums = (Some(old_idx), Some(new_idx));
                old_idx += 1;
                new_idx += 1;
                nums
            }
            DiffChangeTag::Delete => {
                let nums = (Some(old_idx), None);
                old_idx += 1;
                nums
            }
            DiffChangeTag::Insert => {
                let nums = (None, Some(new_idx));
                new_idx += 1;
                nums
            }
        };

        let raw_val = change.value();
        let trimmed_line = raw_val.strip_suffix("\r\n")
            .or_else(|| raw_val.strip_suffix('\n'))
            .unwrap_or(raw_val)
            .to_string();

        lines.push(DiffLine {
            tag,
            old_line_num: old_num,
            new_line_num: new_num,
            text: trimmed_line,
        });
    }

    let unified_patch = text_diff
        .unified_diff()
        .context_radius(3)
        .header("original", "modified")
        .to_string();

    DiffResult {
        lines,
        additions,
        deletions,
        unchanged,
        unified_patch,
    }
}

/// Strip trailing whitespace from each line to allow whitespace-insensitive comparison.
fn strip_line_trailing_whitespace(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for line in input.split('\n') {
        let clean = line.trim_end();
        out.push_str(clean);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_text() {
        let text = "alpha\nbeta\ngamma";
        let res = compute_diff(text, text, false);
        assert_eq!(res.additions, 0);
        assert_eq!(res.deletions, 0);
        assert_eq!(res.unchanged, 3);
        assert_eq!(res.lines.len(), 3);
        assert!(res.lines.iter().all(|l| l.tag == DiffChangeTag::Equal));
    }

    #[test]
    fn test_simple_addition_and_deletion() {
        let old = "apple\nbanana\ncherry";
        let new = "apple\nblueberry\ncherry\ndate";
        let res = compute_diff(old, new, false);
        assert_eq!(res.deletions, 1); // banana
        assert_eq!(res.additions, 2); // blueberry, date
        assert_eq!(res.unchanged, 2); // apple, cherry
    }

    #[test]
    fn test_ignore_whitespace() {
        let old = "foo   \nbar\n";
        let new = "foo\nbar  \n";
        let res = compute_diff(old, new, true);
        assert_eq!(res.deletions, 0);
        assert_eq!(res.additions, 0);
        assert_eq!(res.unchanged, 3); // 2 lines + empty line
    }
}
