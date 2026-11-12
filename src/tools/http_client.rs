// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Pocket HTTP Inspector & cURL Runner.
//!
//! Executes lightweight HTTP requests, measures latency, parses and formats
//! responses, and supports bidirectional conversion to/from cURL commands.

use std::collections::BTreeMap;
use std::io::Read;
use std::time::Instant;

/// HTTP Methods supported by the inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpMethod {
    #[default]
    Get = 0,
    Post = 1,
    Put = 2,
    Delete = 3,
    Patch = 4,
    Head = 5,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "PATCH" => HttpMethod::Patch,
            "HEAD" => HttpMethod::Head,
            _ => HttpMethod::Get,
        }
    }
}

/// HTTP Request description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: String,
}

impl Default for HttpRequest {
    fn default() -> Self {
        Self {
            method: HttpMethod::Get,
            url: "https://httpbin.org/get".to_string(),
            headers: BTreeMap::new(),
            body: String::new(),
        }
    }
}

/// Response output returned after executing the HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub pretty_body: String,
    /// Total time including body download.
    pub latency_ms: u64,
    /// Time until the response head arrived.
    pub ttfb_ms: u64,
    pub content_length: usize,
    pub is_success: bool,
    /// Final URL after redirects (empty when identical to the requested one).
    pub final_url: String,
    pub content_type: String,
    /// True when the body was cut for display or download limits.
    pub truncated: bool,
}

/// Bodies larger than this are not downloaded further.
const MAX_DOWNLOAD_BYTES: u64 = 8 * 1024 * 1024;
/// Text larger than this is cut before being handed to the UI (huge text fields are slow).
const MAX_DISPLAY_CHARS: usize = 200_000;

/// Parse raw headers string ("Header: Value\nOther: Val") into key-value map.
pub fn parse_headers_str(raw: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            let k = k.trim();
            if !k.is_empty() {
                map.insert(k.to_string(), v.trim().to_string());
            }
        }
    }
    map
}

/// Format map of headers back into a single text block.
pub fn format_headers_str(headers: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    for (k, v) in headers {
        out.push_str(k);
        out.push_str(": ");
        out.push_str(v);
        out.push('\n');
    }
    out
}

fn insert_header(headers: &mut BTreeMap<String, String>, raw: &str) {
    if let Some((k, v)) = raw.split_once(':') {
        let k = k.trim();
        if !k.is_empty() {
            headers.insert(k.to_string(), v.trim().to_string());
        }
    }
}

fn has_header(headers: &BTreeMap<String, String>, name: &str) -> bool {
    headers.keys().any(|k| k.eq_ignore_ascii_case(name))
}

