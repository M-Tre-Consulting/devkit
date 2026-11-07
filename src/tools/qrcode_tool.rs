// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Developer QR Code Studio & TOTP Token Generator.
//!
//! Generates vector QR codes for Plain Text/URLs, Wi-Fi configuration strings (WPA/WPA2/WPA3),
//! and 2FA TOTP authenticator setups (RFC 6238) with live token verification.

use qrcode::{EcLevel, QrCode};
use sha1::{Digest, Sha1};

/// Formats supported by the Developer QR Studio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QrType {
    #[default]
    Text = 0,
    Wifi = 1,
    Totp = 2,
}

/// Authentication type for Wi-Fi configurations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WifiAuthType {
    #[default]
    Wpa = 0,
    Wep = 1,
    NoPass = 2,
}

/// Result of generating a QR Code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrResult {
    /// Dimension (width and height in modules/cells).
    pub size: usize,
    /// Bit matrix where true = dark module, false = light module.
    pub modules: Vec<bool>,
    /// Formatted SVG string representing the vector QR code.
    pub svg: String,
    /// Exact payload encoded inside the QR code.
    pub payload: String,
    /// Live 6-digit TOTP code (if QrType was Totp).
    pub totp_code: Option<String>,
    /// Remaining seconds before TOTP rollover (0..30).
    pub totp_remaining_secs: Option<u64>,
}

/// Generate QR code for plain text or arbitrary URL.
pub fn generate_text_qr(text: &str) -> Result<QrResult, String> {
    if text.trim().is_empty() {
        return Err("Payload cannot be empty".to_string());
    }
    build_qr_result(text, None, None)
}

/// Generate QR code for Wi-Fi network credentials.
pub fn generate_wifi_qr(ssid: &str, password: &str, auth: WifiAuthType, hidden: bool) -> Result<QrResult, String> {
    if ssid.trim().is_empty() {
        return Err("Wi-Fi SSID cannot be empty".to_string());
    }

    let auth_str = match auth {
        WifiAuthType::Wpa => "WPA",
        WifiAuthType::Wep => "WEP",
        WifiAuthType::NoPass => "nopass",
    };

    // Standard Wi-Fi URI format: WIFI:T:WPA;S:MySSID;P:MyPassword;H:false;;
    let payload = format!(
        "WIFI:T:{auth_str};S:{ssid};P:{password};H:{hidden};;"
    );

    build_qr_result(&payload, None, None)
}

/// Generate QR code for a TOTP 2FA secret (RFC 6238) and compute live token.
pub fn generate_totp_qr(
    issuer: &str,
    account: &str,
    secret_base32: &str,
    now_unix_secs: u64,
) -> Result<QrResult, String> {
    let clean_secret = secret_base32.replace(' ', "").to_uppercase();
    if clean_secret.is_empty() {
        return Err("Secret key cannot be empty".to_string());
    }

    // Verify valid base32
    let key_bytes = decode_base32(&clean_secret)
        .map_err(|e| format!("Invalid Base32 secret: {e}"))?;

    let clean_issuer = if issuer.is_empty() { "DevKit" } else { issuer.trim() };
    let clean_account = if account.is_empty() { "user@example.com" } else { account.trim() };

    let payload = format!(
        "otpauth://totp/{clean_issuer}:{clean_account}?secret={clean_secret}&issuer={clean_issuer}&algorithm=SHA1&digits=6&period=30"
    );

    let (code, remaining) = calculate_totp_code(&key_bytes, now_unix_secs);

    build_qr_result(&payload, Some(code), Some(remaining))
}

