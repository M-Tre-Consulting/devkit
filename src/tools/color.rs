// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Color Converter module.
//!
//! Provides color parsing and conversions between HEX, RGB, HSL, HSV, and CMYK formats.

/// Supported color input formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorFormat {
    #[default]
    Auto,
    Hex,
    Rgb,
    Hsl,
    Hsv,
}

impl ColorFormat {
    pub fn from_index(index: i32) -> Self {
        match index {
            0 => ColorFormat::Auto,
            1 => ColorFormat::Hex,
            2 => ColorFormat::Rgb,
            3 => ColorFormat::Hsl,
            4 => ColorFormat::Hsv,
            _ => ColorFormat::Auto,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ColorFormat::Auto => "Auto",
            ColorFormat::Hex => "HEX",
            ColorFormat::Rgb => "RGB",
            ColorFormat::Hsl => "HSL",
            ColorFormat::Hsv => "HSV",
        }
    }
}

/// Input parameters for color conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorInput {
    pub value: String,
    pub format: ColorFormat,
}

/// Converted representations across standard color spaces.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ColorOutput {
    pub hex: String,
    pub rgb: String,
    pub hsl: String,
    pub hsv: String,
    pub cmyk: String,
}

/// Errors that can occur during color parsing and conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorError {
    InvalidColorFormat(String),
    EmptyInput,
    ValueOutOfRange(String),
}

impl std::fmt::Display for ColorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidColorFormat(val) => write!(f, "Invalid color format or value: '{val}'"),
            Self::EmptyInput => write!(f, "Color input cannot be empty"),
            Self::ValueOutOfRange(val) => write!(f, "Color component out of range: '{val}'"),
        }
    }
}

impl std::error::Error for ColorError {}

/// Convert a color string in the given format to all supported color representations.
pub fn convert_color(input: &ColorInput) -> Result<ColorOutput, ColorError> {
    let (r, g, b) = parse_to_rgb(&input.value, input.format)?;
    Ok(rgb_to_output(r, g, b))
}

/// Parses an arbitrary color string with auto-detection.
pub fn parse_color(input: &str) -> Result<ColorOutput, ColorError> {
    convert_color(&ColorInput {
        value: input.to_string(),
        format: ColorFormat::Auto,
    })
}

/// Parse an input string in the specified or auto-detected format into standard (R, G, B) u8 values.
pub fn parse_to_rgb(value: &str, format: ColorFormat) -> Result<(u8, u8, u8), ColorError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ColorError::EmptyInput);
    }

    match format {
        ColorFormat::Hex => parse_hex(trimmed),
        ColorFormat::Rgb => parse_rgb(trimmed),
        ColorFormat::Hsl => parse_hsl(trimmed),
        ColorFormat::Hsv => parse_hsv(trimmed),
        ColorFormat::Auto => parse_auto(trimmed),
    }
}

fn parse_auto(s: &str) -> Result<(u8, u8, u8), ColorError> {
    let lower = s.to_ascii_lowercase();
    if lower.starts_with('#') {
        return parse_hex(s);
    }
    if lower.starts_with("rgb") {
        return parse_rgb(s);
    }
    if lower.starts_with("hsl") {
        return parse_hsl(s);
    }
    if lower.starts_with("hsv") {
        return parse_hsv(s);
    }

    // Attempt hex without leading # if string is 3, 6, or 8 valid hex chars
    if (s.len() == 3 || s.len() == 6 || s.len() == 8) && s.chars().all(|c| c.is_ascii_hexdigit()) {
        if let Ok(rgb) = parse_hex(s) {
            return Ok(rgb);
        }
    }

    // Try comma-separated RGB
    if let Ok(rgb) = parse_rgb(s) {
        return Ok(rgb);
    }

    // Try HSL
    if let Ok(hsl) = parse_hsl(s) {
        return Ok(hsl);
    }

    // Try HSV
    if let Ok(hsv) = parse_hsv(s) {
        return Ok(hsv);
    }

    Err(ColorError::InvalidColorFormat(s.to_string()))
}