/// Parse a raw cURL command (as copied from browser dev tools, docs or a terminal)
/// into an `HttpRequest`.
///
/// Supports quoting, `\` line continuations, `$'..'` strings, `-X`, `-H`, `-d` and its
/// variants, `--json`, `-u`, `-A`, `-e`, `-b`, `-I`, `-G`, `--url`, fused short options
/// such as `-XPOST`, and ignores transport-only flags (`-L`, `-s`, `-k`, `--compressed`, ...).
pub fn parse_curl(curl_cmd: &str) -> Result<HttpRequest, String> {
    let tokens = tokenize_command(curl_cmd.trim());
    if tokens.first().map(|t| t.as_str()) != Some("curl") {
        return Err("Input does not start with 'curl'".to_string());
    }

    let mut method: Option<HttpMethod> = None;
    let mut url = String::new();
    let mut headers = BTreeMap::new();
    let mut data: Vec<String> = Vec::new();
    let mut head_only = false;
    let mut data_as_query = false;

    // Options whose value we can safely ignore.
    const IGNORED_WITH_VALUE: &[&str] = &[
        "-o", "--output", "-m", "--max-time", "--connect-timeout", "--retry", "-w", "--write-out",
        "--proxy", "-x", "--cacert", "--cert", "--key", "-c", "--cookie-jar", "--resolve",
        "--max-redirs", "-T", "--upload-file", "--limit-rate",
    ];

    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        let next = tokens.get(i + 1).cloned();
        let take_value = |i: &mut usize| -> Result<String, String> {
            *i += 1;
            next.clone().ok_or_else(|| format!("Option '{t}' is missing its value"))
        };

        match t {
            "-X" | "--request" => method = Some(HttpMethod::from_str(&take_value(&mut i)?)),
            "-H" | "--header" => insert_header(&mut headers, &take_value(&mut i)?),
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii" | "--data-urlencode" => {
                data.push(take_value(&mut i)?)
            }
            "--json" => {
                data.push(take_value(&mut i)?);
                if !has_header(&headers, "Content-Type") {
                    headers.insert("Content-Type".into(), "application/json".into());
                }
                if !has_header(&headers, "Accept") {
                    headers.insert("Accept".into(), "application/json".into());
                }
            }
            "-u" | "--user" => {
                use base64::Engine;
                let creds = take_value(&mut i)?;
                let encoded = base64::engine::general_purpose::STANDARD.encode(creds.as_bytes());
                headers.insert("Authorization".into(), format!("Basic {encoded}"));
            }
            "-A" | "--user-agent" => {
                headers.insert("User-Agent".into(), take_value(&mut i)?);
            }
            "-e" | "--referer" => {
                headers.insert("Referer".into(), take_value(&mut i)?);
            }
            "-b" | "--cookie" => {
                headers.insert("Cookie".into(), take_value(&mut i)?);
            }
            "--url" => url = take_value(&mut i)?,
            "-I" | "--head" => head_only = true,
            "-G" | "--get" => data_as_query = true,
            _ if IGNORED_WITH_VALUE.contains(&t) => {
                take_value(&mut i)?;
            }
            _ if t.starts_with("-X") && t.len() > 2 => method = Some(HttpMethod::from_str(&t[2..])),
            _ if t.starts_with("-H") && t.len() > 2 => insert_header(&mut headers, &t[2..]),
            _ if t.starts_with("-d") && t.len() > 2 && !t.starts_with("--") => data.push(t[2..].to_string()),
            _ if t.starts_with('-') => {} // -L, -s, -k, -v, --compressed, -i, ...
            _ => {
                if url.is_empty() {
                    url = t.to_string();
                }
            }
        }
        i += 1;
    }

    if url.is_empty() {
        return Err("Could not find a valid URL in the cURL command".to_string());
    }
    if !url.contains("://") {
        url = format!("http://{url}");
    }

    let mut body = data.join("&");
    if data_as_query && !body.is_empty() {
        url.push(if url.contains('?') { '&' } else { '?' });
        url.push_str(&body);
        body.clear();
    }

    let method = if head_only {
        HttpMethod::Head
    } else {
        method.unwrap_or(if body.is_empty() { HttpMethod::Get } else { HttpMethod::Post })
    };

    Ok(HttpRequest { method, url, headers, body })
}

