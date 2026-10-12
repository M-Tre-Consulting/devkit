// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Code Formatter module.
//!
//! Provides syntax formatting and indentation for multiple programming and data languages:
//! JSON, YAML, XML, SQL, and TOML.

use serde::Serialize;
use serde_json::ser::PrettyFormatter;

/// Supported languages for code formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SupportedLanguage {
    #[default]
    Json,
    Yaml,
    Xml,
    Sql,
    Toml,
}

impl SupportedLanguage {
    pub fn from_index(index: i32) -> Self {
        match index {
            0 => SupportedLanguage::Json,
            1 => SupportedLanguage::Yaml,
            2 => SupportedLanguage::Xml,
            3 => SupportedLanguage::Sql,
            4 => SupportedLanguage::Toml,
            _ => SupportedLanguage::Json,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SupportedLanguage::Json => "JSON",
            SupportedLanguage::Yaml => "YAML",
            SupportedLanguage::Xml => "XML",
            SupportedLanguage::Sql => "SQL",
            SupportedLanguage::Toml => "TOML",
        }
    }
}

/// Input parameters for code formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatterInput {
    pub source_code: String,
    pub language: SupportedLanguage,
    pub indent_size: u8,
}

/// Resulting formatted code output.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormatterOutput {
    pub formatted_code: String,
}

/// Errors that can occur during syntax parsing or formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatterError {
    SyntaxError(String),
    EmptyInput,
    UnsupportedLanguage,
}

impl std::fmt::Display for FormatterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SyntaxError(msg) => write!(f, "Syntax error: {msg}"),
            Self::EmptyInput => write!(f, "Source code cannot be empty"),
            Self::UnsupportedLanguage => write!(f, "Unsupported language"),
        }
    }
}

impl std::error::Error for FormatterError {}

/// Beautifies and indents source code based on language grammar.
pub fn format(input: &FormatterInput) -> Result<FormatterOutput, FormatterError> {
    if input.source_code.trim().is_empty() {
        return Err(FormatterError::EmptyInput);
    }

    let indent_size = if input.indent_size == 0 { 2 } else { input.indent_size };

    let formatted = match input.language {
        SupportedLanguage::Json => format_json(&input.source_code, indent_size)?,
        SupportedLanguage::Yaml => format_yaml(&input.source_code)?,
        SupportedLanguage::Xml => format_xml(&input.source_code, indent_size)?,
        SupportedLanguage::Sql => format_sql(&input.source_code, indent_size)?,
        SupportedLanguage::Toml => format_toml(&input.source_code, indent_size)?,
    };

    Ok(FormatterOutput {
        formatted_code: formatted,
    })
}

/// Convenience helper for simple formatting.
pub fn format_code(input: &str, language: SupportedLanguage) -> Result<String, FormatterError> {
    format(&FormatterInput {
        source_code: input.to_string(),
        language,
        indent_size: 2,
    })
    .map(|out| out.formatted_code)
}

fn format_json(source: &str, indent_size: u8) -> Result<String, FormatterError> {
    let parsed: serde_json::Value =
        serde_json::from_str(source).map_err(|e| FormatterError::SyntaxError(e.to_string()))?;

    let indent = " ".repeat(indent_size as usize);
    let formatter = PrettyFormatter::with_indent(indent.as_bytes());

    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    parsed
        .serialize(&mut ser)
        .map_err(|e| FormatterError::SyntaxError(e.to_string()))?;

    String::from_utf8(buf).map_err(|e| FormatterError::SyntaxError(e.to_string()))
}

fn format_yaml(source: &str) -> Result<String, FormatterError> {
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(source).map_err(|e| FormatterError::SyntaxError(e.to_string()))?;

    let res = serde_yaml::to_string(&parsed)
        .map_err(|e| FormatterError::SyntaxError(e.to_string()))?;

    Ok(res.trim().to_string())
}