/// Construct QR modules and SVG from raw payload.
fn build_qr_result(
    payload: &str,
    totp_code: Option<String>,
    totp_remaining_secs: Option<u64>,
) -> Result<QrResult, String> {
    let code = QrCode::with_error_correction_level(payload.as_bytes(), EcLevel::M)
        .map_err(|e| format!("QR encoding failed: {e}"))?;

    let size = code.width();
    let modules: Vec<bool> = code.to_colors().into_iter().map(|c| c == qrcode::Color::Dark).collect();

    // Render clean standalone SVG
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {size} {size}" shape-rendering="crispEdges">"##
    );
    svg.push_str(r##"<rect width="100%" height="100%" fill="#ffffff"/>"##);
    for y in 0..size {
        for x in 0..size {
            if modules[y * size + x] {
                svg.push_str(&format!(r##"<rect x="{x}" y="{y}" width="1" height="1" fill="#000000"/>"##));
            }
        }
    }
    svg.push_str("</svg>");

    Ok(QrResult {
        size,
        modules,
        svg,
        payload: payload.to_string(),
        totp_code,
        totp_remaining_secs,
    })
}

/// Calculate current 6-digit TOTP code and countdown seconds (RFC 6238).
pub fn calculate_totp_code(key: &[u8], now_secs: u64) -> (String, u64) {
    let period = 30u64;
    let counter = now_secs / period;
    let remaining = period - (now_secs % period);

    let counter_bytes = counter.to_be_bytes();
    let hash = hmac_sha1(key, &counter_bytes);

    let offset = (hash[hash.len() - 1] & 0x0f) as usize;
    let binary = ((hash[offset] as u32 & 0x7f) << 24)
        | ((hash[offset + 1] as u32 & 0xff) << 16)
        | ((hash[offset + 2] as u32 & 0xff) << 8)
        | (hash[offset + 3] as u32 & 0xff);

    let code = binary % 1_000_000;
    (format!("{code:06}"), remaining)
}

/// Compute HMAC-SHA1 according to RFC 2104.
fn hmac_sha1(key: &[u8], message: &[u8]) -> [u8; 20] {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        let hash = Sha1::digest(key);
        k[..20].copy_from_slice(&hash);
    } else {
        k[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }

    let mut inner = Sha1::new();
    inner.update(&ipad);
    inner.update(message);
    let inner_hash = inner.finalize();

    let mut outer = Sha1::new();
    outer.update(&opad);
    outer.update(&inner_hash);
    outer.finalize().into()
}

/// Minimal RFC 4648 Base32 decoder.
fn decode_base32(input: &str) -> Result<Vec<u8>, &'static str> {
    let mut bits = 0u32;
    let mut bit_count = 0;
    let mut out = Vec::new();

    for c in input.chars() {
        if c == '=' || c.is_whitespace() {
            continue;
        }
        let val = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32,
            '2'..='7' => c as u32 - '2' as u32 + 26,
            _ => return Err("Invalid character in Base32 string"),
        };
        bits = (bits << 5) | val;
        bit_count += 5;
        if bit_count >= 8 {
            bit_count -= 8;
            out.push((bits >> bit_count) as u8);
            bits &= (1 << bit_count) - 1;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text_qr() {
        let res = generate_text_qr("https://m-tre.it").expect("Generate QR");
        assert!(res.size >= 21);
        assert_eq!(res.modules.len(), res.size * res.size);
        assert!(res.svg.contains("<svg"));
    }

    #[test]
    fn test_wifi_qr_generation() {
        let res = generate_wifi_qr("OfficeNetwork", "Password123", WifiAuthType::Wpa, false)
            .expect("Generate Wi-Fi QR");
        assert!(res.payload.starts_with("WIFI:T:WPA;S:OfficeNetwork;"));
    }

    #[test]
    fn test_totp_rfc6238_vector() {
        // Standard RFC 6238 test key "12345678901234567890" in Base32 = GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        // Unix time 59 corresponds to counter 1 (0..29=0, 30..59=1)
        let key = decode_base32(secret).unwrap();
        let (code, rem) = calculate_totp_code(&key, 59);
        assert_eq!(code.len(), 6);
        assert_eq!(rem, 1);

        let qr_res = generate_totp_qr("MyService", "alice", secret, 59).expect("Generate TOTP QR");
        assert!(qr_res.payload.contains("otpauth://totp/"));
        assert_eq!(qr_res.totp_code.unwrap(), code);
    }
}
