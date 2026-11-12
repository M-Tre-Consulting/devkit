// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Android JNI bridge and platform helpers.
//!
//! All JNI access goes through [`with_activity`], which:
//! - reuses one cached `JavaVM` instead of re-wrapping the raw pointer on every call,
//! - attaches the calling thread once (permanently) instead of attach/detach churn,
//! - runs every call inside a JNI local frame, so local references are released
//!   immediately. The UI thread never returns to Java, so without a frame every
//!   `getWindow()`, `getDecorView()`, ... result would leak for the app's lifetime,
//!   growing the local reference table and slowing down the GC.

use std::sync::atomic::{AtomicU32, Ordering};

static STATUS_BAR_INSET: AtomicU32 = AtomicU32::new(0);
static NAVIGATION_BAR_INSET: AtomicU32 = AtomicU32::new(0);

/// Set the cached status bar inset.
pub fn set_status_bar_inset(inset: f32) {
    STATUS_BAR_INSET.store(inset.to_bits(), Ordering::Relaxed);
}

/// Set the cached navigation bar / gesture inset.
pub fn set_navigation_bar_inset(inset: f32) {
    NAVIGATION_BAR_INSET.store(inset.to_bits(), Ordering::Relaxed);
}

fn inset_or(bits: &AtomicU32, android_fallback: f32) -> f32 {
    let val = f32::from_bits(bits.load(Ordering::Relaxed));
    if val > 0.0 {
        val
    } else if cfg!(target_os = "android") {
        android_fallback
    } else {
        0.0
    }
}

/// System status bar inset in logical pixels (dp).
pub fn get_status_bar_inset() -> f32 {
    inset_or(&STATUS_BAR_INSET, 24.0)
}

/// System navigation bar / gesture inset in logical pixels (dp).
pub fn get_navigation_bar_inset() -> f32 {
    inset_or(&NAVIGATION_BAR_INSET, 16.0)
}

/// Extracted Android Material You (Monet) system dynamic palette colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonetColors {
    pub primary_light: (u8, u8, u8),
    pub primary_dark: (u8, u8, u8),
    pub primary_container_light: (u8, u8, u8),
    pub primary_container_dark: (u8, u8, u8),
    pub secondary_light: (u8, u8, u8),
    pub secondary_dark: (u8, u8, u8),
    pub tertiary_light: (u8, u8, u8),
    pub tertiary_dark: (u8, u8, u8),
    pub surface_light: (u8, u8, u8),
    pub surface_dark: (u8, u8, u8),
}

/// Which GPU API the renderer should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsApi {
    /// Skia over Vulkan (through WGPU), falls back to OpenGL ES automatically.
    Vulkan,
    /// Skia over OpenGL ES.
    OpenGlEs,
}

impl GraphicsApi {
    pub fn label(&self) -> &'static str {
        match self {
            GraphicsApi::Vulkan => "Vulkan",
            GraphicsApi::OpenGlEs => "OpenGL ES",
        }
    }
}

/// Resolve the user preference (0 = auto, 1 = Vulkan, 2 = OpenGL ES) to a concrete API.
///
/// In auto mode, Tensor G5/G6 (Imagination DXT GPU) get OpenGL ES: Skia's Vulkan path on
/// that driver shows pipeline-compile hitches and uneven frame pacing. Everything else
/// keeps Vulkan.
pub fn resolve_graphics_api(preference: i32) -> GraphicsApi {
    match preference {
        1 => GraphicsApi::Vulkan,
        2 => GraphicsApi::OpenGlEs,
        _ => {
            if is_tensor_g5_or_newer() {
                GraphicsApi::OpenGlEs
            } else {
                GraphicsApi::Vulkan
            }
        }
    }
}

/// Snapshot of the active network as reported by the OS.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NetworkInfo {
    pub interface: Option<String>,
    pub device_ip: Option<String>,
    pub prefix_len: Option<u8>,
    pub gateway: Option<String>,
    pub dns_servers: Vec<String>,
}

