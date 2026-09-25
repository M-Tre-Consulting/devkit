// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 MTRE Consulting

//! Android JNI bridge stubs and helpers.
//!
//! Provides native integration with Android system APIs such as Material You
//! dynamic colors (Monet), accessibility font scaling, and WindowInsets (status bar height).

use std::sync::atomic::{AtomicU32, Ordering};

static STATUS_BAR_INSET: AtomicU32 = AtomicU32::new(0);
static NAVIGATION_BAR_INSET: AtomicU32 = AtomicU32::new(0);

/// Set the cached status bar inset.
pub fn set_status_bar_inset(inset: f32) {
    STATUS_BAR_INSET.store(inset.to_bits(), Ordering::SeqCst);
}

/// Set the cached navigation bar / gesture inset.
pub fn set_navigation_bar_inset(inset: f32) {
    NAVIGATION_BAR_INSET.store(inset.to_bits(), Ordering::SeqCst);
}

/// Get the system status bar inset in logical pixels (dp).
pub fn get_status_bar_inset() -> f32 {
    let bits = STATUS_BAR_INSET.load(Ordering::SeqCst);
    let val = f32::from_bits(bits);
    if val > 0.0 {
        val
    } else {
        #[cfg(target_os = "android")]
        {
            24.0 // Standard fallback on Android mobile devices
        }
        #[cfg(not(target_os = "android"))]
        {
            0.0 // Non-mobile desktop has no status bar overlay
        }
    }
}

/// Get the system navigation bar / gesture inset in logical pixels (dp).
pub fn get_navigation_bar_inset() -> f32 {
    let bits = NAVIGATION_BAR_INSET.load(Ordering::SeqCst);
    let val = f32::from_bits(bits);
    if val > 0.0 {
        val
    } else {
        #[cfg(target_os = "android")]
        {
            16.0 // Standard fallback on Android devices with gesture navigation
        }
        #[cfg(not(target_os = "android"))]
        {
            0.0 // Non-mobile desktop has no navigation bar overlay
        }
    }
}

/// Query the Android system status bar height via JNI.
#[cfg(target_os = "android")]
pub fn query_status_bar_inset(app: &slint::android::AndroidApp) -> f32 {
    let vm_ptr = app.vm_as_ptr();
    let activity_ptr = app.activity_as_ptr();
    if vm_ptr.is_null() || activity_ptr.is_null() {
        return 24.0;
    }

    unsafe {
        let vm = match jni::JavaVM::from_raw(vm_ptr as *mut _) {
            Ok(v) => v,
            Err(_) => return 24.0,
        };
        let mut env = match vm.attach_current_thread() {
            Ok(e) => e,
            Err(_) => return 24.0,
        };

        let activity = jni::objects::JObject::from_raw(activity_ptr as _);

        let resources = match env.call_method(&activity, "getResources", "()Landroid/content/res/Resources;", &[]) {
            Ok(r) => match r.l() {
                Ok(obj) => obj,
                Err(_) => return 24.0,
            },
            Err(_) => return 24.0,
        };

        let res_name = match env.new_string("status_bar_height") {
            Ok(s) => s,
            Err(_) => return 24.0,
        };
        let def_type = match env.new_string("dimen") {
            Ok(s) => s,
            Err(_) => return 24.0,
        };
        let def_package = match env.new_string("android") {
            Ok(s) => s,
            Err(_) => return 24.0,
        };

        let id_val = match env.call_method(
            &resources,
            "getIdentifier",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
            &[(&res_name).into(), (&def_type).into(), (&def_package).into()],
        ) {
            Ok(v) => v.i().unwrap_or(0),
            Err(_) => 0,
        };

        if id_val <= 0 {
            return 24.0;
        }

        let px = match env.call_method(&resources, "getDimensionPixelSize", "(I)I", &[id_val.into()]) {
            Ok(v) => v.i().unwrap_or(0),
            Err(_) => return 24.0,
        };

        let metrics = match env.call_method(&resources, "getDisplayMetrics", "()Landroid/util/DisplayMetrics;", &[]) {
            Ok(m) => match m.l() {
                Ok(obj) => obj,
                Err(_) => return px as f32,
            },
            Err(_) => return px as f32,
        };

        let density = match env.get_field(&metrics, "density", "F") {
            Ok(d) => d.f().unwrap_or(1.0),
            Err(_) => 1.0,
        };

        if density > 0.0 {
            (px as f32) / density
        } else {
            px as f32
        }
    }
}

