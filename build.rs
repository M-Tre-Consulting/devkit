// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 MTRE Consulting

use std::collections::HashMap;
use std::env;
use std::path::Path;

fn main() {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let material_path = Path::new(&manifest_dir).join("material/material.slint");

    let config = slint_build::CompilerConfiguration::new()
        .with_style("material".to_string())
        .with_library_paths(
            HashMap::from([("material".to_string(), material_path)]),
        );

    slint_build::compile_with_config("ui/app.slint", config).unwrap();

    // Ensure android:windowSoftInputMode="adjustResize" is present.
    // cargo-apk (via ndk-build) serializes [package.metadata.android.activity] window_soft_input_mode.
    // In addition, the manifest is hand-maintained in android/app/src/main/AndroidManifest.xml.
    // If cargo-apk or build steps leave target/<profile>/apk/AndroidManifest.xml without it,
    // we post-process any generated manifest files here.
    patch_generated_manifests(&manifest_dir);
}

fn patch_generated_manifests(manifest_dir: &std::ffi::OsStr) {
    for profile in ["debug", "release"] {
        let manifest_path = Path::new(manifest_dir)
            .join("target")
            .join(profile)
            .join("apk")
            .join("AndroidManifest.xml");
        if manifest_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                if !content.contains("android:windowSoftInputMode") {
                    let patched = content.replace(
                        "<activity ",
                        "<activity android:windowSoftInputMode=\"adjustResize\" ",
                    );
                    let _ = std::fs::write(&manifest_path, patched);
                }
            }
        }
    }
}
