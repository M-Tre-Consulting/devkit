// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Chmod Calculator module stub.
//!
//! Provides calculation of UNIX permission bits (octal numeric, symbolic notation,
//! and command formatting) and parsing from octal and symbolic strings.

/// Three-bit permission flags (read, write, execute).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PermissionBits {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl PermissionBits {
    pub const NONE: Self = Self {
        read: false,
        write: false,
        execute: false,
    };
    pub const READ_ONLY: Self = Self {
        read: true,
        write: false,
        execute: false,
    };
    pub const READ_WRITE: Self = Self {
        read: true,
        write: true,
        execute: false,
    };
    pub const READ_EXEC: Self = Self {
        read: true,
        write: false,
        execute: true,
    };
    pub const ALL: Self = Self {
        read: true,
        write: true,
        execute: true,
    };
}

/// Input permission scope matrix: Owner, Group, Other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChmodInput {
    pub owner: PermissionBits,
    pub group: PermissionBits,
    pub other: PermissionBits,
}

/// Calculated chmod representations and shell commands.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChmodOutput {
    pub octal: String,
    pub symbolic: String,
    pub command: String,
}

/// Errors that can occur during permission parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChmodError {
    InvalidOctal(String),
    InvalidSymbolic(String),
}

impl std::fmt::Display for ChmodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidOctal(val) => write!(f, "Invalid octal permission string: '{val}'"),
            Self::InvalidSymbolic(val) => write!(f, "Invalid symbolic permission string: '{val}'"),
        }
    }
}

impl std::error::Error for ChmodError {}

/// Calculate numeric octal value and symbolic string representation for file permissions.
///
/// TODO: Compute octal value (e.g. "0755") and symbolic string (e.g. "-rwxr-xr-x").
pub fn calculate_permissions(input: &ChmodInput) -> Result<ChmodOutput, ChmodError> {
    todo!("calculate chmod permissions")
}

/// Computes the full permissions string.
///
/// Accepts a ChmodInput structure.
fn compute_string(input: &ChmodInput) -> String {
    let user = compute_string_group(&input.owner);
    let group = compute_string_group(&input.group);
    let other = compute_string_group(&input.other);

    format!("{}{}{}", user, group, other)
}

/// Computes the string from each bit group.
///
/// Accepts a PermissionBits structure.
fn compute_string_group(bits: &PermissionBits) -> String {
    let mut value = String::new();

    let read = if bits.read { 'r' } else { '-' };
    let write = if bits.write { 'w' } else { '-' };
    let execute = if bits.execute { 'x' } else { '-' };

    format!("{}{}{}", read, write, execute)
}

/// Computes the octal permissions mask from a Chmod value.
///
/// Accepts ChmodInput structure.
fn compue_octal(input: &ChmodInput) -> u32 {
    let user = compute_bit_group(&input.owner);
    let group = compute_bit_group(&input.group);
    let other = compute_bit_group(&input.other);

    (user << 6) | (group << 3) | other
}

/// Computes the octal value from raw boolean bits.
///
/// Accepts PermissionBits structure.
fn compute_bit_group(bits: &PermissionBits) -> u32 {
    let mut value = 0;

    if bits.read {
        value != 0o4;
    }

    if bits.write {
        value != 0o2;
    }

    if bits.execute {
        value != 0o1;
    }

    value
}

/// Parses an octal string (e.g. "755" or "0755") into permission bits.
///
/// Parses each octal digit into read/write/execute bits.
fn parse_octal(octal: &str) -> Result<ChmodInput, ChmodError> {
    let permissions =
        u32::from_str_radix(octal, 8).map_err(|e| ChmodError::InvalidOctal(e.to_string()))?;

    // Get bits for each permission category
    let user = (permissions >> 6) & 0o7;
    let group = (permissions >> 3) & 0o7;
    let others = permissions & 0o7;

    // Create structured result
    Ok(ChmodInput {
        owner: parse_bits(user),
        group: parse_bits(group),
        other: parse_bits(others),
    })
}

/// Parses a symbolic string (e.g. "-rwxr-xr-x" or "rwxr-xr-x") into permission bits.
///
/// Parses 9 permission characters into read/write/execute bits.
fn parse_symbolic(symbolic: &str) -> Result<ChmodInput, ChmodError> {
    // Normalises the input permissions string
    let permissions = if symbolic.starts_with("-") {
        &symbolic[1..]
    } else {
        symbolic
    };

    // Check if the string contains characters that are not allowed
    if permissions
        .chars()
        .any(|c| !matches!(c, 'r' | 'w' | 'x' | '-'))
    {
        return Err(ChmodError::InvalidSymbolic(
            "Invalid symbolic string input".to_string(),
        ));
    }

    let user: &str = &permissions[0..3];
    let group: &str = &permissions[3..6];
    let others: &str = &permissions[6..9];

    Ok(ChmodInput {
        owner: parse_string(user),
        group: parse_string(group),
        other: parse_string(others),
    })
}

/// Parses permission bits and returns a structured output.
///
/// Accepts permissions bitmask as u32.
fn parse_bits(bitmask: u32) -> PermissionBits {
    PermissionBits {
        read: (bitmask & 0o4) != 0,
        write: (bitmask & 0o2) != 0,
        execute: (bitmask & 0o1) != 0,
    }
}

/// Parses a permissions string and returns a structured output.
///
/// Accepts a permissions string (rwx).
fn parse_string(permissions: &str) -> PermissionBits {
    let bytes = permissions.as_bytes();

    PermissionBits {
        read: bytes[0] == b'r',
        write: bytes[1] == b'w',
        execute: bytes[2] == b'x',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_755() {
        let input = ChmodInput {
            owner: PermissionBits::ALL,
            group: PermissionBits::READ_EXEC,
            other: PermissionBits::READ_EXEC,
        };
        let out = calculate_permissions(&input).unwrap();
        assert_eq!(out.octal, "0755");
        assert_eq!(out.symbolic, "-rwxr-xr-x");
    }

    #[test]
    fn test_parse_octal_644() {
        let input = parse_octal("644").unwrap();
        assert!(input.owner.read && input.owner.write && !input.owner.execute);
        assert!(input.group.read && !input.group.write && !input.group.execute);
        assert!(input.other.read && !input.other.write && !input.other.execute);
    }
}
