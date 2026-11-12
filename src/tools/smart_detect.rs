// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Smart Paste: recognise what a piece of text is and route it to the right tool.
//!
//! Detection is purely local and heuristic. Each detector returns a confidence score;
//! the caller shows suggestions sorted by score so the most likely tool comes first.

use base64::Engine;

/// One routing suggestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub tool_id: i32,
    /// Short label of what was recognised, e.g. "JWT token".
    pub kind: &'static str,
    /// Text to prefill in the target tool (may be normalised from the input).
    pub payload: String,
    /// 0..=100
    pub score: u8,
}

const MAX_SUGGESTIONS: usize = 4;

/// Analyse `input` and return up to four suggestions, best first.
pub fn detect(input: &str) -> Vec<Suggestion> {
    let text = input.trim();
    if text.is_empty() || text.len() > 512 * 1024 {
        return Vec::new();
    }

    let mut out: Vec<Suggestion> = Vec::new();
    let mut push = |tool_id: i32, kind: &'static str, payload: &str, score: u8| {
        if !out.iter().any(|s| s.tool_id == tool_id) {
            out.push(Suggestion { tool_id, kind, payload: payload.to_string(), score });
        }
    };

    let single_line = !text.contains('\n');

    if text.starts_with("curl ") || text.starts_with("curl\t") {
        push(17, "cURL command", text, 98);
    }
    if is_jwt(text) {
        push(14, "JWT token", text, 97);
    }
    if text.starts_with("otpauth://") {
        push(16, "OTP auth URI", text, 90);
    }
    if single_line && (text.starts_with("http://") || text.starts_with("https://")) {
        push(17, "URL", text, 92);
        if text.starts_with("https://") {
            if let Some(host) = url_host(text) {
                push(19, "HTTPS endpoint", &host, 70);
            }
        }
        push(16, "Link", text, 40);
    }
    if is_cidr_or_ipv4(text) {
        let payload = if text.contains('/') { text.to_string() } else { format!("{text}/24") };
        push(1, "IPv4 address", &payload, 93);
        if !text.contains('/') {
            push(18, "Host", text, 75);
        }
    }
    if let Some(kind) = timestamp_kind(text) {
        push(5, kind, text, 91);
    }
    if is_chmod_octal(text) {
        push(11, "Permission bits", text, 88);
    }
    if is_cron(text) {
        push(8, "Cron expression", text, 90);
    }
    if is_color(text) {
        push(12, "Color", text, 92);
        push(13, "Color", text, 45);
    }
    if is_uuid(text) {
        push(2, "UUID", text, 50);
    }
    if is_hostname(text) {
        push(19, "Domain", text, 72);
        push(18, "Host", text, 60);
    }
    if looks_like_json(text) {
        push(7, "JSON document", text, 89);
        push(10, "JSON document", text, 60);
    } else if looks_like_yaml(text) {
        push(7, "YAML document", text, 72);
    }
    if text.starts_with("---") || text.starts_with("diff --git") || text.starts_with("@@ ") {
        push(15, "Patch", text, 70);
    }
    if is_base64_text(text) {
        push(3, "Base64 data", text, 80);
    }
    if text.starts_with('<') && text.ends_with('>') {
        push(10, "Markup", text, 70);
    }

    // Generic fallbacks for any text: hash it or turn it into a QR code.
    push(2, "Text", text, 20);
    if text.len() <= 1200 {
        push(16, "Text", text, 15);
    }

    out.sort_by(|a, b| b.score.cmp(&a.score));
    out.truncate(MAX_SUGGESTIONS);
    out
}

fn is_b64url_segment(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'=')
}

fn is_jwt(text: &str) -> bool {
    let token = text.strip_prefix("Bearer ").unwrap_or(text).trim();
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 || !parts[..2].iter().all(|p| is_b64url_segment(p)) {
        return false;
    }
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[0].trim_end_matches('='))
        .ok()
        .and_then(|h| serde_json::from_slice::<serde_json::Value>(&h).ok())
        .map(|v| v.get("alg").is_some() || v.get("typ").is_some())
        .unwrap_or(false)
}

