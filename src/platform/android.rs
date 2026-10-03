// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 MTRE Consulting

//! Android JNI bridge stubs and helpers.
//!
//! Provides native integration with Android system APIs such as Material You
//! dynamic colors (Monet), accessibility font scaling, and WindowInsets (status bar height).

#[cfg(target_os = "android")]
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::{AtomicU32, Ordering};

static STATUS_BAR_INSET: AtomicU32 = AtomicU32::new(0);
static NAVIGATION_BAR_INSET: AtomicU32 = AtomicU32::new(0);
static KEYBOARD_INSET: AtomicU32 = AtomicU32::new(0);
#[cfg(target_os = "android")]
static VM_PTR: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());
#[cfg(target_os = "android")]
static ACTIVITY_PTR: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

/// Set the cached keyboard inset.
pub fn set_keyboard_inset(inset: f32) {
    KEYBOARD_INSET.store(inset.to_bits(), Ordering::SeqCst);
}

/// Get the system keyboard inset in logical pixels (dp).
pub fn get_keyboard_inset() -> f32 {
    let bits = KEYBOARD_INSET.load(Ordering::SeqCst);
    f32::from_bits(bits)
}

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

        let resources = match env.call_method(
            &activity,
            "getResources",
            "()Landroid/content/res/Resources;",
            &[],
        ) {
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
            &[
                (&res_name).into(),
                (&def_type).into(),
                (&def_package).into(),
            ],
        ) {
            Ok(v) => v.i().unwrap_or(0),
            Err(_) => 0,
        };

        if id_val <= 0 {
            return 24.0;
        }

        let px = match env.call_method(
            &resources,
            "getDimensionPixelSize",
            "(I)I",
            &[id_val.into()],
        ) {
            Ok(v) => v.i().unwrap_or(0),
            Err(_) => return 24.0,
        };

        let metrics = match env.call_method(
            &resources,
            "getDisplayMetrics",
            "()Landroid/util/DisplayMetrics;",
            &[],
        ) {
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
        if let Ok(wm) = env.call_method(
            &activity,
            "getWindowManager",
            "()Landroid/view/WindowManager;",
            &[],
        ) {
            if let Ok(wm_obj) = wm.l() {
                if let Ok(metrics) = env.call_method(
                    &wm_obj,
                    "getCurrentWindowMetrics",
                    "()Landroid/view/WindowMetrics;",
                    &[],
                ) {
                    if let Ok(metrics_obj) = metrics.l() {
                        if let Ok(window_insets) = env.call_method(
                            &metrics_obj,
                            "getWindowInsets",
                            "()Landroid/view/WindowInsets;",
                            &[],
                        ) {
                            if let Ok(winsets_obj) = window_insets.l() {
                                // 2 = WindowInsets.Type.navigationBars()
                                if let Ok(insets) = env.call_method(
                                    &winsets_obj,
                                    "getInsets",
                                    "(I)Landroid/graphics/Insets;",
                                    &[2i32.into()],
                                ) {
                                    if let Ok(insets_obj) = insets.l() {
                                        if let Ok(bottom_val) =
                                            env.get_field(&insets_obj, "bottom", "I")
                                        {
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
            if let Ok(window) =
                env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
            {
                if let Ok(window_obj) = window.l() {
                    if let Ok(decor_view) =
                        env.call_method(&window_obj, "getDecorView", "()Landroid/view/View;", &[])
                    {
                        if let Ok(decor_obj) = decor_view.l() {
                            if let Ok(insets) = env.call_method(
                                &decor_obj,
                                "getRootWindowInsets",
                                "()Landroid/view/WindowInsets;",
                                &[],
                            ) {
                                if let Ok(insets_obj) = insets.l() {
                                    if !insets_obj.as_raw().is_null() {
                                        if let Ok(b) = env.call_method(
                                            &insets_obj,
                                            "getSystemWindowInsetBottom",
                                            "()I",
                                            &[],
                                        ) {
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

        let resources = match env.call_method(
            &activity,
            "getResources",
            "()Landroid/content/res/Resources;",
            &[],
        ) {
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
                &[
                    (&res_name).into(),
                    (&def_type).into(),
                    (&def_package).into(),
                ],
            ) {
                Ok(v) => v.i().unwrap_or(0),
                Err(_) => 0,
            };

            if id_val > 0 {
                inset_px = match env.call_method(
                    &resources,
                    "getDimensionPixelSize",
                    "(I)I",
                    &[id_val.into()],
                ) {
                    Ok(v) => v.i().unwrap_or(0),
                    Err(_) => 0,
                };
            }
            let _ = env.exception_clear();
        }

        if inset_px <= 0 {
            return 16.0;
        }

        let metrics = match env.call_method(
            &resources,
            "getDisplayMetrics",
            "()Landroid/util/DisplayMetrics;",
            &[],
        ) {
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

/// Configure the window soft input mode to adjustResize (0x10) so Android notifies
/// the window of IME insets and resizes the content area.
#[cfg(target_os = "android")]
pub fn configure_window_soft_input_mode(app: &slint::android::AndroidApp) {
    let vm_ptr = app.vm_as_ptr();
    let activity_ptr = app.activity_as_ptr();
    VM_PTR.store(vm_ptr as *mut _, Ordering::SeqCst);
    ACTIVITY_PTR.store(activity_ptr as *mut _, Ordering::SeqCst);
    if vm_ptr.is_null() || activity_ptr.is_null() {
        return;
    }

    unsafe {
        let vm = match jni::JavaVM::from_raw(vm_ptr as *mut _) {
            Ok(v) => v,
            Err(_) => return,
        };
        let mut env = match vm.attach_current_thread() {
            Ok(e) => e,
            Err(_) => return,
        };

        let activity = jni::objects::JObject::from_raw(activity_ptr as _);
        if let Ok(window) = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
        {
            if let Ok(window_obj) = window.l() {
                // WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE = 0x10 (16)
                let _ = env.call_method(&window_obj, "setSoftInputMode", "(I)V", &[16i32.into()]);
            }
        }
        let _ = env.exception_clear();
    }
}

/// Query the Android system keyboard (IME) inset height in logical pixels (dp) via JNI.
#[cfg(target_os = "android")]
pub fn query_keyboard_inset() -> f32 {
    let vm_ptr = VM_PTR.load(Ordering::SeqCst);
    let activity_ptr = ACTIVITY_PTR.load(Ordering::SeqCst);
    if vm_ptr.is_null() || activity_ptr.is_null() {
        return get_keyboard_inset();
    }

    unsafe {
        let vm = match jni::JavaVM::from_raw(vm_ptr as *mut _) {
            Ok(v) => v,
            Err(_) => return get_keyboard_inset(),
        };
        let mut env = match vm.attach_current_thread() {
            Ok(e) => e,
            Err(_) => return get_keyboard_inset(),
        };

        let activity = jni::objects::JObject::from_raw(activity_ptr as _);

        let window = match env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[]) {
            Ok(w) => match w.l() {
                Ok(obj) => obj,
                Err(_) => {
                    let _ = env.exception_clear();
                    return get_keyboard_inset();
                }
            },
            Err(_) => {
                let _ = env.exception_clear();
                return get_keyboard_inset();
            }
        };

        let decor_view =
            match env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[]) {
                Ok(d) => match d.l() {
                    Ok(obj) => obj,
                    Err(_) => {
                        let _ = env.exception_clear();
                        return get_keyboard_inset();
                    }
                },
                Err(_) => {
                    let _ = env.exception_clear();
                    return get_keyboard_inset();
                }
            };

        let insets = match env.call_method(
            &decor_view,
            "getRootWindowInsets",
            "()Landroid/view/WindowInsets;",
            &[],
        ) {
            Ok(i) => match i.l() {
                Ok(obj) => obj,
                Err(_) => {
                    let _ = env.exception_clear();
                    return get_keyboard_inset();
                }
            },
            Err(_) => {
                let _ = env.exception_clear();
                return get_keyboard_inset();
            }
        };

        if insets.as_raw().is_null() {
            return get_keyboard_inset();
        }

        let mut ime_bottom_px = 0;

        // WindowInsets.Type.ime() = 8 (API 30+)
        if let Ok(ime_insets) = env.call_method(
            &insets,
            "getInsets",
            "(I)Landroid/graphics/Insets;",
            &[8i32.into()],
        ) {
            if let Ok(ime_obj) = ime_insets.l() {
                if !ime_obj.as_raw().is_null() {
                    if let Ok(bottom_val) = env.get_field(&ime_obj, "bottom", "I") {
                        ime_bottom_px = bottom_val.i().unwrap_or(0);
                    }
                }
            }
        }
        let _ = env.exception_clear();

        // Fallback for API < 30: compare getSystemWindowInsetBottom with navigation bar inset
        if ime_bottom_px <= 0 {
            if let Ok(bottom_val) =
                env.call_method(&insets, "getSystemWindowInsetBottom", "()I", &[])
            {
                let total_bottom = bottom_val.i().unwrap_or(0);
                let nav_px = (get_navigation_bar_inset() * 2.625) as i32;
                if total_bottom > nav_px + 50 {
                    ime_bottom_px = total_bottom - nav_px;
                }
            }
            let _ = env.exception_clear();
        }

        let resources = match env.call_method(
            &activity,
            "getResources",
            "()Landroid/content/res/Resources;",
            &[],
        ) {
            Ok(r) => match r.l() {
                Ok(obj) => obj,
                Err(_) => {
                    let _ = env.exception_clear();
                    return 0.0;
                }
            },
            Err(_) => {
                let _ = env.exception_clear();
                return 0.0;
            }
        };

        let metrics = match env.call_method(
            &resources,
            "getDisplayMetrics",
            "()Landroid/util/DisplayMetrics;",
            &[],
        ) {
            Ok(m) => match m.l() {
                Ok(obj) => obj,
                Err(_) => {
                    let _ = env.exception_clear();
                    return ime_bottom_px as f32;
                }
            },
            Err(_) => {
                let _ = env.exception_clear();
                return ime_bottom_px as f32;
            }
        };

        let density = match env.get_field(&metrics, "density", "F") {
            Ok(d) => d.f().unwrap_or(1.0),
            Err(_) => {
                let _ = env.exception_clear();
                1.0
            }
        };

        let dp = if density > 0.0 && ime_bottom_px > 0 {
            (ime_bottom_px as f32) / density
        } else {
            0.0
        };
        set_keyboard_inset(dp);
        dp
    }
}

#[cfg(not(target_os = "android"))]
pub fn query_keyboard_inset() -> f32 {
    get_keyboard_inset()
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
/// - Guard API level: Verify Build.VERSION.SDK_INT >= Build.VERSION_CODES.R (API 30+) before calling setFrameRate().
/// - Validate display modes: Query Display.getSupportedModes() to find the actual maximum supported refresh rate before requesting it.
/// - Seamless transition: When requesting a higher refresh rate, use the strategy that avoids visual interruptions.
///   Passing CHANGE_FRAME_RATE_ALWAYS can cause a black screen or flicker on some devices.
///   Use Surface.CHANGE_FRAME_RATE_ONLY_IF_SEAMLESS (or ANATIVEWINDOW_CHANGE_FRAME_RATE_ONLY_IF_SEAMLESS in native code) to avoid flicker.
/// - Graceful failure: If the device does not support the requested rate or the system overrides it (e.g., battery saver),
///   catch any exceptions and silently revert to the default behavior.
/// - Do nothing if enabled is false.
pub fn apply_refresh_rate_setting(enabled: bool) {
    if !enabled {
        return;
    }
    todo!("Query Display.getSupportedModes(), check SDK_INT >= 30, use seamless frame rate switching strategy, and gracefully handle unsupported displays")
}

/// Query whether the device display hardware supports high refresh rates (>= 90 Hz).
///
/// On Android, queries Display.getSupportedModes() via JNI and checks if any available
/// mode supports a refresh rate >= 89.0 Hz.
/// Returns false on 60 Hz devices, on non-Android platforms, or if JNI query fails.
#[cfg(target_os = "android")]
pub fn is_high_refresh_rate_supported() -> bool {
    let vm_ptr = VM_PTR.load(Ordering::SeqCst);
    let activity_ptr = ACTIVITY_PTR.load(Ordering::SeqCst);
    if vm_ptr.is_null() || activity_ptr.is_null() {
        return false;
    }

    unsafe {
        let vm = match jni::JavaVM::from_raw(vm_ptr as *mut _) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let mut env = match vm.attach_current_thread() {
            Ok(e) => e,
            Err(_) => return false,
        };

        let activity = jni::objects::JObject::from_raw(activity_ptr as _);
        let window_manager = match env.call_method(
            &activity,
            "getWindowManager",
            "()Landroid/view/WindowManager;",
            &[],
        ) {
            Ok(r) => match r.l() {
                Ok(wm) => wm,
                Err(_) => {
                    let _ = env.exception_clear();
                    return false;
                }
            },
            Err(_) => {
                let _ = env.exception_clear();
                return false;
            }
        };

        let display = match env.call_method(
            &window_manager,
            "getDefaultDisplay",
            "()Landroid/view/Display;",
            &[],
        ) {
            Ok(r) => match r.l() {
                Ok(d) => d,
                Err(_) => {
                    let _ = env.exception_clear();
                    return false;
                }
            },
            Err(_) => {
                let _ = env.exception_clear();
                return false;
            }
        };

        let modes_val = match env.call_method(
            &display,
            "getSupportedModes",
            "()[Landroid/view/Display$Mode;",
            &[],
        ) {
            Ok(r) => match r.l() {
                Ok(arr) => arr,
                Err(_) => {
                    let _ = env.exception_clear();
                    return false;
                }
            },
            Err(_) => {
                let _ = env.exception_clear();
                return false;
            }
        };

        let modes_array = jni::objects::JObjectArray::from_raw(modes_val.as_raw());
        let count = env.get_array_length(&modes_array).unwrap_or(0);
        let mut max_rate = 0.0f32;

        for i in 0..count {
            if let Ok(mode) = env.get_object_array_element(&modes_array, i) {
                let rate = env
                    .call_method(&mode, "getRefreshRate", "()F", &[])
                    .and_then(|r| r.f())
                    .unwrap_or(0.0);
                if rate > max_rate {
                    max_rate = rate;
                }
            }
        }
        let _ = env.exception_clear();

        // Only supported if at least 90Hz (89.0 to account for ~89.9Hz modes)
        max_rate >= 89.0
    }
}

/// Fallback for non-Android platforms.
#[cfg(not(target_os = "android"))]
pub fn is_high_refresh_rate_supported() -> bool {
    false
}

/// Perform a subtle haptic tap (KEYBOARD_TAP) via JNI on Android.
///
/// Respects the user's system haptic feedback setting (Settings.System.HAPTIC_FEEDBACK_ENABLED).
#[cfg(target_os = "android")]
pub fn haptic_tap() {
    let vm_ptr = VM_PTR.load(Ordering::SeqCst);
    let activity_ptr = ACTIVITY_PTR.load(Ordering::SeqCst);
    if vm_ptr.is_null() || activity_ptr.is_null() {
        return;
    }

    unsafe {
        let vm = match jni::JavaVM::from_raw(vm_ptr as *mut _) {
            Ok(v) => v,
            Err(_) => return,
        };
        let mut env = match vm.attach_current_thread() {
            Ok(e) => e,
            Err(_) => return,
        };

        let activity = jni::objects::JObject::from_raw(activity_ptr as _);
        if let Ok(window) = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
        {
            if let Ok(window_obj) = window.l() {
                if let Ok(decor) =
                    env.call_method(&window_obj, "getDecorView", "()Landroid/view/View;", &[])
                {
                    if let Ok(decor_obj) = decor.l() {
                        // HapticFeedbackConstants.KEYBOARD_TAP = 3
                        // Automatically checks and respects system haptic feedback settings
                        let _ = env.call_method(
                            &decor_obj,
                            "performHapticFeedback",
                            "(I)Z",
                            &[3i32.into()],
                        );
                    }
                }
            }
        }
        let _ = env.exception_clear();
    }
}

/// Fallback for non-Android targets.
#[cfg(not(target_os = "android"))]
pub fn haptic_tap() {}
