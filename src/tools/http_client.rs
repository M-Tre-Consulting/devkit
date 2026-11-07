// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Pocket HTTP Inspector & cURL Runner.
//!
//! Executes lightweight HTTP requests, measures latency, parses and formats
//! responses, and supports bidirectional conversion to/from cURL commands.

use std::collections::BTreeMap;
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
    pub latency_ms: u64,
    pub content_length: usize,
    pub is_success: bool,
}

/// Parse raw headers string ("Header: Value\nOther: Val") into key-value map.
pub fn parse_headers_str(raw: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            map.insert(k.trim().to_string(), v.trim().to_string());
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

/// Parse a raw cURL command into an `HttpRequest`.
pub fn parse_curl(curl_cmd: &str) -> Result<HttpRequest, String> {
    let clean = curl_cmd.trim();
    if !clean.starts_with("curl") {
        return Err("Input does not start with 'curl'".to_string());
    }

    let mut method = HttpMethod::Get;
    let mut url = String::new();
    let mut headers = BTreeMap::new();
    let mut body = String::new();

    // Simple robust tokenizer that respects quotes
    let tokens = tokenize_command(clean);
    let mut i = 1; // skip "curl"
    while i < tokens.len() {
        let t = &tokens[i];
        if t == "-X" || t == "--request" {
            if i + 1 < tokens.len() {
                method = HttpMethod::from_str(&tokens[i + 1]);
                i += 2;
                continue;
            }
        } else if t == "-H" || t == "--header" {
            if i + 1 < tokens.len() {
                let h = &tokens[i + 1];
                if let Some((k, v)) = h.split_once(':') {
                    headers.insert(k.trim().to_string(), v.trim().to_string());
                }
                i += 2;
                continue;
            }
        } else if t == "-d" || t == "--data" || t == "--data-raw" {
            if i + 1 < tokens.len() {
                body = tokens[i + 1].clone();
                if method == HttpMethod::Get {
                    method = HttpMethod::Post;
                }
                i += 2;
                continue;
            }
        } else if t.starts_with("http://") || t.starts_with("https://") {
            url = t.clone();
        } else if !t.starts_with('-') && url.is_empty() {
            url = t.clone();
        }
        i += 1;
    }

    if url.is_empty() {
        return Err("Could not find a valid URL in the cURL command".to_string());
    }

    Ok(HttpRequest {
        method,
        url,
        headers,
        body,
    })
}

/// Export an `HttpRequest` into a clean copyable cURL command.
pub fn export_to_curl(req: &HttpRequest) -> String {
    let mut cmd = format!("curl -X {} \"{}\"", req.method.as_str(), req.url);
    for (k, v) in &req.headers {
        cmd.push_str(&format!(" -H \"{k}: {v}\""));
    }
    if !req.body.is_empty() {
        let escaped = req.body.replace('\"', "\\\"");
        cmd.push_str(&format!(" -d \"{escaped}\""));
    }
    cmd
}

/// Execute the HTTP request synchronously and measure latency.
pub fn execute_request(req: &HttpRequest) -> Result<HttpResponse, String> {
    if !req.url.starts_with("http://") && !req.url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(12))
        .build();

    let start = Instant::now();
    let mut builder = agent.request(req.method.as_str(), &req.url);

    for (k, v) in &req.headers {
        builder = builder.set(k, v);
    }

    let resp_res = if !req.body.is_empty()
        && matches!(req.method, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch)
    {
        builder.send_string(&req.body)
    } else {
        builder.call()
    };

    let latency_ms = start.elapsed().as_millis() as u64;

    match resp_res {
        Ok(resp) => {
            let status_code = resp.status();
            let status_text = resp.status_text().to_string();
            let headers = extract_headers(&resp);
            let body = resp.into_string().unwrap_or_default();
            let content_length = body.len();
            let pretty_body = format_if_json(&body);

            Ok(HttpResponse {
                status_code,
                status_text,
                headers,
                body,
                pretty_body,
                latency_ms,
                content_length,
                is_success: true,
            })
        }
        Err(ureq::Error::Status(code, resp)) => {
            let status_text = resp.status_text().to_string();
            let headers = extract_headers(&resp);
            let body = resp.into_string().unwrap_or_default();
            let content_length = body.len();
            let pretty_body = format_if_json(&body);

            Ok(HttpResponse {
                status_code: code,
                status_text,
                headers,
                body,
                pretty_body,
                latency_ms,
                content_length,
                is_success: false,
            })
        }
        Err(e) => Err(format!("Network request failed: {e}")),
    }
}

fn extract_headers(resp: &ureq::Response) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for name in resp.headers_names() {
        if let Some(val) = resp.header(&name) {
            out.push((name, val.to_string()));
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
    trimmed.to_string()
}

/// Tokenize command line string considering single and double quotes.
fn tokenize_command(cmd: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quote = None;

    for c in cmd.chars() {
        match in_quote {
            Some(q) if c == q => {
                in_quote = None;
            }
            Some(_) => {
                current.push(c);
            }
            None => {
                if c == '\'' || c == '\"' {
                    in_quote = Some(c);
                } else if c.is_whitespace() {
                    if !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                    }
                } else {
                    current.push(c);
                }
            }
        }
    }
    if !current.is_empty() {
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
        assert!(curl.contains("curl -X GET \"https://httpbin.org/get\""));
        assert!(curl.contains("-H \"Accept: application/json\""));
    }
}
