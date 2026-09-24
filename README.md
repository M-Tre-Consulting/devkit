# DevKit

A fully-Rust Android developer toolbox built with Slint and the official Slint Material 3 component library. A "Dev Toolbox for GNOME, but on Android."

- **Application ID:** `it.mtreconsulting.devkit`
- **Crate Name:** `devkit`
- **Display Name:** `DevKit`
- **Status:** scaffolding

## Tech Stack

- **Language:** Rust (plus unavoidable JNI/Kotlin glue where Android requires it)
- **UI Framework:** [Slint](https://slint.dev/)
- **Design System:** Official Slint Material 3 components (`@material`)
- **Target:** Android (`arm64-v8a`, `armeabi-v7a`, `x86_64` for emulator)
- **Slint Backend:** `backend-android-activity-06`
- **Build System:** Cargo + `cargo-apk`
- **Minimum SDK:** 26 (Android 8.0)
- **Target SDK:** 35 (Android 15)