/// Classify a `Build.SOC_MODEL` / `Build.HARDWARE` style string.
///
/// Returns true for Google Tensor G5 and later ("laguna", "malibu", "Tensor G5", ...) and
/// for Imagination PowerVR based hardware strings.
pub fn soc_strings_indicate_tensor_g5_plus(fields: &[&str]) -> bool {
    for raw in fields {
        let f = raw.to_ascii_lowercase();
        if f.contains("laguna") || f.contains("malibu") || f.contains("powervr") {
            return true;
        }
        // "tensor g5", "tensor-g5", "tensor_g5", "tensorg6", ...
        if let Some(pos) = f.find("tensor") {
            let rest = f[pos + "tensor".len()..].trim_start_matches([' ', '-', '_']);
            if let Some(num) = rest.strip_prefix('g') {
                let digits: String = num.chars().take_while(|c| c.is_ascii_digit()).collect();
                if digits.parse::<u32>().map(|n| n >= 5).unwrap_or(false) {
                    return true;
                }
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Android implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "android")]
mod imp {
    use super::*;
    use jni::objects::{JObject, JString, JValue};
    use jni::JNIEnv;
    use std::sync::atomic::AtomicPtr;
    use std::sync::OnceLock;

    static VM: OnceLock<jni::JavaVM> = OnceLock::new();
    static ACTIVITY: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());
    static DENSITY: AtomicU32 = AtomicU32::new(0);
    static IS_TENSOR_G5: OnceLock<bool> = OnceLock::new();

    /// Cache the VM and activity handles. Must be called first, from `android_main`.
    pub fn init(app: &slint::android::AndroidApp) {
        let vm_ptr = app.vm_as_ptr();
        let activity_ptr = app.activity_as_ptr();
        if vm_ptr.is_null() || activity_ptr.is_null() {
            return;
        }
        if let Ok(vm) = unsafe { jni::JavaVM::from_raw(vm_ptr as *mut _) } {
            let _ = VM.set(vm);
        }
        ACTIVITY.store(activity_ptr as *mut _, Ordering::SeqCst);
    }

    /// Run `f` with an attached JNIEnv and the activity, inside a local reference frame.
    pub fn with_activity<R>(
        f: impl for<'a> FnOnce(&mut JNIEnv<'a>, &JObject<'static>) -> jni::errors::Result<R>,
    ) -> Option<R> {
        let vm = VM.get()?;
        let act = ACTIVITY.load(Ordering::SeqCst);
        if act.is_null() {
            return None;
        }
        let mut env = vm.attach_current_thread_permanently().ok()?;
        let activity: JObject<'static> = unsafe { JObject::from_raw(act as jni::sys::jobject) };
        let result = env.with_local_frame(32, |env| f(env, &activity));
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
        result.ok()
    }

    fn get_string(env: &mut JNIEnv, obj: JObject) -> jni::errors::Result<Option<String>> {
        if obj.is_null() {
            return Ok(None);
        }
        let s = JString::from(obj);
        let out: String = env.get_string(&s)?.into();
        Ok(Some(out))
    }

    fn density(env: &mut JNIEnv, activity: &JObject) -> jni::errors::Result<f32> {
        let cached = DENSITY.load(Ordering::Relaxed);
        if cached != 0 {
            return Ok(f32::from_bits(cached));
        }
        let res = env
            .call_method(activity, "getResources", "()Landroid/content/res/Resources;", &[])?
            .l()?;
        let metrics = env
            .call_method(&res, "getDisplayMetrics", "()Landroid/util/DisplayMetrics;", &[])?
            .l()?;
        let d = env.get_field(&metrics, "density", "F")?.f()?;
        let d = if d > 0.0 { d } else { 1.0 };
        DENSITY.store(d.to_bits(), Ordering::Relaxed);
        Ok(d)
    }

    fn system_dimen_px(env: &mut JNIEnv, activity: &JObject, name: &str) -> jni::errors::Result<i32> {
        let res = env
            .call_method(activity, "getResources", "()Landroid/content/res/Resources;", &[])?
            .l()?;
        let jname = env.new_string(name)?;
        let jtype = env.new_string("dimen")?;
        let jpkg = env.new_string("android")?;
        let id = env
            .call_method(
                &res,
                "getIdentifier",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
                &[(&jname).into(), (&jtype).into(), (&jpkg).into()],
            )?
            .i()?;
        if id <= 0 {
            return Ok(0);
        }
        env.call_method(&res, "getDimensionPixelSize", "(I)I", &[id.into()])?.i()
    }

    /// WindowInsets bottom for the given `WindowInsets.Type` mask (API 30+), in px.
    fn window_insets_bottom(env: &mut JNIEnv, activity: &JObject, type_mask: i32) -> jni::errors::Result<i32> {
        let wm = env
            .call_method(activity, "getWindowManager", "()Landroid/view/WindowManager;", &[])?
            .l()?;
        let metrics = env
            .call_method(&wm, "getCurrentWindowMetrics", "()Landroid/view/WindowMetrics;", &[])?
            .l()?;
        let insets = env
            .call_method(&metrics, "getWindowInsets", "()Landroid/view/WindowInsets;", &[])?
            .l()?;
        let typed = env
            .call_method(&insets, "getInsets", "(I)Landroid/graphics/Insets;", &[type_mask.into()])?
            .l()?;
        env.get_field(&typed, "bottom", "I")?.i()
    }

    pub fn query_status_bar_inset() -> f32 {
        with_activity(|env, act| {
            let px = system_dimen_px(env, act, "status_bar_height")?;
            let d = density(env, act)?;
            Ok(px as f32 / d)
        })
        .filter(|v| *v > 0.0)
        .unwrap_or(24.0)
    }

    pub fn query_navigation_bar_inset() -> f32 {
        with_activity(|env, act| {
            // 2 = WindowInsets.Type.navigationBars()
            let mut px = window_insets_bottom(env, act, 2).unwrap_or(0);
            if env.exception_check().unwrap_or(false) {
                env.exception_clear()?;
            }
            if px <= 0 {
                px = system_dimen_px(env, act, "navigation_bar_height").unwrap_or(0);
                if env.exception_check().unwrap_or(false) {
                    env.exception_clear()?;
                }
            }
            let d = density(env, act)?;
            Ok(px as f32 / d)
        })
        .filter(|v| *v > 0.0)
        .unwrap_or(16.0)
    }

    pub fn configure_window_soft_input_mode() {
        with_activity(|env, act| {
            let window = env.call_method(act, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
            // WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE = 0x10
            env.call_method(&window, "setSoftInputMode", "(I)V", &[16i32.into()])?;
            Ok(())
        });
    }

    fn build_string(env: &mut JNIEnv, field: &str) -> jni::errors::Result<String> {
        let class = env.find_class("android/os/Build")?;
        let val = env.get_static_field(class, field, "Ljava/lang/String;")?.l()?;
        Ok(get_string(env, val)?.unwrap_or_default())
    }

    pub fn is_tensor_g5_or_newer() -> bool {
        *IS_TENSOR_G5.get_or_init(|| {
            with_activity(|env, _| {
                let mut fields = Vec::new();
                for name in ["SOC_MODEL", "HARDWARE", "BOARD", "DEVICE"] {
                    // SOC_MODEL only exists on API 31+; ignore missing fields.
                    match build_string(env, name) {
                        Ok(v) => fields.push(v),
                        Err(_) => {
                            if env.exception_check().unwrap_or(false) {
                                env.exception_clear()?;
                            }
                        }
                    }
                }
                let refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
                Ok(soc_strings_indicate_tensor_g5_plus(&refs))
            })
            .unwrap_or(false)
        })
    }

    pub fn get_system_monet_colors() -> Option<MonetColors> {
        with_activity(|env, act| {
            let res = env
                .call_method(act, "getResources", "()Landroid/content/res/Resources;", &[])?
                .l()?;
            let jtype = env.new_string("color")?;
            let jpkg = env.new_string("android")?;

            let mut query = |name: &str| -> Option<(u8, u8, u8)> {
                let jname = env.new_string(name).ok()?;
                let id = env
                    .call_method(
                        &res,
                        "getIdentifier",
                        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
                        &[(&jname).into(), (&jtype).into(), (&jpkg).into()],
                    )
                    .ok()?
                    .i()
                    .ok()?;
                if id <= 0 {
                    return None;
                }
                let c = env.call_method(act, "getColor", "(I)I", &[id.into()]).ok()?.i().ok()?;
                Some((((c >> 16) & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, (c & 0xFF) as u8))
            };

            let Some(primary_light) = query("system_accent1_600").or_else(|| query("system_accent1_500")) else {
                return Ok(None);
            };
            let primary_dark = query("system_accent1_200")
                .or_else(|| query("system_accent1_300"))
                .unwrap_or(primary_light);
            let colors = MonetColors {
                primary_light,
                primary_dark,
                primary_container_light: query("system_accent1_100").unwrap_or(primary_dark),
                primary_container_dark: query("system_accent1_700").unwrap_or(primary_light),
                secondary_light: query("system_accent2_600")
                    .or_else(|| query("system_accent2_500"))
                    .unwrap_or(primary_light),
                secondary_dark: query("system_accent2_200").unwrap_or(primary_dark),
                tertiary_light: query("system_accent3_600")
                    .or_else(|| query("system_accent3_500"))
                    .unwrap_or(primary_light),
                tertiary_dark: query("system_accent3_200").unwrap_or(primary_dark),
                surface_light: query("system_neutral1_10")
                    .or_else(|| query("system_neutral1_50"))
                    .unwrap_or((249, 249, 255)),
                surface_dark: query("system_neutral1_900").unwrap_or((17, 19, 24)),
            };
            Ok(Some(colors))
        })
        .flatten()
    }

    /// Pin the display to its highest refresh rate *at the current resolution*, or hand
    /// control back to the system when `enabled` is false.
    ///
    /// Picking a mode purely by refresh rate (as before) could select a mode with a
    /// different resolution, forcing a mode switch and a blurry or janky first frames.
    pub fn apply_refresh_rate_setting(enabled: bool) {
        with_activity(|env, act| {
            let window = env.call_method(act, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
            let mut best_mode_id = 0i32;

            if enabled {
                let wm = env
                    .call_method(act, "getWindowManager", "()Landroid/view/WindowManager;", &[])?
                    .l()?;
                let display = env.call_method(&wm, "getDefaultDisplay", "()Landroid/view/Display;", &[])?.l()?;
                let current = env.call_method(&display, "getMode", "()Landroid/view/Display$Mode;", &[])?.l()?;
                let cur_w = env.call_method(&current, "getPhysicalWidth", "()I", &[])?.i()?;
                let cur_h = env.call_method(&current, "getPhysicalHeight", "()I", &[])?.i()?;

                let modes = env
                    .call_method(&display, "getSupportedModes", "()[Landroid/view/Display$Mode;", &[])?
                    .l()?;
                let modes = jni::objects::JObjectArray::from(modes);
                let count = env.get_array_length(&modes)?;
                let mut max_rate = 0.0f32;
                for i in 0..count {
                    let mode = env.get_object_array_element(&modes, i)?;
                    let w = env.call_method(&mode, "getPhysicalWidth", "()I", &[])?.i()?;
                    let h = env.call_method(&mode, "getPhysicalHeight", "()I", &[])?.i()?;
                    let rate = env.call_method(&mode, "getRefreshRate", "()F", &[])?.f()?;
                    if w == cur_w && h == cur_h && rate > max_rate {
                        max_rate = rate;
                        best_mode_id = env.call_method(&mode, "getModeId", "()I", &[])?.i()?;
                    }
                    env.delete_local_ref(mode)?;
                }
            }

            let attrs = env
                .call_method(&window, "getAttributes", "()Landroid/view/WindowManager$LayoutParams;", &[])?
                .l()?;
            env.set_field(&attrs, "preferredDisplayModeId", "I", JValue::Int(best_mode_id))?;
            env.call_method(
                &window,
                "setAttributes",
                "(Landroid/view/WindowManager$LayoutParams;)V",
                &[(&attrs).into()],
            )?;
            Ok(())
        });
    }

    pub fn haptic_tap() {
        with_activity(|env, act| {
            let window = env.call_method(act, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
            let decor = env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;
            // HapticFeedbackConstants.KEYBOARD_TAP = 3 (honours the system haptics setting)
            env.call_method(&decor, "performHapticFeedback", "(I)Z", &[3i32.into()])?;
            Ok(())
        });
    }

    fn clipboard_manager<'a>(env: &mut JNIEnv<'a>, act: &JObject) -> jni::errors::Result<JObject<'a>> {
        let name = env.new_string("clipboard")?;
        env.call_method(act, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[(&name).into()])?
            .l()
    }

    pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
        with_activity(|env, act| {
            let cm = clipboard_manager(env, act)?;
            if cm.is_null() {
                return Ok(false);
            }
            let label = env.new_string("DevKit")?;
            let clip_text = env.new_string(text)?;
            let clip = env
                .call_static_method(
                    "android/content/ClipData",
                    "newPlainText",
                    "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                    &[(&label).into(), (&clip_text).into()],
                )?
                .l()?;
            env.call_method(&cm, "setPrimaryClip", "(Landroid/content/ClipData;)V", &[(&clip).into()])?;
            Ok(true)
        })
        .filter(|ok| *ok)
        .map(|_| ())
        .ok_or_else(|| "Clipboard unavailable".to_string())
    }

    pub fn read_clipboard() -> Option<String> {
        with_activity(|env, act| {
            let cm = clipboard_manager(env, act)?;
            if cm.is_null() {
                return Ok(None);
            }
            let clip = env.call_method(&cm, "getPrimaryClip", "()Landroid/content/ClipData;", &[])?.l()?;
            if clip.is_null() {
                return Ok(None);
            }
            let count = env.call_method(&clip, "getItemCount", "()I", &[])?.i()?;
            if count <= 0 {
                return Ok(None);
            }
            let item = env
                .call_method(&clip, "getItemAt", "(I)Landroid/content/ClipData$Item;", &[0i32.into()])?
                .l()?;
            let seq = env
                .call_method(
                    &item,
                    "coerceToText",
                    "(Landroid/content/Context;)Ljava/lang/CharSequence;",
                    &[act.into()],
                )?
                .l()?;
            if seq.is_null() {
                return Ok(None);
            }
            let s = env.call_method(&seq, "toString", "()Ljava/lang/String;", &[])?.l()?;
            get_string(env, s)
        })
        .flatten()
    }

    fn inet_host(env: &mut JNIEnv, addr: &JObject) -> jni::errors::Result<Option<String>> {
        if addr.is_null() {
            return Ok(None);
        }
        let s = env.call_method(addr, "getHostAddress", "()Ljava/lang/String;", &[])?.l()?;
        get_string(env, s)
    }

    fn list_len(env: &mut JNIEnv, list: &JObject) -> jni::errors::Result<i32> {
        if list.is_null() {
            return Ok(0);
        }
        env.call_method(list, "size", "()I", &[])?.i()
    }

    fn list_get<'a>(env: &mut JNIEnv<'a>, list: &JObject, i: i32) -> jni::errors::Result<JObject<'a>> {
        env.call_method(list, "get", "(I)Ljava/lang/Object;", &[i.into()])?.l()
    }

    /// Query the active network through ConnectivityManager / LinkProperties.
    ///
    /// Works on Android 10+, where `/proc/net/route` is no longer readable by apps.
    pub fn query_network_info() -> Option<NetworkInfo> {
        with_activity(|env, act| {
            let name = env.new_string("connectivity")?;
            let cm = env
                .call_method(act, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[(&name).into()])?
                .l()?;
            if cm.is_null() {
                return Ok(None);
            }
            let net = env.call_method(&cm, "getActiveNetwork", "()Landroid/net/Network;", &[])?.l()?;
            if net.is_null() {
                return Ok(None);
            }
            let lp = env
                .call_method(
                    &cm,
                    "getLinkProperties",
                    "(Landroid/net/Network;)Landroid/net/LinkProperties;",
                    &[(&net).into()],
                )?
                .l()?;
            if lp.is_null() {
                return Ok(None);
            }

            let mut info = NetworkInfo::default();
            let iface = env.call_method(&lp, "getInterfaceName", "()Ljava/lang/String;", &[])?.l()?;
            info.interface = get_string(env, iface)?;

            let addrs = env.call_method(&lp, "getLinkAddresses", "()Ljava/util/List;", &[])?.l()?;
            for i in 0..list_len(env, &addrs)? {
                let la = list_get(env, &addrs, i)?;
                let addr = env.call_method(&la, "getAddress", "()Ljava/net/InetAddress;", &[])?.l()?;
                if let Some(host) = inet_host(env, &addr)? {
                    if !host.contains(':') {
                        let prefix = env.call_method(&la, "getPrefixLength", "()I", &[])?.i()?;
                        info.device_ip = Some(host);
                        info.prefix_len = u8::try_from(prefix).ok();
                        break;
                    }
                }
            }

            let routes = env.call_method(&lp, "getRoutes", "()Ljava/util/List;", &[])?.l()?;
            for i in 0..list_len(env, &routes)? {
                let route = list_get(env, &routes, i)?;
                let is_default = env.call_method(&route, "isDefaultRoute", "()Z", &[])?.z()?;
                if !is_default {
                    continue;
                }
                let gw = env.call_method(&route, "getGateway", "()Ljava/net/InetAddress;", &[])?.l()?;
                if let Some(host) = inet_host(env, &gw)? {
                    if !host.contains(':') && host != "0.0.0.0" {
                        info.gateway = Some(host);
                        break;
                    }
                }
            }

            let dns = env.call_method(&lp, "getDnsServers", "()Ljava/util/List;", &[])?.l()?;
            for i in 0..list_len(env, &dns)? {
                let a = list_get(env, &dns, i)?;
                if let Some(host) = inet_host(env, &a)? {
                    info.dns_servers.push(host);
                }
            }
            Ok(Some(info))
        })
        .flatten()
    }
}