/// Quote a value for a POSIX shell using single quotes.
fn shell_quote(s: &str) -> String {
    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_./:=?&%+@,".contains(&b)) {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Export an `HttpRequest` into a clean, copy-paste safe cURL command.
pub fn export_to_curl(req: &HttpRequest) -> String {
    let mut parts: Vec<String> = vec!["curl".into()];
    let implicit = match req.method {
        HttpMethod::Get => req.body.is_empty(),
        HttpMethod::Post => !req.body.is_empty(),
        _ => false,
    };
    if req.method == HttpMethod::Head {
        parts.push("-I".into());
    } else if !implicit {
        parts.push(format!("-X {}", req.method.as_str()));
    }
    parts.push(shell_quote(&req.url));
    for (k, v) in &req.headers {
        parts.push(format!("-H {}", shell_quote(&format!("{k}: {v}"))));
    }
    if !req.body.is_empty() {
        parts.push(format!("--data-raw {}", shell_quote(&req.body)));
    }
    if parts.len() <= 3 {
        parts.join(" ")
    } else {
        parts.join(" \\\n  ")
    }
}

/// Execute the HTTP request (blocking: run it on a worker thread).
pub fn execute_request(req: &HttpRequest) -> Result<HttpResponse, String> {
    let url = req.url.trim();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(8))
        .timeout(std::time::Duration::from_secs(30))
        .redirects(8)
        .user_agent(concat!("DevKit/", env!("CARGO_PKG_VERSION")))
        .build();

    let start = Instant::now();
    let mut builder = agent.request(req.method.as_str(), url);
    for (k, v) in &req.headers {
        builder = builder.set(k, v);
    }

    let sends_body = !req.body.is_empty() && !matches!(req.method, HttpMethod::Get | HttpMethod::Head);
    let resp_res = if sends_body { builder.send_string(&req.body) } else { builder.call() };
    let ttfb_ms = start.elapsed().as_millis() as u64;

    let (resp, is_success) = match resp_res {
        Ok(resp) => (resp, true),
        Err(ureq::Error::Status(_, resp)) => (resp, false),
        Err(ureq::Error::Transport(t)) => return Err(describe_transport_error(&t)),
    };

    let status_code = resp.status();
    let status_text = resp.status_text().to_string();
    let headers = extract_headers(&resp);
    let final_url = if resp.get_url() != url { resp.get_url().to_string() } else { String::new() };
    let content_type = resp.header("content-type").unwrap_or("").to_string();

    let mut raw = Vec::new();
    let read_res = resp.into_reader().take(MAX_DOWNLOAD_BYTES + 1).read_to_end(&mut raw);
    let latency_ms = start.elapsed().as_millis() as u64;
    if let Err(e) = read_res {
        if raw.is_empty() {
            return Err(format!("Failed while reading the response body: {e}"));
        }
    }
    let mut truncated = raw.len() as u64 > MAX_DOWNLOAD_BYTES;
    raw.truncate(MAX_DOWNLOAD_BYTES as usize);
    let content_length = raw.len();

    let mut body = match String::from_utf8(raw) {
        Ok(text) => text,
        Err(e) => binary_preview(e.as_bytes()),
    };
    if body.len() > MAX_DISPLAY_CHARS {
        let mut cut = MAX_DISPLAY_CHARS;
        while !body.is_char_boundary(cut) {
            cut -= 1;
        }
        body.truncate(cut);
        truncated = true;
    }
    let pretty_body = format_if_json(&body);

    Ok(HttpResponse {
        status_code,
        status_text,
        headers,
        body,
        pretty_body,
        latency_ms,
        ttfb_ms,
        content_length,
        is_success,
        final_url,
        content_type,
        truncated,
    })
}

fn describe_transport_error(t: &ureq::Transport) -> String {
    use ureq::ErrorKind;
    let detail = t.message().map(|m| format!(": {m}")).unwrap_or_default();
    match t.kind() {
        ErrorKind::Dns => format!("DNS lookup failed{detail}"),
        ErrorKind::ConnectionFailed => format!("Connection failed{detail}"),
        ErrorKind::TooManyRedirects => "Too many redirects (limit 8)".to_string(),
        ErrorKind::InvalidUrl => format!("Invalid URL{detail}"),
        ErrorKind::UnknownScheme => "Unsupported URL scheme".to_string(),
        ErrorKind::Io => format!("Network I/O error{detail} (timeout or connection reset)"),
        _ => format!("Request failed: {t}"),
    }
}

/// Render non-UTF-8 content as a short hex dump.
fn binary_preview(bytes: &[u8]) -> String {
    let mut out = format!("[binary content, {} bytes — showing first 256]\n\n", bytes.len());
    for chunk in bytes.chunks(16).take(16) {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
            .collect();
        out.push_str(&format!("{:<48}  {}\n", hex.join(" "), ascii));
    }
    out
}

fn extract_headers(resp: &ureq::Response) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for name in resp.headers_names() {
        for val in resp.all(&name) {
            out.push((name.clone(), val.to_string()));
        }
    }
    out
}

fn format_if_json(text: &str) -> String {
    let trimmed = text.trim();
    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Ok(pretty) = serde_json::to_string_pretty(&v) {
                return pretty;
            }
        }
    }
    String::new()
}