fn parse_hex(s: &str) -> Result<(u8, u8, u8), ColorError> {
    let clean = s.trim().trim_start_matches('#');
    match clean.len() {
        3 => {
            let r = u8::from_str_radix(&clean[0..1], 16)
                .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))? * 17;
            let g = u8::from_str_radix(&clean[1..2], 16)
                .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))? * 17;
            let b = u8::from_str_radix(&clean[2..3], 16)
                .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))? * 17;
            Ok((r, g, b))
        }
        4 => {
            // #RGBA format: ignore alpha for RGB representation
            let r = u8::from_str_radix(&clean[0..1], 16)
                .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))? * 17;
            let g = u8::from_str_radix(&clean[1..2], 16)
                .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))? * 17;
            let b = u8::from_str_radix(&clean[2..3], 16)
                .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))? * 17;
            Ok((r, g, b))
        }
        6 | 8 => {
            let r = u8::from_str_radix(&clean[0..2], 16)
                .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))?;
            let g = u8::from_str_radix(&clean[2..4], 16)
                .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))?;
            let b = u8::from_str_radix(&clean[4..6], 16)
                .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))?;
            Ok((r, g, b))
        }
        _ => Err(ColorError::InvalidColorFormat(s.to_string())),
    }
}

fn extract_numeric_tokens(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' || c == '-' {
            current.push(c);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_rgb(s: &str) -> Result<(u8, u8, u8), ColorError> {
    let tokens = extract_numeric_tokens(s);
    if tokens.len() < 3 {
        return Err(ColorError::InvalidColorFormat(s.to_string()));
    }

    let r_val = tokens[0]
        .parse::<f32>()
        .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))?;
    let g_val = tokens[1]
        .parse::<f32>()
        .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))?;
    let b_val = tokens[2]
        .parse::<f32>()
        .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))?;

    if !(0.0..=255.0).contains(&r_val) {
        return Err(ColorError::ValueOutOfRange(format!("Red channel ({r_val}) must be 0-255")));
    }
    if !(0.0..=255.0).contains(&g_val) {
        return Err(ColorError::ValueOutOfRange(format!("Green channel ({g_val}) must be 0-255")));
    }
    if !(0.0..=255.0).contains(&b_val) {
        return Err(ColorError::ValueOutOfRange(format!("Blue channel ({b_val}) must be 0-255")));
    }

    Ok((r_val.round() as u8, g_val.round() as u8, b_val.round() as u8))
}