fn format_xml(source: &str, indent_size: u8) -> Result<String, FormatterError> {
    let mut tokens: Vec<XmlToken> = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '<' {
            let start = i;
            let mut in_quote = None;
            while i < len {
                let c = chars[i];
                if let Some(q) = in_quote {
                    if c == q {
                        in_quote = None;
                    }
                } else if c == '"' || c == '\'' {
                    in_quote = Some(c);
                } else if c == '>' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            if in_quote.is_some() || i <= start || chars[i - 1] != '>' {
                return Err(FormatterError::SyntaxError("Unclosed XML tag".to_string()));
            }
            let raw_tag: String = chars[start..i].iter().collect();
            let tag_content = raw_tag.trim();

            if tag_content.starts_with("<?") || tag_content.starts_with("<!") {
                tokens.push(XmlToken::Declaration(tag_content.to_string()));
            } else if tag_content.starts_with("</") {
                let name = tag_content[2..tag_content.len() - 1].trim().to_string();
                tokens.push(XmlToken::CloseTag(name));
            } else if tag_content.ends_with("/>") {
                tokens.push(XmlToken::SelfClosing(tag_content.to_string()));
            } else {
                let inner = &tag_content[1..tag_content.len() - 1];
                let tag_name = inner.split_whitespace().next().unwrap_or("").to_string();
                tokens.push(XmlToken::OpenTag {
                    name: tag_name,
                    full: tag_content.to_string(),
                });
            }
        } else {
            let start = i;
            while i < len && chars[i] != '<' {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                tokens.push(XmlToken::Text(trimmed.to_string()));
            }
        }
    }

    if tokens.is_empty() {
        return Err(FormatterError::EmptyInput);
    }

    let mut result = Vec::new();
    let mut depth: usize = 0;
    let indent_str = " ".repeat(indent_size as usize);

    let mut idx = 0;
    while idx < tokens.len() {
        match &tokens[idx] {
            XmlToken::Declaration(decl) => {
                result.push(format!("{}{}", indent_str.repeat(depth), decl));
                idx += 1;
            }
            XmlToken::SelfClosing(tag) => {
                result.push(format!("{}{}", indent_str.repeat(depth), tag));
                idx += 1;
            }
            XmlToken::OpenTag { name, full } => {
                // Check if next token is Text and token after is matching CloseTag
                if idx + 2 < tokens.len() {
                    if let (XmlToken::Text(txt), XmlToken::CloseTag(close_name)) =
                        (&tokens[idx + 1], &tokens[idx + 2])
                    {
                        if close_name == name {
                            result.push(format!(
                                "{}{}{}</{}>",
                                indent_str.repeat(depth),
                                full,
                                txt,
                                close_name
                            ));
                            idx += 3;
                            continue;
                        }
                    }
                }
                result.push(format!("{}{}", indent_str.repeat(depth), full));
                depth += 1;
                idx += 1;
            }
            XmlToken::CloseTag(name) => {
                depth = depth.saturating_sub(1);
                result.push(format!("{}</{}>", indent_str.repeat(depth), name));
                idx += 1;
            }
            XmlToken::Text(txt) => {
                result.push(format!("{}{}", indent_str.repeat(depth), txt));
                idx += 1;
            }
        }
    }

    Ok(result.join("\n"))
}

enum XmlToken {
    Declaration(String),
    OpenTag { name: String, full: String },
    CloseTag(String),
    SelfClosing(String),
    Text(String),
}

fn format_sql(source: &str, indent_size: u8) -> Result<String, FormatterError> {
    let mut formatted = String::new();
    let indent_str = " ".repeat(indent_size as usize);

    // List of major keywords that start a new line at base indent
    let major_keywords = [
        "SELECT", "FROM", "WHERE", "GROUP BY", "ORDER BY", "HAVING", "LIMIT",
        "OFFSET", "INSERT INTO", "VALUES", "UPDATE", "SET", "DELETE FROM",
        "LEFT JOIN", "RIGHT JOIN", "INNER JOIN", "FULL JOIN", "CROSS JOIN", "JOIN",
        "UNION ALL", "UNION", "CREATE TABLE", "DROP TABLE", "ALTER TABLE",
    ];

    let words: Vec<&str> = source.split_whitespace().collect();
    if words.is_empty() {
        return Err(FormatterError::EmptyInput);
    }

    let mut i = 0;
    let mut current_line = String::new();

    while i < words.len() {
        let w = words[i];
        let w_upper = w.to_ascii_uppercase();

        // Check 2-word combinations like "GROUP BY", "ORDER BY", "INSERT INTO", etc.
        let two_word = if i + 1 < words.len() {
            format!("{} {}", w_upper, words[i + 1].to_ascii_uppercase())
        } else {
            String::new()
        };

        if major_keywords.contains(&two_word.as_str()) {
            if !current_line.is_empty() {
                formatted.push_str(current_line.trim_end());
                formatted.push('\n');
                current_line.clear();
            }
            current_line.push_str(&two_word);
            current_line.push(' ');
            i += 2;
            continue;
        }

        if major_keywords.contains(&w_upper.as_str()) {
            if !current_line.is_empty() {
                formatted.push_str(current_line.trim_end());
                formatted.push('\n');
                current_line.clear();
            }
            current_line.push_str(&w_upper);
            current_line.push(' ');
            i += 1;
            continue;
        }

        // Check sub-keywords that should start indented new lines
        if w_upper == "AND" || w_upper == "OR" || w_upper == "ON" {
            if !current_line.is_empty() {
                formatted.push_str(current_line.trim_end());
                formatted.push('\n');
                current_line.clear();
                current_line.push_str(&indent_str);
            }
            current_line.push_str(&w_upper);
            current_line.push(' ');
            i += 1;
            continue;
        }

        // Standardize common SQL keyword casing
        let formatted_word = match w_upper.as_str() {
            "AS" | "IN" | "IS" | "NULL" | "NOT" | "LIKE" | "BETWEEN" | "EXISTS" | "ASC"
            | "DESC" | "DISTINCT" | "CASE" | "WHEN" | "THEN" | "ELSE" | "END" | "BY" => {
                w_upper.to_string()
            }
            _ => w.to_string(),
        };

        current_line.push_str(&formatted_word);
        current_line.push(' ');
        i += 1;
    }

    if !current_line.is_empty() {
        formatted.push_str(current_line.trim_end());
    }

    Ok(formatted)
}