fn url_host(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host_port = authority.rsplit('@').next()?;
    if host_port.is_empty() {
        None
    } else {
        Some(host_port.to_string())
    }
}

fn is_ipv4(s: &str) -> bool {
    s.parse::<std::net::Ipv4Addr>().is_ok() && s.split('.').count() == 4
}

fn is_cidr_or_ipv4(text: &str) -> bool {
    match text.split_once('/') {
        Some((ip, prefix)) => is_ipv4(ip) && prefix.parse::<u8>().map(|p| p <= 32).unwrap_or(false),
        None => is_ipv4(text),
    }
}

fn timestamp_kind(text: &str) -> Option<&'static str> {
    if text.bytes().all(|b| b.is_ascii_digit()) {
        let n: u64 = text.parse().ok()?;
        // 2001-09-09 .. 2286-11-20 in seconds or milliseconds.
        return match text.len() {
            10 if n >= 1_000_000_000 => Some("Unix time (s)"),
            13 if n >= 1_000_000_000_000 => Some("Unix time (ms)"),
            _ => None,
        };
    }
    if chrono::DateTime::parse_from_rfc3339(text).is_ok() || chrono::DateTime::parse_from_rfc2822(text).is_ok() {
        return Some("Date & time");
    }
    None
}

fn is_chmod_octal(text: &str) -> bool {
    (text.len() == 3 || (text.len() == 4 && text.starts_with('0')))
        && text.bytes().all(|b| (b'0'..=b'7').contains(&b))
        && text != "000"
}

fn is_cron(text: &str) -> bool {
    if text.starts_with('@') {
        return matches!(text, "@yearly" | "@annually" | "@monthly" | "@weekly" | "@daily" | "@midnight" | "@hourly");
    }
    let fields: Vec<&str> = text.split_whitespace().collect();
    if !(5..=6).contains(&fields.len()) {
        return false;
    }
    let ok_field = |f: &str| {
        f.chars().all(|c| c.is_ascii_alphanumeric() || "*/,-?LW#".contains(c))
            && f.chars().any(|c| c.is_ascii_digit() || c == '*' || c == '?')
    };
    fields.iter().all(|f| ok_field(f)) && fields.iter().any(|f| f.contains('*') || f.contains('/'))
}

fn is_color(text: &str) -> bool {
    let t = text.to_ascii_lowercase();
    if let Some(hex) = t.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit());
    }
    ["rgb(", "rgba(", "hsl(", "hsla(", "hsv("].iter().any(|p| t.starts_with(p)) && t.ends_with(')')
}

fn is_uuid(text: &str) -> bool {
    uuid::Uuid::parse_str(text).is_ok() && text.len() >= 32
}

fn is_hostname(text: &str) -> bool {
    if text.len() > 253 || !text.contains('.') || text.contains(char::is_whitespace) || is_ipv4(text) {
        return false;
    }
    let host = text.split(':').next().unwrap_or(text);
    let labels: Vec<&str> = host.split('.').collect();
    let tld = labels.last().copied().unwrap_or("");
    tld.len() >= 2
        && tld.bytes().all(|b| b.is_ascii_alphabetic())
        && labels.iter().all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

fn looks_like_json(text: &str) -> bool {
    let starts = text.starts_with('{') || text.starts_with('[');
    starts && serde_json::from_str::<serde_json::Value>(text).is_ok()
}

fn looks_like_yaml(text: &str) -> bool {
    if !text.contains('\n') {
        return false;
    }
    let keyed = text
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !t.is_empty() && !t.starts_with('#')
        })
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("- ") || t.split_once(": ").map(|(k, _)| !k.contains(' ')).unwrap_or(false) || t.ends_with(':')
        })
        .count();
    keyed >= 2 && serde_yaml::from_str::<serde_yaml::Value>(text).map(|v| v.is_mapping() || v.is_sequence()).unwrap_or(false)
}

