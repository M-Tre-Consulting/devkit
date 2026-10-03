// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 MTRE Consulting

//! Subnet Calculator module stub.
//!
//! Provides IPv4 and IPv6 CIDR network calculation, address range estimation,
//! and netmask conversions.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Input parameters for subnet calculation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubnetInput {
    pub ip_or_cidr: String,
    pub prefix: Option<u8>,
}

/// Detailed calculation output for a subnet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SubnetOutput {
    pub network_address: String,
    pub broadcast_address: String,
    pub first_host: String,
    pub last_host: String,
    pub host_count: String,
    pub netmask: String,
    pub wildcard_mask: String,
    pub cidr_notation: String,
}

/// Errors that can occur during subnet parsing and calculation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubnetError {
    InvalidFormat,
    InvalidPrefix,
    InvalidIp,
}

impl std::fmt::Display for SubnetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubnetError::InvalidFormat => write!(f, "Invalid subnet format"),
            SubnetError::InvalidPrefix => write!(f, "Invalid prefix length"),
            SubnetError::InvalidIp => write!(f, "Invalid IP address"),
        }
    }
}

impl std::error::Error for SubnetError {}

/// Calculates network, broadcast, host range, and masks from IP and prefix input.
///
pub fn calculate(_input: &SubnetInput) -> Result<SubnetOutput, SubnetError> {
    todo!("calculate subnet details from input")
}

/// Parses an IPv4 address from string.
fn parse_ipv4_address(input: &str) -> Result<Ipv4Addr, SubnetError> {
    input
        .trim()
        .parse::<Ipv4Addr>()
        .map_err(|_| SubnetError::InvalidIp)
}

/// Parses an IPv6 address from string.
fn parse_ipv6_address(input: &str) -> Result<Ipv6Addr, SubnetError> {
    input
        .trim()
        .parse::<Ipv6Addr>()
        .map_err(|_| SubnetError::InvalidIp)
}

/// Parses an IPv4 or IPv6 address from string.
fn parse_ip_address(input: &str) -> Result<IpAddr, SubnetError> {
    input
        .trim()
        .parse::<IpAddr>()
        .map_err(|_| SubnetError::InvalidIp)
}

/// Parses an IPv4 prefix (CIDR value 0..=32) from string.
fn parse_prefix(input: &str) -> Result<u8, SubnetError> {
    input
        .trim()
        .parse::<u8>()
        .ok()
        .filter(|p| *p <= 32)
        .ok_or(SubnetError::InvalidPrefix)
}

/// Parses an IPv6 prefix (CIDR value 0..=128) from string.
fn parse_prefix_v6(input: &str) -> Result<u8, SubnetError> {
    input
        .trim()
        .parse::<u8>()
        .ok()
        .filter(|p| *p <= 128)
        .ok_or(SubnetError::InvalidPrefix)
}

/// Returns the bitmask version of the netmask calculated from the input prefix value.
///
/// Accepts prefix as a u8 integer value.
fn get_netmask(prefix: u8) -> u32 {
    if prefix == 0 {
        return 0;
    }

    !0u32 << (32 - prefix)
}

/// Returns the wildcard mask, which is the inverse of the netmask.
///
/// Accepts the netmask as u32 integer value.
fn get_wildcard_mask(netmask: u32) -> u32 {
    !netmask
}

/// Calculates and returns the network address by bitwise ANDing IP and
/// netmask values.
///
/// Accepts netmask and IP address as u32 integer values.
fn get_network_address(netmask: u32, ip_value: u32) -> u32 {
    netmask & ip_value
}

/// Calculates and returns the broadcast address by bitwise ORing the
/// network address and the corresponding wildcard mask.
///
/// Accepts network address and wildcard mask as u32 integer values.
fn get_broadcast_address(net_address: u32, wildcard_mask: u32) -> u32 {
    net_address | wildcard_mask
}

/// Contains the result of the host counts calculation
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct HostCount {
    first_host: u32,
    last_host: u32,
    bcast_addr: u32,
    host_count: u32,
}

/// Calculates and returns the host count and other useful parameters starting from
/// network address, broadcast address and prefix value.
///
/// Accepts network address as u32, broadcast address as u32 and prefix as u8.
fn get_host_counts(net_addr: u32, bcast_addr: u32, prefix: u8) -> Result<HostCount, SubnetError> {
    match prefix {
        0..=30 => Ok(HostCount {
            first_host: net_addr + 1,
            last_host: bcast_addr - 1,
            bcast_addr: bcast_addr,
            host_count: ((1u64 << (32 - prefix)) - 2) as u32,
        }),
        31 => Ok(HostCount {
            first_host: net_addr,
            last_host: bcast_addr,
            bcast_addr: bcast_addr,
            host_count: 2,
        }),
        32 => Ok(HostCount {
            first_host: net_addr,
            last_host: net_addr,
            bcast_addr: net_addr,
            host_count: 1,
        }),
        _ => Err(SubnetError::InvalidPrefix),
    }
}

