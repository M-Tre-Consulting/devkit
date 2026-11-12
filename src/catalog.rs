// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Single source of truth for the tool list shown on Home, Recent and Favorites.
//!
//! The IDs are persisted (recents / favorites), so never renumber existing entries.

/// Static description of one tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolDef {
    pub id: i32,
    pub title: &'static str,
    pub description: &'static str,
    /// 0 = primary, 1 = secondary, 2 = tertiary accent.
    pub accent: i32,
    /// Extra search terms (lowercase).
    pub keywords: &'static str,
}

pub const TOOLS: &[ToolDef] = &[
    ToolDef { id: 1, title: "Subnet Calculator", description: "IPv4 CIDR, netmask & range", accent: 0, keywords: "ip ipv4 cidr network mask netmask broadcast lan" },
    ToolDef { id: 2, title: "Hash Calculator", description: "MD5, SHA-256 checksums", accent: 1, keywords: "md5 sha sha1 sha256 sha512 blake checksum digest" },
    ToolDef { id: 3, title: "Base64", description: "Encode & decode strings", accent: 2, keywords: "base64 encode decode url-safe" },
    ToolDef { id: 4, title: "UUID Generator", description: "Generate unique identifiers", accent: 0, keywords: "uuid guid v4 v7 v1 id random" },
    ToolDef { id: 5, title: "Timestamp", description: "Epoch & date conversions", accent: 1, keywords: "time date epoch unix iso 8601 rfc 2822 timezone" },
    ToolDef { id: 6, title: "Regex Tester", description: "Evaluate pattern matches", accent: 2, keywords: "regex regexp pattern match capture" },
    ToolDef { id: 7, title: "JSON ↔ YAML", description: "Convert between formats", accent: 0, keywords: "json yaml yml convert config" },
    ToolDef { id: 8, title: "CRON Parser", description: "Parse schedule syntax", accent: 1, keywords: "cron crontab schedule job" },
    ToolDef { id: 9, title: "GZip", description: "Compress & decompress", accent: 2, keywords: "gzip zip compress deflate decompress" },
    ToolDef { id: 10, title: "Code Formatter", description: "Format code syntax", accent: 0, keywords: "format pretty beautify json xml sql html css minify" },
    ToolDef { id: 11, title: "Chmod Calculator", description: "Octal & symbolic bits", accent: 1, keywords: "chmod permission unix octal rwx" },
    ToolDef { id: 12, title: "Color Converter", description: "HEX, RGB, HSL values", accent: 2, keywords: "color colour hex rgb hsl hsv cmyk" },
    ToolDef { id: 13, title: "Contrast Checker", description: "WCAG contrast ratio", accent: 0, keywords: "contrast wcag accessibility a11y aa aaa" },
    ToolDef { id: 14, title: "JWT Decoder", description: "Decode headers & claims", accent: 1, keywords: "jwt token bearer oauth claims" },
    ToolDef { id: 15, title: "Diff Studio", description: "Visual diff & code patches", accent: 2, keywords: "diff compare patch text" },
    ToolDef { id: 16, title: "QR Code Studio", description: "Wi-Fi, URLs & live TOTP 2FA", accent: 0, keywords: "qr code wifi totp 2fa otp" },
    ToolDef { id: 17, title: "HTTP Client", description: "Pocket cURL & inspector", accent: 1, keywords: "http https curl rest api request get post" },
    ToolDef { id: 18, title: "LAN Scout", description: "Hosts, ports & dev services", accent: 2, keywords: "lan port scan network host discovery service" },
    ToolDef { id: 19, title: "TLS Inspector", description: "Certificates, expiry & cipher", accent: 0, keywords: "tls ssl https certificate cert x509 expiry cipher san" },
];

/// Highest valid tool ID.
pub const MAX_TOOL_ID: i32 = 19;

pub fn find(id: i32) -> Option<&'static ToolDef> {
    TOOLS.iter().find(|t| t.id == id)
}

/// Case-insensitive search across title, description and keywords.
/// Every whitespace-separated term must match somewhere.
pub fn search(query: &str) -> Vec<&'static ToolDef> {
    let terms: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();
    if terms.is_empty() {
        return TOOLS.iter().collect();
    }
    let mut scored: Vec<(i32, &'static ToolDef)> = TOOLS
        .iter()
        .filter_map(|t| {
            let title = t.title.to_lowercase();
            let hay = format!("{} {} {}", title, t.description.to_lowercase(), t.keywords);
            if !terms.iter().all(|term| hay.contains(term.as_str())) {
                return None;
            }
            // Title prefix matches rank first, then title matches, then the rest.
            let score = if title.starts_with(terms[0].as_str()) {
                0
            } else if title.contains(terms[0].as_str()) {
                1
            } else {
                2
            };
            Some((score, t))
        })
        .collect();
    scored.sort_by_key(|(score, t)| (*score, t.id));
    scored.into_iter().map(|(_, t)| t).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_contiguous() {
        for (i, t) in TOOLS.iter().enumerate() {
            assert_eq!(t.id, i as i32 + 1);
        }
        assert_eq!(TOOLS.len() as i32, MAX_TOOL_ID);
    }

    #[test]
    fn search_matches_keywords_and_ranks_titles() {
        let r = search("sha256");
        assert_eq!(r[0].id, 2);
        let r = search("cert");
        assert_eq!(r[0].id, 19);
        let r = search("j");
        assert_eq!(r[0].id, 7); // "JSON ↔ YAML" title prefix before "JWT" (id order)
        assert!(search("zzzz-nothing").is_empty());
        assert_eq!(search("  ").len(), TOOLS.len());
    }
}
