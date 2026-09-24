# DevKit

A fully-Rust Android developer toolbox built with Slint and the official Slint Material 3 component library. A "Dev Toolbox for GNOME, but on Android."

- **Application ID:** `it.mtreconsulting.devkit`
- **Crate Name:** `devkit`
- **Display Name:** `DevKit`
- **Current Status:** Scaffolding complete — navigation shell, responsive tool grid, and tool stubs implemented.

## Tech Stack

- **Language:** Rust (plus unavoidable JNI/Kotlin glue where Android requires it)
- **UI Framework:** [Slint](https://slint.dev/)
- **Design System:** Official Slint Material 3 components (`@material`)
- **Target:** Android (`arm64-v8a`, `armeabi-v7a`, `x86_64` for emulator)
- **Slint Backend:** `backend-android-activity-06`
- **Build System:** Cargo + `cargo-apk`
- **Minimum SDK:** 26 (Android 8.0)
- **Target SDK:** 35 (Android 15)

## Prerequisites

1. **Rust Toolchain:**
   Ensure `rustc` and `cargo` are installed. Add the Android targets:
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
   ```

2. **cargo-apk:**
   Install `cargo-apk` for building and packaging Android APKs:
   ```bash
   cargo install cargo-apk
   ```

3. **Android SDK & NDK:**
   - Set `ANDROID_HOME` pointing to your Android SDK directory (e.g., `~/Android/Sdk`).
   - Install Android SDK Platform 35 and Build-Tools.
   - Install Android NDK and set `ANDROID_NDK_ROOT` pointing to your NDK directory.
   - Ensure `JAVA_HOME` is set to JDK 17+ (e.g., Android Studio's bundled JBR at `/opt/android-studio/jbr`).

## Build Instructions

### Run on Desktop (Linux / macOS / Windows)

DevKit can run natively on desktop for rapid UI development and testing:

```bash
# Build binary
cargo build

# Run application
cargo run
```

### Check Android Rust Library Compilation

To verify that the Rust codebase and Slint components compile for Android target:

```bash
JAVA_HOME=/opt/android-studio/jbr ANDROID_HOME=$HOME/Android/Sdk \
cargo check --target aarch64-linux-android --lib
```

### Build Android APK

Build the debug APK using `cargo-apk`:

```bash
PATH=/opt/android-studio/jbr/bin:$PATH cargo apk build --target aarch64-linux-android --lib
```

### Install and Run on Android Device / Emulator

Ensure an Android device with USB debugging enabled (or an active emulator) is connected via `adb`:

```bash
PATH=/opt/android-studio/jbr/bin:$PATH cargo apk run --target aarch64-linux-android --lib
```

## Project Structure

```
devkit/
├── Cargo.toml
├── build.rs
├── .gitignore
├── README.md
├── ui/
│   ├── components/
│   │   ├── tool-card.slint       # Material Card with icon, title, description
│   │   └── section-header.slint  # Section title with Material typography
│   ├── screens/
│   │   ├── home.slint            # Responsive tool grid (2 cols phone, 3+ wide)
│   │   ├── subnet.slint          # Subnet Calculator stub
│   │   ├── hash.slint            # Hash Calculator stub
│   │   ├── base64.slint          # Base64 Encoder stub
│   │   ├── uuid.slint            # UUID Generator stub
│   │   ├── timestamp.slint       # Timestamp Converter stub
│   │   └── regex.slint           # Regex Tester stub
│   └── app.slint                 # Main application window & NavigationBar shell
├── src/
│   ├── main.rs                   # Desktop entry point
│   ├── lib.rs                    # Dual bin/lib root & android_main entry point
│   ├── app.rs                    # Slint app bootstrap & navigation event loop
│   ├── navigation.rs             # Screen routing enum and state
│   ├── tools/
│   │   ├── mod.rs                # Tool modules declaration
│   │   ├── subnet.rs             # Subnet calculator API stubs
│   │   ├── hash.rs               # Hash calculator API stubs
│   │   ├── base64.rs             # Base64 encoder API stubs
│   │   ├── uuid.rs               # UUID generator API stubs
│   │   ├── timestamp.rs          # Timestamp converter API stubs
│   │   └── regex.rs              # Regex tester API stubs
│   └── platform/
│       ├── mod.rs                # Platform conditional modules
│       └── android.rs            # JNI bridge stubs (system colors, font scale)
├── material/                     # Official Slint Material 3 component library
└── android/                      # Gradle build files & AndroidManifest.xml
    ├── build.gradle
    ├── settings.gradle
    ├── gradle.properties
    └── app/
        ├── build.gradle
        └── src/main/AndroidManifest.xml
```

## Implemented Tools (Stubs)

| Tool | Screen | Module | Status |
| --- | --- | --- | --- |
| **Subnet Calculator** | `ui/screens/subnet.slint` | `src/tools/subnet.rs` | Scaffolded |
| **Hash Calculator** | `ui/screens/hash.slint` | `src/tools/hash.rs` | Scaffolded |
| **Base64 Encoder** | `ui/screens/base64.slint` | `src/tools/base64.rs` | Scaffolded |
| **UUID Generator** | `ui/screens/uuid.slint` | `src/tools/uuid.rs` | Scaffolded |
| **Timestamp Converter** | `ui/screens/timestamp.slint` | `src/tools/timestamp.rs` | Scaffolded |
| **Regex Tester** | `ui/screens/regex.slint` | `src/tools/regex.rs` | Scaffolded |
