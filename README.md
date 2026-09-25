# DevKit

A Rust + Slint dev toolbox for Android, built to test whether Slint is a viable UI framework for mobile.

Status: Experimental — functional prototype, not production-ready.

## What this is

DevKit is a fully-Rust Android application using Slint for its user interface. It provides a developer toolbox covering common utility workflows: a subnet calculator, hash calculator, base64 encoder/decoder, UUID generator, timestamp converter, and regex tester. The project exists primarily as an empirical testbed for Slint on Android to evaluate rendering performance, Material 3 design fidelity, animation capabilities, and the ergonomics of writing a complete mobile UI in Rust. The developer tool concepts serve as a realistic workload to test these properties, rather than being the end goal themselves.

## What this is not

- Not production-ready.
- Not a polished replacement for existing dev tool applications.
- Not affiliated with Google, GNOME, or the Slint project.
- Not a general statement about whether Rust should be used for mobile apps, but an inquiry into whether Slint specifically is ready for that role.

## Why it exists

Most existing Rust-on-Android architectures rely on a Kotlin UI layer built with Jetpack Compose, relegating Rust to background logic communicated over JNI bindings. Slint is one of the few frameworks attempting a unified, fully-Rust UI stack on mobile without JVM-side UI frameworks. DevKit exists to evaluate whether writing a native-feeling mobile interface entirely in Rust and Slint is practical, whether the layout and animation ergonomics hold up, and whether the resulting application achieves acceptable runtime performance on real hardware.

## Tech stack

- **Language:** Rust (2021 edition)
- **UI Framework:** [Slint](https://slint.dev)
- **Design System:** Slint Material 3 component library
- **Android Runtime:** `android-activity` (`backend-android-activity-06`) via `cargo-apk`
- **Minimum SDK:** 26 (Android 8.0)
- **Target SDK:** 35 (Android 15)

## Screenshots

Screenshots will be added as visual polish and component styling stabilize.

<!-- TODO: add home, tool, and pill screenshots -->

## Building and running

### Prerequisites

- Rust stable toolchain with Android targets added (`rustup target add aarch64-linux-android`)
- `cargo-apk` installed (`cargo install cargo-apk`)
- Android SDK (API 35 platform and build-tools) and NDK (r26+)
- A JDK 17+ installation (e.g., Android Studio's bundled JBR)

Supported target architectures: `aarch64-linux-android`, `armeabi-v7a`, `x86_64`.

### Run command

To compile and launch directly on a connected device or running emulator:

```bash
PATH=/opt/android-studio/jbr/bin:$PATH cargo apk run --target aarch64-linux-android --lib
```

> Note: Setting `JAVA_HOME` alone is not sufficient because `apksigner` requires the `java` binary to be present directly in `PATH`.

## Project structure

```
ui/                      Slint UI markup, components, and screen layouts
├── components/          Shared UI components (floating pill nav bar, tool cards, headers)
└── screens/             Screen definitions for modes and tool placeholder views
src/                     Rust application backend and lifecycle glue
├── platform/            Android JNI bridge and native system integration
└── tools/               Tool interface definitions and computation stubs
```

## Roadmap

- Implement actual calculation and conversion logic across all tool modules
- Query and apply dynamic color tokens (Monet) from system wallpaper
- Incorporate Material 3 Expressive motion curves and container transforms
- Support adaptive layouts for foldable and tablet screen form factors
- Add unit tests for tool computation and UI state models

## License

License: TBD

## Acknowledgements

- The [Slint](https://slint.dev) project team and the authors of the Slint Material 3 component library.
- Material Design 3 is a design system developed by Google and used here under its public specifications.
