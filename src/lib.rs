pub mod app;
pub mod navigation;

slint::include_modules!();

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: slint::android::AndroidApp) {
    app::android_main(android_app);
}