#[cfg(target_os = "android")]
pub use imp::{
    apply_refresh_rate_setting, configure_window_soft_input_mode, copy_to_clipboard,
    get_system_monet_colors, haptic_tap, init, is_tensor_g5_or_newer, query_navigation_bar_inset,
    query_network_info, query_status_bar_inset, read_clipboard, with_activity,
};

// ---------------------------------------------------------------------------
// Desktop fallbacks
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "android"))]
mod desktop {
    use super::*;
    use std::sync::Mutex;

    /// In-process clipboard so copy/paste round-trips work while developing on desktop.
    static CLIPBOARD: Mutex<Option<String>> = Mutex::new(None);

    pub fn get_system_monet_colors() -> Option<MonetColors> {
        None
    }
    pub fn apply_refresh_rate_setting(_enabled: bool) {}
    pub fn is_tensor_g5_or_newer() -> bool {
        false
    }
    pub fn haptic_tap() {}
    pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
        if let Ok(mut c) = CLIPBOARD.lock() {
            *c = Some(text.to_string());
        }
        Ok(())
    }
    pub fn read_clipboard() -> Option<String> {
        CLIPBOARD.lock().ok().and_then(|c| c.clone())
    }
    pub fn query_network_info() -> Option<NetworkInfo> {
        None
    }
}