fn parse_hsl(s: &str) -> Result<(u8, u8, u8), ColorError> {
    let tokens = extract_numeric_tokens(s);
    if tokens.len() < 3 {
        return Err(ColorError::InvalidColorFormat(s.to_string()));
    }

    let h = tokens[0]
        .parse::<f32>()
        .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))?;
    let s_pct = tokens[1]
        .parse::<f32>()
        .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))?;
    let l_pct = tokens[2]
        .parse::<f32>()
        .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))?;

    if !(0.0..=360.0).contains(&h) {
        return Err(ColorError::ValueOutOfRange(format!("Hue ({h}) must be 0-360")));
    }
    if !(0.0..=100.0).contains(&s_pct) {
        return Err(ColorError::ValueOutOfRange(format!("Saturation ({s_pct}) must be 0-100%")));
    }
    if !(0.0..=100.0).contains(&l_pct) {
        return Err(ColorError::ValueOutOfRange(format!("Lightness ({l_pct}) must be 0-100%")));
    }

    let h_norm = (h % 360.0 + 360.0) % 360.0;
    let s_norm = s_pct / 100.0;
    let l_norm = l_pct / 100.0;

    let c = (1.0 - (2.0 * l_norm - 1.0).abs()) * s_norm;
    let x = c * (1.0 - ((h_norm / 60.0) % 2.0 - 1.0).abs());
    let m = l_norm - c / 2.0;

    let (r_prime, g_prime, b_prime) = if h_norm < 60.0 {
        (c, x, 0.0)
    } else if h_norm < 120.0 {
        (x, c, 0.0)
    } else if h_norm < 180.0 {
        (0.0, c, x)
    } else if h_norm < 240.0 {
        (0.0, x, c)
    } else if h_norm < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let r = ((r_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = ((g_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = ((b_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8;

    Ok((r, g, b))
}

fn parse_hsv(s: &str) -> Result<(u8, u8, u8), ColorError> {
    let tokens = extract_numeric_tokens(s);
    if tokens.len() < 3 {
        return Err(ColorError::InvalidColorFormat(s.to_string()));
    }

    let h = tokens[0]
        .parse::<f32>()
        .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))?;
    let s_pct = tokens[1]
        .parse::<f32>()
        .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))?;
    let v_pct = tokens[2]
        .parse::<f32>()
        .map_err(|_| ColorError::InvalidColorFormat(s.to_string()))?;

    if !(0.0..=360.0).contains(&h) {
        return Err(ColorError::ValueOutOfRange(format!("Hue ({h}) must be 0-360")));
    }
    if !(0.0..=100.0).contains(&s_pct) {
        return Err(ColorError::ValueOutOfRange(format!("Saturation ({s_pct}) must be 0-100%")));
    }
    if !(0.0..=100.0).contains(&v_pct) {
        return Err(ColorError::ValueOutOfRange(format!("Value ({v_pct}) must be 0-100%")));
    }

    let h_norm = (h % 360.0 + 360.0) % 360.0;
    let s_norm = s_pct / 100.0;
    let v_norm = v_pct / 100.0;

    let c = v_norm * s_norm;
    let x = c * (1.0 - ((h_norm / 60.0) % 2.0 - 1.0).abs());
    let m = v_norm - c;

    let (r_prime, g_prime, b_prime) = if h_norm < 60.0 {
        (c, x, 0.0)
    } else if h_norm < 120.0 {
        (x, c, 0.0)
    } else if h_norm < 180.0 {
        (0.0, c, x)
    } else if h_norm < 240.0 {
        (0.0, x, c)
    } else if h_norm < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let r = ((r_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = ((g_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = ((b_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8;

    Ok((r, g, b))
}

fn rgb_to_output(r: u8, g: u8, b: u8) -> ColorOutput {
    let hex = format!("#{r:02x}{g:02x}{b:02x}");
    let rgb = format!("rgb({r}, {g}, {b})");

    let r_n = r as f32 / 255.0;
    let g_n = g as f32 / 255.0;
    let b_n = b as f32 / 255.0;

    let max = r_n.max(g_n).max(b_n);
    let min = r_n.min(g_n).min(b_n);
    let delta = max - min;

    // HSL calculation
    let l = (max + min) / 2.0;
    let (h_deg, s_hsl) = if delta.abs() < f32::EPSILON {
        (0.0, 0.0)
    } else {
        let s = delta / (1.0 - (2.0 * l - 1.0).abs());
        let h = if (max - r_n).abs() < f32::EPSILON {
            60.0 * (((g_n - b_n) / delta) % 6.0)
        } else if (max - g_n).abs() < f32::EPSILON {
            60.0 * (((b_n - r_n) / delta) + 2.0)
        } else {
            60.0 * (((r_n - g_n) / delta) + 4.0)
        };
        let h_pos = (h % 360.0 + 360.0) % 360.0;
        (h_pos, s)
    };

    let hsl = format!(
        "hsl({}, {}%, {}%)",
        h_deg.round() as u32,
        (s_hsl * 100.0).round().clamp(0.0, 100.0) as u32,
        (l * 100.0).round().clamp(0.0, 100.0) as u32
    );

    // HSV calculation
    let v = max;
    let s_hsv = if max.abs() < f32::EPSILON {
        0.0
    } else {
        delta / max
    };

    let hsv = format!(
        "hsv({}, {}%, {}%)",
        h_deg.round() as u32,
        (s_hsv * 100.0).round().clamp(0.0, 100.0) as u32,
        (v * 100.0).round().clamp(0.0, 100.0) as u32
    );

    // CMYK calculation
    let k = 1.0 - max;
    let (c, m, y, k_res) = if (1.0 - k).abs() < f32::EPSILON {
        (0.0, 0.0, 0.0, 1.0)
    } else {
        let denom = 1.0 - k;
        let c = (1.0 - r_n - k) / denom;
        let m = (1.0 - g_n - k) / denom;
        let y = (1.0 - b_n - k) / denom;
        (c.max(0.0), m.max(0.0), y.max(0.0), k)
    };

    let cmyk = format!(
        "cmyk({}%, {}%, {}%, {}%)",
        (c * 100.0).round().clamp(0.0, 100.0) as u32,
        (m * 100.0).round().clamp(0.0, 100.0) as u32,
        (y * 100.0).round().clamp(0.0, 100.0) as u32,
        (k_res * 100.0).round().clamp(0.0, 100.0) as u32
    );

    ColorOutput {
        hex,
        rgb,
        hsl,
        hsv,
        cmyk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_hex() {
        let input = ColorInput {
            value: "#6750A4".to_string(),
            format: ColorFormat::Hex,
        };
        let out = convert_color(&input).unwrap();
        assert_eq!(out.hex, "#6750a4");
        assert_eq!(out.rgb, "rgb(103, 80, 164)");
        assert_eq!(out.hsl, "hsl(256, 34%, 48%)");
        assert_eq!(out.hsv, "hsv(256, 51%, 64%)");
        assert_eq!(out.cmyk, "cmyk(37%, 51%, 0%, 36%)");
    }

    #[test]
    fn test_convert_rgb() {
        let input = ColorInput {
            value: "rgb(255, 0, 0)".to_string(),
            format: ColorFormat::Rgb,
        };
        let out = convert_color(&input).unwrap();
        assert_eq!(out.hex, "#ff0000");
        assert_eq!(out.rgb, "rgb(255, 0, 0)");
        assert_eq!(out.hsl, "hsl(0, 100%, 50%)");
        assert_eq!(out.hsv, "hsv(0, 100%, 100%)");
        assert_eq!(out.cmyk, "cmyk(0%, 100%, 100%, 0%)");
    }

    #[test]
    fn test_convert_hsl() {
        let input = ColorInput {
            value: "hsl(256, 34%, 48%)".to_string(),
            format: ColorFormat::Hsl,
        };
        let out = convert_color(&input).unwrap();
        assert_eq!(out.hex, "#6751a4");

        let green = ColorInput {
            value: "hsl(120, 100%, 50%)".to_string(),
            format: ColorFormat::Hsl,
        };
        assert_eq!(convert_color(&green).unwrap().hex, "#00ff00");
    }

    #[test]
    fn test_convert_hsv() {
        let input = ColorInput {
            value: "hsv(256, 51%, 64%)".to_string(),
            format: ColorFormat::Hsv,
        };
        let out = convert_color(&input).unwrap();
        assert_eq!(out.hex, "#6650a3");

        let cyan = ColorInput {
            value: "hsv(180, 100%, 100%)".to_string(),
            format: ColorFormat::Hsv,
        };
        assert_eq!(convert_color(&cyan).unwrap().hex, "#00ffff");
    }

    #[test]
    fn test_convert_auto_detection() {
        let out1 = parse_color("#2196F3").unwrap();
        assert_eq!(out1.rgb, "rgb(33, 150, 243)");

        let out2 = parse_color("rgb(0, 150, 136)").unwrap();
        assert_eq!(out2.hex, "#009688");

        let out3 = parse_color("6750A4").unwrap();
        assert_eq!(out3.hex, "#6750a4");
    }

    #[test]
    fn test_empty_input() {
        let res = parse_color("   ");
        assert_eq!(res, Err(ColorError::EmptyInput));
    }

    #[test]
    fn test_out_of_range() {
        let res = parse_color("rgb(300, 0, 0)");
        assert!(matches!(res, Err(ColorError::ValueOutOfRange(_))));
    }

    #[test]
    fn test_pure_black_and_white() {
        let black = parse_color("#000000").unwrap();
        assert_eq!(black.cmyk, "cmyk(0%, 0%, 0%, 100%)");
        assert_eq!(black.hsl, "hsl(0, 0%, 0%)");

        let white = parse_color("#ffffff").unwrap();
        assert_eq!(white.cmyk, "cmyk(0%, 0%, 0%, 0%)");
        assert_eq!(white.hsl, "hsl(0, 0%, 100%)");
    }
}
