// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 MTRE Consulting

//! Platform-specific modules.

#[cfg(target_os = "android")]
pub mod android;

#[cfg(not(target_os = "android"))]
pub mod android;
