// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Contrast Checker module.
//!
//! Provides WCAG 2.1 color contrast calculation between foreground and background colors.

use crate::tools::color::{self, ColorFormat};

/// Input colors for contrast verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContrastInput {
    pub foreground: String,
    pub background: String,
}

/// Detailed WCAG 2.1 contrast verification results.
#[derive(Debug, Clone, PartialEq)]
pub struct ContrastResult {
    pub ratio: f32,
    pub ratio_display: String,
    pub aa_normal: bool,
    pub aa_large: bool,
    pub aaa_normal: bool,
    pub aaa_large: bool,
    pub ui_components: bool,
}

impl Default for ContrastResult {
    fn default() -> Self {
        Self {
            ratio: 1.0,
            ratio_display: "1.00 : 1".to_string(),
            aa_normal: false,
            aa_large: false,
            aaa_normal: false,
            aaa_large: false,
            ui_components: false,
        }
    }
}

/// Errors that can occur during color parsing or contrast computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContrastError {
    InvalidForegroundColor(String),
    InvalidBackgroundColor(String),
    EmptyInput,
}

impl std::fmt::Display for ContrastError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidForegroundColor(val) => {
                write!(f, "Invalid foreground color specification: '{val}'")
            }
            Self::InvalidBackgroundColor(val) => {
                write!(f, "Invalid background color specification: '{val}'")
            }
            Self::EmptyInput => write!(f, "Color input cannot be empty"),
        }
    }
}

impl std::error::Error for ContrastError {}

/// Calculate the contrast ratio between foreground and background colors and check WCAG compliance.
pub fn check_contrast(input: &ContrastInput) -> Result<ContrastResult, ContrastError> {
    if input.foreground.trim().is_empty() || input.background.trim().is_empty() {
        return Err(ContrastError::EmptyInput);
    }

    let fg_rgb = color::parse_to_rgb(&input.foreground, ColorFormat::Auto)
        .map_err(|_| ContrastError::InvalidForegroundColor(input.foreground.clone()))?;
    let bg_rgb = color::parse_to_rgb(&input.background, ColorFormat::Auto)
        .map_err(|_| ContrastError::InvalidBackgroundColor(input.background.clone()))?;

    let l_fg = relative_luminance(fg_rgb.0, fg_rgb.1, fg_rgb.2);
    let l_bg = relative_luminance(bg_rgb.0, bg_rgb.1, bg_rgb.2);

    let l1 = l_fg.max(l_bg);
    let l2 = l_fg.min(l_bg);

    let ratio_f64 = (l1 as f64 + 0.05) / (l2 as f64 + 0.05);
    let ratio = ((ratio_f64 * 100.0).round() / 100.0) as f32;
    let ratio_display = format!("{:.2} : 1", ratio);

    let aa_normal = ratio >= 4.5;
    let aa_large = ratio >= 3.0;
    let aaa_normal = ratio >= 7.0;
    let aaa_large = ratio >= 4.5;
    let ui_components = ratio >= 3.0;

    Ok(ContrastResult {
        ratio,
        ratio_display,
        aa_normal,
        aa_large,
        aaa_normal,
        aaa_large,
        ui_components,
    })
}

/// Calculate contrast ratio between two raw color strings.
pub fn calculate_ratio(fg: &str, bg: &str) -> Result<ContrastResult, ContrastError> {
    check_contrast(&ContrastInput {
        foreground: fg.to_string(),
        background: bg.to_string(),
    })
}

/// Computes relative luminance according to WCAG 2.1 specification using sRGB gamma expansion.
fn relative_luminance(r: u8, g: u8, b: u8) -> f32 {
    let r_lin = linearize(r as f64 / 255.0);
    let g_lin = linearize(g as f64 / 255.0);
    let b_lin = linearize(b as f64 / 255.0);

    (0.2126 * r_lin + 0.7152 * g_lin + 0.0722 * b_lin) as f32
}

fn linearize(c: f64) -> f64 {
    if c >= 1.0 {
        1.0
    } else if c <= 0.0 {
        0.0
    } else if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_high_contrast_black_white() {
        let input = ContrastInput {
            foreground: "#000000".to_string(),
            background: "#ffffff".to_string(),
        };
        let out = check_contrast(&input).unwrap();
        assert!(out.ratio >= 21.0);
        assert_eq!(out.ratio_display, "21.00 : 1");
        assert!(out.aa_normal && out.aaa_normal);
        assert!(out.aa_large && out.aaa_large && out.ui_components);
    }

    #[test]
    fn test_low_contrast() {
        let input = ContrastInput {
            foreground: "#777777".to_string(),
            background: "#888888".to_string(),
        };
        let out = check_contrast(&input).unwrap();
        assert!(!out.aa_normal);
        assert!(!out.aaa_normal);
    }

    #[test]
    fn test_material_theme_contrast() {
        // White text on purple primary (#6750A4)
        let out = calculate_ratio("#FFFFFF", "#6750A4").unwrap();
        assert!(out.ratio >= 4.5); // Should pass AA normal
        assert!(out.aa_normal);
    }

    #[test]
    fn test_empty_input() {
        let res = calculate_ratio("", "#ffffff");
        assert_eq!(res, Err(ContrastError::EmptyInput));
    }

    #[test]
    fn test_invalid_colors() {
        let res1 = calculate_ratio("invalid-color", "#ffffff");
        assert!(matches!(res1, Err(ContrastError::InvalidForegroundColor(_))));

        let res2 = calculate_ratio("#000000", "not-a-color");
        assert!(matches!(res2, Err(ContrastError::InvalidBackgroundColor(_))));
    }
}
