// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 MTRE Consulting

use std::collections::HashMap;
use std::env;
use std::path::Path;

fn main() {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let material_path = Path::new(&manifest_dir).join("material/material.slint");

    let config = slint_build::CompilerConfiguration::new().with_library_paths(
        HashMap::from([("material".to_string(), material_path)]),
    );

    slint_build::compile_with_config("ui/app.slint", config).unwrap();
}