/// Tokenize a shell command line: single/double quotes, `$'..'`, backslash escapes and
/// `\`+newline continuations (as produced by "Copy as cURL").
fn tokenize_command(cmd: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut has_token = false;
    let mut chars = cmd.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('\n') => {}
                Some('\r') => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                }
                Some(other) => {
                    current.push(other);
                    has_token = true;
                }
                None => {}
            },
            '\'' => {
                has_token = true;
                for q in chars.by_ref() {
                    if q == '\'' {
                        break;
                    }
                    current.push(q);
                }
            }
            '$' if chars.peek() == Some(&'\'') => {
                chars.next();
                has_token = true;
                while let Some(q) = chars.next() {
                    match q {
                        '\'' => break,
                        '\\' => match chars.next() {
                            Some('n') => current.push('\n'),
                            Some('t') => current.push('\t'),
                            Some('r') => current.push('\r'),
                            Some(o) => current.push(o),
                            None => break,
                        },
                        o => current.push(o),
                    }
                }
            }
            '"' => {
                has_token = true;
                while let Some(q) = chars.next() {
                    match q {
                        '"' => break,
                        '\\' => match chars.peek() {
                            Some(&n) if "\"\\$`\n".contains(n) => {
                                chars.next();
                                if n != '\n' {
                                    current.push(n);
                                }
                            }
                            _ => current.push('\\'),
                        },
                        o => current.push(o),
                    }
                }
            }
            c if c.is_whitespace() => {
                if has_token {
                    tokens.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            c => {
                current.push(c);
                has_token = true;
            }
        }
    }
    if has_token {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_curl_post() {
        let cmd = "curl -X POST https://api.example.com/v1/users -H \"Authorization: Bearer token123\" -H \"Content-Type: application/json\" -d '{\"username\":\"alice\"}'";
        let req = parse_curl(cmd).expect("Parse cURL");
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.url, "https://api.example.com/v1/users");
        assert_eq!(req.headers.get("Authorization").map(|s| s.as_str()), Some("Bearer token123"));
        assert_eq!(req.body, "{\"username\":\"alice\"}");
    }

    #[test]
    fn test_export_curl_roundtrip() {
        let mut headers = BTreeMap::new();
        headers.insert("Accept".to_string(), "application/json".to_string());
        let req = HttpRequest {
            method: HttpMethod::Get,
            url: "https://httpbin.org/get".to_string(),
            headers,
            body: String::new(),
        };
        let curl = export_to_curl(&req);
        assert!(curl.starts_with("curl https://httpbin.org/get"));
        assert!(curl.contains("-H 'Accept: application/json'"));
        let back = parse_curl(&curl).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn test_parse_browser_copy_as_curl() {
        let cmd = "curl 'https://api.example.com/v2/items?page=2' \\\n  -H 'accept: application/json' \\\n  -H $'x-note: it\\'s' \\\n  --compressed -L -s \\\n  --data-raw '{\"a\":1}'";
        let req = parse_curl(cmd).unwrap();
        assert_eq!(req.url, "https://api.example.com/v2/items?page=2");
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.headers.get("x-note").map(String::as_str), Some("it's"));
        assert_eq!(req.body, "{\"a\":1}");
    }

    #[test]
    fn test_parse_curl_flags() {
        let req = parse_curl("curl -XPUT -u alice:secret example.com/x --json '{}'").unwrap();
        assert_eq!(req.method, HttpMethod::Put);
        assert_eq!(req.url, "http://example.com/x");
        assert_eq!(req.headers.get("Authorization").map(String::as_str), Some("Basic YWxpY2U6c2VjcmV0"));
        assert_eq!(req.headers.get("Content-Type").map(String::as_str), Some("application/json"));
        let head = parse_curl("curl -I https://a.b").unwrap();
        assert_eq!(head.method, HttpMethod::Head);
        let get = parse_curl("curl -G https://a.b/s -d q=rust -d n=1").unwrap();
        assert_eq!(get.url, "https://a.b/s?q=rust&n=1");
        assert_eq!(get.method, HttpMethod::Get);
        assert!(parse_curl("wget x").is_err());
    }

    #[test]
    fn test_export_quotes_dangerous_chars() {
        let req = HttpRequest {
            method: HttpMethod::Post,
            url: "https://x.y/$(rm)".into(),
            headers: BTreeMap::new(),
            body: "it's `x`".into(),
        };
        let curl = export_to_curl(&req);
        assert!(curl.contains("'https://x.y/$(rm)'"));
        assert!(curl.contains("--data-raw 'it'\\''s `x`'"));
        assert_eq!(parse_curl(&curl).unwrap().body, "it's `x`");
    }
}