fn format_toml(source: &str, _indent_size: u8) -> Result<String, FormatterError> {
    let mut formatted_lines = Vec::new();

    for raw_line in source.lines() {
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            if !formatted_lines.is_empty() && formatted_lines.last() != Some(&String::new()) {
                formatted_lines.push(String::new());
            }
            continue;
        }

        // Preserve comments
        if trimmed.starts_with('#') {
            formatted_lines.push(trimmed.to_string());
            continue;
        }

        // Table header e.g. [table] or [[array]]
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Add a blank line before new table header if previous wasn't blank
            if !formatted_lines.is_empty() && formatted_lines.last() != Some(&String::new()) {
                formatted_lines.push(String::new());
            }
            formatted_lines.push(trimmed.to_string());
            continue;
        }

        // Key = Value
        if let Some((key, val)) = trimmed.split_once('=') {
            let k_trim = key.trim();
            let v_trim = val.trim();
            formatted_lines.push(format!("{k_trim} = {v_trim}"));
        } else {
            formatted_lines.push(trimmed.to_string());
        }
    }

    if formatted_lines.is_empty() {
        return Err(FormatterError::EmptyInput);
    }

    Ok(formatted_lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_json() {
        let input = FormatterInput {
            source_code: r#"{"a":1,"b":2}"#.to_string(),
            language: SupportedLanguage::Json,
            indent_size: 2,
        };
        let res = format(&input).unwrap();
        assert!(res.formatted_code.contains('\n'));
        assert!(res.formatted_code.contains("  \"a\": 1"));
    }

    #[test]
    fn test_format_empty() {
        let input = FormatterInput {
            source_code: "".to_string(),
            language: SupportedLanguage::Json,
            indent_size: 2,
        };
        assert_eq!(format(&input), Err(FormatterError::EmptyInput));
    }

    #[test]
    fn test_format_yaml() {
        let input = FormatterInput {
            source_code: "name: DevKit\nstatus: ready".to_string(),
            language: SupportedLanguage::Yaml,
            indent_size: 2,
        };
        let res = format(&input).unwrap();
        assert!(res.formatted_code.contains("name: DevKit"));
    }

    #[test]
    fn test_format_xml() {
        let input = FormatterInput {
            source_code: "<root><user id=\"1\"><name>John</name></user></root>".to_string(),
            language: SupportedLanguage::Xml,
            indent_size: 2,
        };
        let res = format(&input).unwrap();
        assert!(res.formatted_code.contains("<root>"));
        assert!(res.formatted_code.contains("  <user id=\"1\">"));
        assert!(res.formatted_code.contains("    <name>John</name>"));
    }

    #[test]
    fn test_format_sql() {
        let input = FormatterInput {
            source_code: "select id, name from users where active = 1 and age > 18 order by id desc"
                .to_string(),
            language: SupportedLanguage::Sql,
            indent_size: 2,
        };
        let res = format(&input).unwrap();
        assert!(res.formatted_code.contains("SELECT id, name"));
        assert!(res.formatted_code.contains("FROM users"));
        assert!(res.formatted_code.contains("WHERE active = 1"));
        assert!(res.formatted_code.contains("ORDER BY id DESC"));
    }

    #[test]
    fn test_format_toml() {
        let input = FormatterInput {
            source_code: "[package]\nname=\"devkit\"\nversion=\"0.2.1\"\n\n[dependencies]\nserde=\"1.0\""
                .to_string(),
            language: SupportedLanguage::Toml,
            indent_size: 2,
        };
        let res = format(&input).unwrap();
        assert!(res.formatted_code.contains("[package]"));
        assert!(res.formatted_code.contains("name = \"devkit\""));
        assert!(res.formatted_code.contains("[dependencies]"));
    }
}
