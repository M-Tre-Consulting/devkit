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

/// Input permission scope: either a permission matrix or raw octal string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChmodInput {
    Matrix {
        owner: PermissionBits,
        group: PermissionBits,
        other: PermissionBits,
    },
    Octal(String),
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
/// Computes octal value (e.g. "0755"), symbolic string (e.g. "-rwxr-xr-x"), and command.
pub fn calculate_permissions(input: &ChmodInput) -> Result<ChmodOutput, ChmodError> {
    let matrix_input = match input {
        ChmodInput::Matrix { .. } => input.clone(),
        ChmodInput::Octal(raw) => parse_octal(raw)?,
    };

    let octal_val = compue_octal(&matrix_input);
    let octal = format!("0{:03o}", octal_val);
    let symbolic = format!("-{}", compute_string(&matrix_input));
    let command = format!("chmod {:03o} <file>", octal_val);

    Ok(ChmodOutput {
        octal,
        symbolic,
        command,
    })
}

/// Computes the full permissions string.
///
/// Accepts a ChmodInput structure.
fn compute_string(input: &ChmodInput) -> String {
    let (owner, group, other) = match input {
        ChmodInput::Matrix { owner, group, other } => (owner, group, other),
        ChmodInput::Octal(_) => unreachable!(),
    };
    let user = compute_string_group(owner);
    let group = compute_string_group(group);
    let other = compute_string_group(other);

    format!("{}{}{}", user, group, other)
}

/// Computes the string from each bit group.
///
/// Accepts a PermissionBits structure.
fn compute_string_group(bits: &PermissionBits) -> String {
    let read = if bits.read { 'r' } else { '-' };
    let write = if bits.write { 'w' } else { '-' };
    let execute = if bits.execute { 'x' } else { '-' };

    format!("{}{}{}", read, write, execute)
}

/// Computes the octal permissions mask from a Chmod value.
///
/// Accepts ChmodInput structure.
fn compue_octal(input: &ChmodInput) -> u32 {
    let (owner, group, other) = match input {
        ChmodInput::Matrix { owner, group, other } => (owner, group, other),
        ChmodInput::Octal(_) => unreachable!(),
    };
    let user = compute_bit_group(owner);
    let group = compute_bit_group(group);
    let other = compute_bit_group(other);

    (user << 6) | (group << 3) | other
}

/// Computes the octal value from raw boolean bits.
///
/// Accepts PermissionBits structure.
fn compute_bit_group(bits: &PermissionBits) -> u32 {
    let mut value = 0;

    if bits.read {
        value |= 0o4;
    }

    if bits.write {
        value |= 0o2;
    }

    if bits.execute {
        value |= 0o1;
    }

    value
}

/// Parses an octal string (e.g. "755" or "0755") into permission bits.
///
/// Parses each octal digit into read/write/execute bits.
pub fn parse_octal(octal: &str) -> Result<ChmodInput, ChmodError> {
    let trimmed = octal.trim();
    if trimmed.is_empty() {
        return Err(ChmodError::InvalidOctal("Input cannot be empty".to_string()));
    }

    let permissions =
        u32::from_str_radix(trimmed, 8).map_err(|e| ChmodError::InvalidOctal(e.to_string()))?;

    // Get bits for each permission category
    let user = (permissions >> 6) & 0o7;
    let group = (permissions >> 3) & 0o7;
    let others = permissions & 0o7;

    // Create structured result
    Ok(ChmodInput::Matrix {
        owner: parse_bits(user),
        group: parse_bits(group),
        other: parse_bits(others),
    })
}