#[cfg(not(target_os = "android"))]
pub use desktop::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tensor_generations() {
        assert!(soc_strings_indicate_tensor_g5_plus(&["Tensor G5"]));
        assert!(soc_strings_indicate_tensor_g5_plus(&["tensor-g6"]));
        assert!(soc_strings_indicate_tensor_g5_plus(&["", "laguna"]));
        assert!(soc_strings_indicate_tensor_g5_plus(&["malibu"]));
        assert!(!soc_strings_indicate_tensor_g5_plus(&["Tensor G4", "caiman"]));
        assert!(!soc_strings_indicate_tensor_g5_plus(&["Tensor G3"]));
        assert!(!soc_strings_indicate_tensor_g5_plus(&["SM8650", "pineapple"]));
        // "img" used to match anything containing those letters (e.g. "pilgrim").
        assert!(!soc_strings_indicate_tensor_g5_plus(&["pilgrim"]));
    }

    #[test]
    fn auto_resolves_to_vulkan_on_desktop() {
        assert_eq!(resolve_graphics_api(0), GraphicsApi::Vulkan);
        assert_eq!(resolve_graphics_api(2), GraphicsApi::OpenGlEs);
        assert_eq!(resolve_graphics_api(1), GraphicsApi::Vulkan);
    }
}
