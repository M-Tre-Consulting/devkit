//! Android JNI bridge stubs.
//!
//! Provides native integration with Android system APIs such as Material You
//! dynamic colors (Monet) and accessibility font scaling.

/// Get the Android system accent color (Material You / Monet dynamic theming).
///
/// Returns RGB tuple `(r, g, b)` if available, or `None` if dynamic theming
/// is unsupported (pre-API 31 / Android 12) or unavailable.
///
/// TODO: Use JNI via `android-activity`'s VM pointer:
/// 1. Acquire JNIEnv from `slint::android::AndroidApp::vm()`.
/// 2. Call `Context.getColor(android.R.color.system_accent1_500)` on API 31+.
/// 3. Extract the integer RGB channels and map to Slint color values.
pub fn get_system_accent_color() -> Option<(u8, u8, u8)> {
    // Default fallback (no custom accent overridden yet)
    None
}

/// Get the system font scaling factor configured by the user in Android Settings.
///
/// Returns a multiplier where `1.0` is the default font size.
///
/// TODO: Use JNI to query the Android Configuration:
/// 1. Acquire JNIEnv from `slint::android::AndroidApp::vm()`.
/// 2. Call `context.getResources().getConfiguration().fontScale`.
/// 3. Return the float value (typically between 0.85 and 2.0).
pub fn get_font_scale() -> f32 {
    // Default fallback: 1.0 (standard scale)
    1.0
}