/// Splits and validates an IPv4 address and prefix length from input.
///
/// Accepts CIDR notation (e.g. "192.168.1.1/24") or separate prefix in `SubnetInput`.
fn split_input(input: &SubnetInput) -> Result<(Ipv4Addr, u8), SubnetError> {
    let trimmed = input.ip_or_cidr.trim();
    if trimmed.is_empty() {
        return Err(SubnetError::InvalidFormat);
    }

    let parts: Vec<&str> = trimmed.split('/').collect();
    match parts.as_slice() {
        [ip_str] => {
            let ip = parse_ipv4_address(ip_str)?;
            let prefix = input
                .prefix
                .ok_or(SubnetError::InvalidPrefix)
                .and_then(|p| {
                    if p <= 32 {
                        Ok(p)
                    } else {
                        Err(SubnetError::InvalidPrefix)
                    }
                })?;
            Ok((ip, prefix))
        }
        [ip_str, prefix_str] => {
            if ip_str.trim().is_empty() || prefix_str.trim().is_empty() {
                return Err(SubnetError::InvalidFormat);
            }
            let ip = parse_ipv4_address(ip_str)?;
            let prefix = parse_prefix(prefix_str)?;
            Ok((ip, prefix))
        }
        _ => Err(SubnetError::InvalidFormat),
    }
}

/// Splits and validates an IPv6 address and prefix length from input.
///
/// Accepts CIDR notation (e.g. "2001:db8::1/64") or separate prefix in `SubnetInput`.
fn split_input_v6(input: &SubnetInput) -> Result<(Ipv6Addr, u8), SubnetError> {
    let trimmed = input.ip_or_cidr.trim();
    if trimmed.is_empty() {
        return Err(SubnetError::InvalidFormat);
    }

    let parts: Vec<&str> = trimmed.split('/').collect();
    match parts.as_slice() {
        [ip_str] => {
            let ip = parse_ipv6_address(ip_str)?;
            let prefix = input
                .prefix
                .ok_or(SubnetError::InvalidPrefix)
                .and_then(|p| {
                    if p <= 128 {
                        Ok(p)
                    } else {
                        Err(SubnetError::InvalidPrefix)
                    }
                })?;
            Ok((ip, prefix))
        }
        [ip_str, prefix_str] => {
            if ip_str.trim().is_empty() || prefix_str.trim().is_empty() {
                return Err(SubnetError::InvalidFormat);
            }
            let ip = parse_ipv6_address(ip_str)?;
            let prefix = parse_prefix_v6(prefix_str)?;
            Ok((ip, prefix))
        }
        _ => Err(SubnetError::InvalidFormat),
    }
}

