//! Android JNI bridge stubs and helpers.
//!
//! Provides native integration with Android system APIs such as Material You
//! dynamic colors (Monet), accessibility font scaling, and WindowInsets (status bar height).

use std::sync::atomic::{AtomicU32, Ordering};

static STATUS_BAR_INSET: AtomicU32 = AtomicU32::new(0);

/// Set the cached status bar inset.
pub fn set_status_bar_inset(inset: f32) {
    STATUS_BAR_INSET.store(inset.to_bits(), Ordering::SeqCst);
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

/// Get the Android system accent color (Material You / Monet dynamic theming).
pub fn get_system_accent_color() -> Option<(u8, u8, u8)> {
    None
}

/// Get the system font scaling factor configured by the user in Android Settings.
pub fn get_font_scale() -> f32 {
    1.0
}