/// Parses a symbolic string (e.g. "-rwxr-xr-x" or "rwxr-xr-x") into permission bits.
///
/// Parses 9 permission characters into read/write/execute bits.
pub fn parse_symbolic(symbolic: &str) -> Result<ChmodInput, ChmodError> {
    // Normalises the input permissions string
    let permissions = if symbolic.starts_with("-") {
        &symbolic[1..]
    } else {
        symbolic
    };

    if permissions.len() != 9 {
        return Err(ChmodError::InvalidSymbolic(
            "Symbolic string must contain exactly 9 or 10 characters".to_string(),
        ));
    }

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

    Ok(ChmodInput::Matrix {
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
        let input = ChmodInput::Matrix {
            owner: PermissionBits::ALL,
            group: PermissionBits::READ_EXEC,
            other: PermissionBits::READ_EXEC,
        };
        let out = calculate_permissions(&input).unwrap();
        assert_eq!(out.octal, "0755");
        assert_eq!(out.symbolic, "-rwxr-xr-x");
        assert_eq!(out.command, "chmod 755 <file>");
    }

    #[test]
    fn test_calculate_matrix_644() {
        let input = ChmodInput::Matrix {
            owner: PermissionBits::READ_WRITE,
            group: PermissionBits::READ_ONLY,
            other: PermissionBits::READ_ONLY,
        };
        let out = calculate_permissions(&input).unwrap();
        assert_eq!(out.octal, "0644");
        assert_eq!(out.symbolic, "-rw-r--r--");
        assert_eq!(out.command, "chmod 644 <file>");
    }

    #[test]
    fn test_calculate_matrix_600() {
        let input = ChmodInput::Matrix {
            owner: PermissionBits::READ_WRITE,
            group: PermissionBits::NONE,
            other: PermissionBits::NONE,
        };
        let out = calculate_permissions(&input).unwrap();
        assert_eq!(out.octal, "0600");
        assert_eq!(out.symbolic, "-rw-------");
        assert_eq!(out.command, "chmod 600 <file>");
    }

    #[test]
    fn test_calculate_matrix_777() {
        let input = ChmodInput::Matrix {
            owner: PermissionBits::ALL,
            group: PermissionBits::ALL,
            other: PermissionBits::ALL,
        };
        let out = calculate_permissions(&input).unwrap();
        assert_eq!(out.octal, "0777");
        assert_eq!(out.symbolic, "-rwxrwxrwx");
        assert_eq!(out.command, "chmod 777 <file>");
    }

    #[test]
    fn test_calculate_matrix_000() {
        let input = ChmodInput::Matrix {
            owner: PermissionBits::NONE,
            group: PermissionBits::NONE,
            other: PermissionBits::NONE,
        };
        let out = calculate_permissions(&input).unwrap();
        assert_eq!(out.octal, "0000");
        assert_eq!(out.symbolic, "----------");
        assert_eq!(out.command, "chmod 000 <file>");
    }

    #[test]
    fn test_calculate_octal_755() {
        let input = ChmodInput::Octal("755".to_string());
        let out = calculate_permissions(&input).unwrap();
        assert_eq!(out.octal, "0755");
        assert_eq!(out.symbolic, "-rwxr-xr-x");
        assert_eq!(out.command, "chmod 755 <file>");
    }

    #[test]
    fn test_calculate_octal_leading_zero() {
        let input = ChmodInput::Octal("0755".to_string());
        let out = calculate_permissions(&input).unwrap();
        assert_eq!(out.octal, "0755");
        assert_eq!(out.symbolic, "-rwxr-xr-x");
        assert_eq!(out.command, "chmod 755 <file>");
    }

    #[test]
    fn test_calculate_octal_644() {
        let input = ChmodInput::Octal("644".to_string());
        let out = calculate_permissions(&input).unwrap();
        assert_eq!(out.octal, "0644");
        assert_eq!(out.symbolic, "-rw-r--r--");
        assert_eq!(out.command, "chmod 644 <file>");
    }

    #[test]
    fn test_calculate_octal_600() {
        let input = ChmodInput::Octal("600".to_string());
        let out = calculate_permissions(&input).unwrap();
        assert_eq!(out.octal, "0600");
        assert_eq!(out.symbolic, "-rw-------");
        assert_eq!(out.command, "chmod 600 <file>");
    }

    #[test]
    fn test_calculate_octal_whitespace() {
        let input = ChmodInput::Octal("  755 \n".to_string());
        let out = calculate_permissions(&input).unwrap();
        assert_eq!(out.octal, "0755");
        assert_eq!(out.symbolic, "-rwxr-xr-x");
    }

    #[test]
    fn test_calculate_octal_invalid_digits() {
        let input = ChmodInput::Octal("789".to_string());
        let err = calculate_permissions(&input).unwrap_err();
        assert!(matches!(err, ChmodError::InvalidOctal(_)));
    }

    #[test]
    fn test_calculate_octal_invalid_letters() {
        let input = ChmodInput::Octal("abc".to_string());
        let err = calculate_permissions(&input).unwrap_err();
        assert!(matches!(err, ChmodError::InvalidOctal(_)));
    }

    #[test]
    fn test_calculate_octal_empty() {
        let input = ChmodInput::Octal("".to_string());
        let err = calculate_permissions(&input).unwrap_err();
        assert!(matches!(err, ChmodError::InvalidOctal(_)));
    }

    #[test]
    fn test_parse_octal_644() {
        let input = parse_octal("644").unwrap();
        match input {
            ChmodInput::Matrix { owner, group, other } => {
                assert!(owner.read && owner.write && !owner.execute);
                assert!(group.read && !group.write && !group.execute);
                assert!(other.read && !other.write && !other.execute);
            }
            _ => panic!("Expected matrix output from parse_octal"),
        }
    }

    #[test]
    fn test_parse_symbolic_valid_9_chars() {
        let input = parse_symbolic("rwxr-xr-x").unwrap();
        match input {
            ChmodInput::Matrix { owner, group, other } => {
                assert_eq!(owner, PermissionBits::ALL);
                assert_eq!(group, PermissionBits::READ_EXEC);
                assert_eq!(other, PermissionBits::READ_EXEC);
            }
            _ => panic!("Expected matrix output"),
        }
    }

    #[test]
    fn test_parse_symbolic_valid_10_chars() {
        let input = parse_symbolic("-rw-r--r--").unwrap();
        match input {
            ChmodInput::Matrix { owner, group, other } => {
                assert_eq!(owner, PermissionBits::READ_WRITE);
                assert_eq!(group, PermissionBits::READ_ONLY);
                assert_eq!(other, PermissionBits::READ_ONLY);
            }
            _ => panic!("Expected matrix output"),
        }
    }

    #[test]
    fn test_parse_symbolic_invalid_length() {
        assert!(matches!(
            parse_symbolic("rwx"),
            Err(ChmodError::InvalidSymbolic(_))
        ));
    }

    #[test]
    fn test_parse_symbolic_invalid_chars() {
        assert!(matches!(
            parse_symbolic("rwxr-xr-z"),
            Err(ChmodError::InvalidSymbolic(_))
        ));
    }

    #[test]
    fn test_permission_bits_constants() {
        assert_eq!(
            PermissionBits::NONE,
            PermissionBits {
                read: false,
                write: false,
                execute: false,
            }
        );
        assert_eq!(
            PermissionBits::READ_ONLY,
            PermissionBits {
                read: true,
                write: false,
                execute: false,
            }
        );
        assert_eq!(
            PermissionBits::READ_WRITE,
            PermissionBits {
                read: true,
                write: true,
                execute: false,
            }
        );
        assert_eq!(
            PermissionBits::READ_EXEC,
            PermissionBits {
                read: true,
                write: false,
                execute: true,
            }
        );
        assert_eq!(
            PermissionBits::ALL,
            PermissionBits {
                read: true,
                write: true,
                execute: true,
            }
        );
    }

    #[test]
    fn test_fuzz_all_512_octal_combinations() {
        for octal_num in 0..=0o777 {
            let octal_str = format!("{:03o}", octal_num);
            let octal_input = ChmodInput::Octal(octal_str.clone());
            let out_from_octal = calculate_permissions(&octal_input)
                .expect(&format!("Failed for octal {octal_str}"));
            assert_eq!(out_from_octal.octal, format!("0{octal_str}"));

            // Parse via parse_octal to test roundtrip with Matrix
            let matrix_input = parse_octal(&octal_str).unwrap();
            let out_from_matrix = calculate_permissions(&matrix_input)
                .expect(&format!("Failed for matrix {octal_str}"));

            assert_eq!(out_from_octal.octal, out_from_matrix.octal);
            assert_eq!(out_from_octal.symbolic, out_from_matrix.symbolic);
            assert_eq!(out_from_octal.command, out_from_matrix.command);
        }
    }
}