/// Splits and validates either an IPv4 or IPv6 address and prefix length from input.
fn split_input_any(input: &SubnetInput) -> Result<(IpAddr, u8), SubnetError> {
    let trimmed = input.ip_or_cidr.trim();
    if trimmed.is_empty() {
        return Err(SubnetError::InvalidFormat);
    }

    let parts: Vec<&str> = trimmed.split('/').collect();
    match parts.as_slice() {
        [ip_str] => {
            let ip = parse_ip_address(ip_str)?;
            let max_prefix = match ip {
                IpAddr::V4(_) => 32,
                IpAddr::V6(_) => 128,
            };
            let prefix = input
                .prefix
                .ok_or(SubnetError::InvalidPrefix)
                .and_then(|p| {
                    if p <= max_prefix {
                        Ok(p)
                    } else {
                        Err(SubnetError::InvalidPrefix)
                    }
                })?;
            Ok((ip, prefix))
        }
        [ip_str, prefix_str] => {
            if ip_str.trim().is_empty() || prefix_str.trim().is_empty() {
                return Err(SubnetError::InvalidFormat);
            }
            let ip = parse_ip_address(ip_str)?;
            let prefix = match ip {
                IpAddr::V4(_) => parse_prefix(prefix_str)?,
                IpAddr::V6(_) => parse_prefix_v6(prefix_str)?,
            };
            Ok((ip, prefix))
        }
        _ => Err(SubnetError::InvalidFormat),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_netmask() {
        // Empty netmask
        assert_eq!(get_netmask(0), 0);

        // Standard netmasks
        assert_eq!(get_netmask(8), 0b11111111_00000000_00000000_00000000u32);
        assert_eq!(get_netmask(16), 0b11111111_11111111_00000000_00000000u32);
        assert_eq!(get_netmask(24), 0b11111111_11111111_11111111_00000000u32);

        // Non-standard netmasks
        assert_eq!(get_netmask(22), 0b11111111_11111111_11111100_00000000u32);
        assert_eq!(get_netmask(21), 0b11111111_11111111_11111000_00000000u32);
        assert_eq!(get_netmask(14), 0b11111111_11111100_00000000_00000000u32);
    }

    #[test]
    fn test_get_wildcard_mask() {
        // Empty netmask
        let empty_mask = get_netmask(0u8);
        assert_eq!(get_wildcard_mask(empty_mask), !0u32);

        // Standard netmasks
        let class_a = get_netmask(8u8);
        assert_eq!(
            get_wildcard_mask(class_a),
            !(0b11111111_00000000_00000000_00000000u32)
        );

        let class_b = get_netmask(16u8);
        assert_eq!(
            get_wildcard_mask(class_b),
            !(0b11111111_11111111_00000000_00000000u32)
        );

        // Non-standard netmasks
        let nmp_21 = get_netmask(21);
        assert_eq!(
            get_wildcard_mask(nmp_21),
            !(0b11111111_11111111_11111000_00000000u32)
        );
    }

    #[test]
    fn test_get_host_counts() {
        // Standard /24 subnet (192.168.1.0/24)
        let net = u32::from(Ipv4Addr::new(192, 168, 1, 0));
        let bcast = u32::from(Ipv4Addr::new(192, 168, 1, 255));
        let res = get_host_counts(net, bcast, 24).unwrap();
        assert_eq!(res.first_host, u32::from(Ipv4Addr::new(192, 168, 1, 1)));
        assert_eq!(res.last_host, u32::from(Ipv4Addr::new(192, 168, 1, 254)));
        assert_eq!(res.bcast_addr, bcast);
        assert_eq!(res.host_count, 254);

        // Standard /30 subnet (10.0.0.4/30) - 2 usable hosts
        let net30 = u32::from(Ipv4Addr::new(10, 0, 0, 4));
        let bcast30 = u32::from(Ipv4Addr::new(10, 0, 0, 7));
        let res30 = get_host_counts(net30, bcast30, 30).unwrap();
        assert_eq!(res30.first_host, u32::from(Ipv4Addr::new(10, 0, 0, 5)));
        assert_eq!(res30.last_host, u32::from(Ipv4Addr::new(10, 0, 0, 6)));
        assert_eq!(res30.bcast_addr, bcast30);
        assert_eq!(res30.host_count, 2);

        // RFC 3021 Point-to-Point /31 subnet (172.16.0.2/31) - both endpoints are hosts
        let net31 = u32::from(Ipv4Addr::new(172, 16, 0, 2));
        let bcast31 = u32::from(Ipv4Addr::new(172, 16, 0, 3));
        let res31 = get_host_counts(net31, bcast31, 31).unwrap();
        assert_eq!(res31.first_host, net31);
        assert_eq!(res31.last_host, bcast31);
        assert_eq!(res31.bcast_addr, bcast31);
        assert_eq!(res31.host_count, 2);

        // Single host /32 (10.10.10.10/32)
        let host32 = u32::from(Ipv4Addr::new(10, 10, 10, 10));
        let res32 = get_host_counts(host32, host32, 32).unwrap();
        assert_eq!(res32.first_host, host32);
        assert_eq!(res32.last_host, host32);
        assert_eq!(res32.bcast_addr, host32);
        assert_eq!(res32.host_count, 1);

        // Default route /0 (0.0.0.0/0) - full IPv4 space without shift overflow
        let net0 = 0u32;
        let bcast0 = u32::MAX;
        let res0 = get_host_counts(net0, bcast0, 0).unwrap();
        assert_eq!(res0.first_host, 1);
        assert_eq!(res0.last_host, u32::MAX - 1);
        assert_eq!(res0.bcast_addr, u32::MAX);
        assert_eq!(res0.host_count, 4_294_967_294);

        // Invalid prefix (> 32)
        assert_eq!(get_host_counts(net, bcast, 33).unwrap_err(), SubnetError::InvalidPrefix);
        assert_eq!(get_host_counts(net, bcast, 128).unwrap_err(), SubnetError::InvalidPrefix);
    }

    #[test]
    fn test_parse_ipv4_address() {
        assert_eq!(
            parse_ipv4_address("192.168.1.1").unwrap(),
            Ipv4Addr::new(192, 168, 1, 1)
        );
        assert_eq!(
            parse_ipv4_address(" 10.0.0.1 ").unwrap(),
            Ipv4Addr::new(10, 0, 0, 1)
        );
        assert_eq!(
            parse_ipv4_address("256.0.0.1").unwrap_err(),
            SubnetError::InvalidIp
        );
        assert_eq!(
            parse_ipv4_address("invalid").unwrap_err(),
            SubnetError::InvalidIp
        );
    }

    #[test]
    fn test_parse_prefix() {
        assert_eq!(parse_prefix("0").unwrap(), 0);
        assert_eq!(parse_prefix("24").unwrap(), 24);
        assert_eq!(parse_prefix("32").unwrap(), 32);
        assert_eq!(parse_prefix("33").unwrap_err(), SubnetError::InvalidPrefix);
        assert_eq!(parse_prefix("abc").unwrap_err(), SubnetError::InvalidPrefix);
        assert_eq!(parse_prefix("-1").unwrap_err(), SubnetError::InvalidPrefix);
    }

    #[test]
    fn test_split_input_cidr() {
        let input = SubnetInput {
            ip_or_cidr: "192.168.1.100/24".to_string(),
            prefix: None,
        };
        let (ip, prefix) = split_input(&input).unwrap();
        assert_eq!(ip, Ipv4Addr::new(192, 168, 1, 100));
        assert_eq!(prefix, 24);
    }

    #[test]
    fn test_split_input_separate_prefix() {
        let input = SubnetInput {
            ip_or_cidr: "10.0.0.1".to_string(),
            prefix: Some(8),
        };
        let (ip, prefix) = split_input(&input).unwrap();
        assert_eq!(ip, Ipv4Addr::new(10, 0, 0, 1));
        assert_eq!(prefix, 8);
    }

    #[test]
    fn test_split_input_errors() {
        // Missing prefix
        let input = SubnetInput {
            ip_or_cidr: "192.168.1.1".to_string(),
            prefix: None,
        };
        assert_eq!(split_input(&input).unwrap_err(), SubnetError::InvalidPrefix);

        // Invalid prefix in CIDR
        let input = SubnetInput {
            ip_or_cidr: "192.168.1.1/35".to_string(),
            prefix: None,
        };
        assert_eq!(split_input(&input).unwrap_err(), SubnetError::InvalidPrefix);

        // Invalid IP
        let input = SubnetInput {
            ip_or_cidr: "999.999.999.999/24".to_string(),
            prefix: None,
        };
        assert_eq!(split_input(&input).unwrap_err(), SubnetError::InvalidIp);

        // Empty input
        let input = SubnetInput {
            ip_or_cidr: "".to_string(),
            prefix: None,
        };
        assert_eq!(split_input(&input).unwrap_err(), SubnetError::InvalidFormat);

        // Empty parts around slash
        let input = SubnetInput {
            ip_or_cidr: "/24".to_string(),
            prefix: None,
        };
        assert_eq!(split_input(&input).unwrap_err(), SubnetError::InvalidFormat);

        // Multiple slashes
        let input = SubnetInput {
            ip_or_cidr: "192.168.1.1/24/32".to_string(),
            prefix: None,
        };
        assert_eq!(split_input(&input).unwrap_err(), SubnetError::InvalidFormat);
    }

    #[test]
    fn test_ipv6_parsing_and_split() {
        assert_eq!(parse_prefix_v6("64").unwrap(), 64);
        assert_eq!(parse_prefix_v6("128").unwrap(), 128);
        assert_eq!(
            parse_prefix_v6("129").unwrap_err(),
            SubnetError::InvalidPrefix
        );

        let input = SubnetInput {
            ip_or_cidr: "2001:db8::1/64".to_string(),
            prefix: None,
        };
        let (ip, prefix) = split_input_v6(&input).unwrap();
        assert_eq!(ip, "2001:db8::1".parse::<Ipv6Addr>().unwrap());
        assert_eq!(prefix, 64);

        let (ip_any, prefix_any) = split_input_any(&input).unwrap();
        assert_eq!(
            ip_any,
            IpAddr::V6("2001:db8::1".parse::<Ipv6Addr>().unwrap())
        );
        assert_eq!(prefix_any, 64);
    }

    #[test]
    #[ignore = "not implemented yet"]
    fn test_calculate_ipv4() {
        let input = SubnetInput {
            ip_or_cidr: "192.168.1.100/24".to_string(),
            prefix: None,
        };
        let res = calculate(&input).unwrap();
        assert_eq!(res.network_address, "192.168.1.0");
    }

    #[test]
    #[ignore = "not implemented yet"]
    fn test_calculate_ipv6() {
        let input = SubnetInput {
            ip_or_cidr: "2001:db8::1/64".to_string(),
            prefix: None,
        };
        let res = calculate(&input).unwrap();
        assert_eq!(res.network_address, "2001:db8::");
    }
}