/// Query the Android system navigation bar / gesture inset height via JNI.
#[cfg(target_os = "android")]
pub fn query_navigation_bar_inset(app: &slint::android::AndroidApp) -> f32 {
    let vm_ptr = app.vm_as_ptr();
    let activity_ptr = app.activity_as_ptr();
    if vm_ptr.is_null() || activity_ptr.is_null() {
        return 16.0;
    }

    unsafe {
        let vm = match jni::JavaVM::from_raw(vm_ptr as *mut _) {
            Ok(v) => v,
            Err(_) => return 16.0,
        };
        let mut env = match vm.attach_current_thread() {
            Ok(e) => e,
            Err(_) => return 16.0,
        };

        let activity = jni::objects::JObject::from_raw(activity_ptr as _);

        let mut inset_px = 0;

        // Attempt 1: Modern Android 11+ (API 30+) WindowMetrics API
        if let Ok(wm) = env.call_method(&activity, "getWindowManager", "()Landroid/view/WindowManager;", &[]) {
            if let Ok(wm_obj) = wm.l() {
                if let Ok(metrics) = env.call_method(&wm_obj, "getCurrentWindowMetrics", "()Landroid/view/WindowMetrics;", &[]) {
                    if let Ok(metrics_obj) = metrics.l() {
                        if let Ok(window_insets) = env.call_method(&metrics_obj, "getWindowInsets", "()Landroid/view/WindowInsets;", &[]) {
                            if let Ok(winsets_obj) = window_insets.l() {
                                // 2 = WindowInsets.Type.navigationBars()
                                if let Ok(insets) = env.call_method(&winsets_obj, "getInsets", "(I)Landroid/graphics/Insets;", &[2i32.into()]) {
                                    if let Ok(insets_obj) = insets.l() {
                                        if let Ok(bottom_val) = env.get_field(&insets_obj, "bottom", "I") {
                                            inset_px = bottom_val.i().unwrap_or(0);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let _ = env.exception_clear();

        // Attempt 2: Query WindowInsets from DecorView (API 20+)
        if inset_px <= 0 {
            if let Ok(window) = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[]) {
                if let Ok(window_obj) = window.l() {
                    if let Ok(decor_view) = env.call_method(&window_obj, "getDecorView", "()Landroid/view/View;", &[]) {
                        if let Ok(decor_obj) = decor_view.l() {
                            if let Ok(insets) = env.call_method(&decor_obj, "getRootWindowInsets", "()Landroid/view/WindowInsets;", &[]) {
                                if let Ok(insets_obj) = insets.l() {
                                    if !insets_obj.as_raw().is_null() {
                                        if let Ok(b) = env.call_method(&insets_obj, "getSystemWindowInsetBottom", "()I", &[]) {
                                            inset_px = b.i().unwrap_or(0);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let _ = env.exception_clear();
        }

        let resources = match env.call_method(&activity, "getResources", "()Landroid/content/res/Resources;", &[]) {
            Ok(r) => match r.l() {
                Ok(obj) => obj,
                Err(_) => {
                    let _ = env.exception_clear();
                    return 16.0;
                }
            },
            Err(_) => {
                let _ = env.exception_clear();
                return 16.0;
            }
        };

        // Attempt 3: Fallback to system dimension resource "navigation_bar_height"
        if inset_px <= 0 {
            let res_name = match env.new_string("navigation_bar_height") {
                Ok(s) => s,
                Err(_) => return 16.0,
            };
            let def_type = match env.new_string("dimen") {
                Ok(s) => s,
                Err(_) => return 16.0,
            };
            let def_package = match env.new_string("android") {
                Ok(s) => s,
                Err(_) => return 16.0,
            };

            let id_val = match env.call_method(
                &resources,
                "getIdentifier",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
                &[(&res_name).into(), (&def_type).into(), (&def_package).into()],
            ) {
                Ok(v) => v.i().unwrap_or(0),
                Err(_) => 0,
            };

            if id_val > 0 {
                inset_px = match env.call_method(&resources, "getDimensionPixelSize", "(I)I", &[id_val.into()]) {
                    Ok(v) => v.i().unwrap_or(0),
                    Err(_) => 0,
                };
            }
            let _ = env.exception_clear();
        }

        if inset_px <= 0 {
            return 16.0;
        }

        let metrics = match env.call_method(&resources, "getDisplayMetrics", "()Landroid/util/DisplayMetrics;", &[]) {
            Ok(m) => match m.l() {
                Ok(obj) => obj,
                Err(_) => return inset_px as f32,
            },
            Err(_) => return inset_px as f32,
        };

        let density = match env.get_field(&metrics, "density", "F") {
            Ok(d) => d.f().unwrap_or(1.0),
            Err(_) => 1.0,
        };

        if density > 0.0 {
            (inset_px as f32) / density
        } else {
            inset_px as f32
        }
    }
}


/// Get the Android system accent color (Material You / Monet dynamic theming).
pub fn get_system_accent_color() -> Option<(u8, u8, u8)> {
    None
}

/// Get the system font scaling factor configured by the user in Android Settings.
pub fn get_font_scale() -> f32 {
    1.0
}

/// Request the display's maximum supported refresh rate (e.g., 90 Hz or 120 Hz).
///
/// Intended behavior when the JNI bridge is implemented:
/// - Query the display's supported modes.
/// - Pick the mode with the highest refresh rate.
/// - Call Window.setFrameRate() or set preferredDisplayModeId on API 30+.
/// - Fall back gracefully on older Android versions.
/// - Do nothing if enabled is false.
pub fn apply_refresh_rate_setting(enabled: bool) {
    if !enabled {
        return;
    }
    todo!("Query the display's supported modes, pick the mode with the highest refresh rate, call Window.setFrameRate() or set preferredDisplayModeId on API 30+, and fall back gracefully on older Android versions")
}