fn is_base64_text(text: &str) -> bool {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.len() < 8 || compact.len() % 4 != 0 {
        return false;
    }
    let std_alphabet = compact.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=');
    let url_alphabet = compact.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'=');
    if !(std_alphabet || url_alphabet) {
        return false;
    }
    // Plain words like "password" are valid base64 too; require a mix of cases or digits,
    // and that decoding yields mostly printable text or a known binary signature.
    let has_upper = compact.bytes().any(|b| b.is_ascii_uppercase());
    let has_lower = compact.bytes().any(|b| b.is_ascii_lowercase());
    let has_digit_or_sym = compact.bytes().any(|b| b.is_ascii_digit() || b"+/=-_".contains(&b));
    if !(has_upper && has_lower && has_digit_or_sym) {
        return false;
    }
    let engine = if std_alphabet {
        &base64::engine::general_purpose::STANDARD
    } else {
        &base64::engine::general_purpose::URL_SAFE
    };
    match engine.decode(&compact) {
        Ok(bytes) => {
            if bytes.starts_with(&[0x1f, 0x8b]) || bytes.starts_with(b"\x89PNG") || bytes.starts_with(b"%PDF") {
                return true;
            }
            let printable = bytes
                .iter()
                .filter(|b| b.is_ascii_graphic() || b.is_ascii_whitespace())
                .count();
            !bytes.is_empty() && printable * 100 / bytes.len() >= 85
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top(input: &str) -> i32 {
        detect(input).first().map(|s| s.tool_id).unwrap_or(-1)
    }

    #[test]
    fn routes_common_developer_inputs() {
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        assert_eq!(top(jwt), 14);
        assert_eq!(top(&format!("Bearer {jwt}")), 14);
        assert_eq!(top("curl -X POST https://api.example.com -d '{}'"), 17);
        assert_eq!(top("https://example.com/path?q=1"), 17);
        assert_eq!(top("10.0.0.0/8"), 1);
        assert_eq!(top("192.168.1.20"), 1);
        assert_eq!(top("1700000000"), 5);
        assert_eq!(top("1700000000123"), 5);
        assert_eq!(top("2024-05-01T10:00:00Z"), 5);
        assert_eq!(top("*/15 * * * *"), 8);
        assert_eq!(top("@daily"), 8);
        assert_eq!(top("#6750A4"), 12);
        assert_eq!(top("rgb(10, 20, 30)"), 12);
        assert_eq!(top("755"), 11);
        assert_eq!(top("{\"a\": 1}"), 7);
        assert_eq!(top("name: devkit\nversion: 1\n"), 7);
        assert_eq!(top("SGVsbG8sIERldktpdCE="), 3);
        assert_eq!(top("github.com"), 19);
        assert_eq!(top("otpauth://totp/DevKit:me?secret=JBSWY3DPEHPK3PXP"), 16);
    }

    #[test]
    fn plain_text_falls_back_to_generic_tools() {
        let s = detect("hello world");
        assert_eq!(s[0].tool_id, 2);
        assert!(detect("   ").is_empty());
        // Ordinary words are not mistaken for base64.
        assert!(!detect("password").iter().any(|s| s.tool_id == 3));
    }

    #[test]
    fn https_url_suggests_tls_with_host_payload() {
        let s = detect("https://example.org:8443/x");
        let tls = s.iter().find(|s| s.tool_id == 19).expect("tls suggestion");
        assert_eq!(tls.payload, "example.org:8443");
    }

    #[test]
    fn suggestions_are_unique_and_capped() {
        let s = detect("#ffffff");
        let mut ids: Vec<i32> = s.iter().map(|s| s.tool_id).collect();
        ids.dedup();
        assert_eq!(ids.len(), s.len());
        assert!(s.len() <= MAX_SUGGESTIONS);
    }
}
