// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 MTRE Consulting

pub mod app;
pub mod navigation;
pub mod platform;
pub mod storage;
pub mod tools;

slint::include_modules!();

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: slint::android::AndroidApp) {
    app::android_main(android_app);
}
