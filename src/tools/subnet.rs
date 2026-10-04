// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Subnet Calculator module stub.
//!
//! Provides IPv4 and IPv6 CIDR network calculation, address range estimation,
//! and netmask conversions.

use std::net::Ipv4Addr;

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
pub fn calculate(input: &SubnetInput) -> Result<SubnetOutput, SubnetError> {
    // Parse the IP address
    let (ip_addr, prefix) = split_input(input)?;

    // Get IP address as 32-bit integer
    let ip_addr_u32 = u32::from(ip_addr);

    // Perform network calculations
    let netmask = get_netmask(prefix);
    let wcard_mask = get_wildcard_mask(netmask);
    let net_addr = get_network_address(netmask, ip_addr_u32);
    let bcast_addr = get_broadcast_address(net_addr, wcard_mask);
    let host_count = get_host_counts(net_addr, bcast_addr, prefix)?;

    // Now convert all of them to the corresponding Ipv4Addr type
    let nmask_ipv4 = Ipv4Addr::from(netmask);
    let wcard_mask_ipv4 = Ipv4Addr::from(wcard_mask);
    let net_addr_ipv4 = Ipv4Addr::from(net_addr);
    let bcast_addr_ipv4 = Ipv4Addr::from(bcast_addr);

    let first_host_ipv4 = Ipv4Addr::from(host_count.first_host);
    let last_host_ipv4 = Ipv4Addr::from(host_count.last_host);

    // Convert everything to string and output in the correct format
    Ok(SubnetOutput {
        network_address: net_addr_ipv4.to_string(),
        broadcast_address: bcast_addr_ipv4.to_string(),
        first_host: first_host_ipv4.to_string(),
        last_host: last_host_ipv4.to_string(),
        host_count: host_count.host_count.to_string(),
        netmask: nmask_ipv4.to_string(),
        wildcard_mask: wcard_mask_ipv4.to_string(),
        cidr_notation: format!("/{}", prefix),
    })
}

/// Parses an IPv4 address from string.
fn parse_ipv4_address(input: &str) -> Result<Ipv4Addr, SubnetError> {
    input
        .trim()
        .parse::<Ipv4Addr>()
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
    fn test_get_network_address() {
        let ip = u32::from(Ipv4Addr::new(192, 168, 1, 150));
        let mask = get_netmask(24);
        let net = get_network_address(mask, ip);
        assert_eq!(net, u32::from(Ipv4Addr::new(192, 168, 1, 0)));

        let mask_26 = get_netmask(26);
        let net_26 = get_network_address(mask_26, ip);
        assert_eq!(net_26, u32::from(Ipv4Addr::new(192, 168, 1, 128)));
    }

    #[test]
    fn test_get_broadcast_address() {
        let net = u32::from(Ipv4Addr::new(192, 168, 1, 0));
        let wildcard = get_wildcard_mask(get_netmask(24));
        let bcast = get_broadcast_address(net, wildcard);
        assert_eq!(bcast, u32::from(Ipv4Addr::new(192, 168, 1, 255)));

        let net_27 = u32::from(Ipv4Addr::new(10, 0, 0, 32));
        let wildcard_27 = get_wildcard_mask(get_netmask(27));
        let bcast_27 = get_broadcast_address(net_27, wildcard_27);
        assert_eq!(bcast_27, u32::from(Ipv4Addr::new(10, 0, 0, 63)));
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
        assert_eq!(
            get_host_counts(net, bcast, 33).unwrap_err(),
            SubnetError::InvalidPrefix
        );
        assert_eq!(
            get_host_counts(net, bcast, 128).unwrap_err(),
            SubnetError::InvalidPrefix
        );
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
    fn test_calculate_class_c_full() {
        let input = SubnetInput {
            ip_or_cidr: "192.168.1.100/24".to_string(),
            prefix: None,
        };
        let res = calculate(&input).unwrap();
        assert_eq!(res.network_address, "192.168.1.0");
        assert_eq!(res.broadcast_address, "192.168.1.255");
        assert_eq!(res.first_host, "192.168.1.1");
        assert_eq!(res.last_host, "192.168.1.254");
        assert_eq!(res.host_count, "254");
        assert_eq!(res.netmask, "255.255.255.0");
        assert_eq!(res.wildcard_mask, "0.0.0.255");
        assert_eq!(res.cidr_notation, "/24");
    }

    #[test]
    fn test_calculate_class_a_separate_prefix() {
        let input = SubnetInput {
            ip_or_cidr: "10.45.67.89".to_string(),
            prefix: Some(8),
        };
        let res = calculate(&input).unwrap();
        assert_eq!(res.network_address, "10.0.0.0");
        assert_eq!(res.broadcast_address, "10.255.255.255");
        assert_eq!(res.first_host, "10.0.0.1");
        assert_eq!(res.last_host, "10.255.255.254");
        assert_eq!(res.host_count, "16777214");
        assert_eq!(res.netmask, "255.0.0.0");
        assert_eq!(res.wildcard_mask, "0.255.255.255");
        assert_eq!(res.cidr_notation, "/8");
    }

    #[test]
    fn test_calculate_class_b() {
        let input = SubnetInput {
            ip_or_cidr: "172.16.50.25/16".to_string(),
            prefix: None,
        };
        let res = calculate(&input).unwrap();
        assert_eq!(res.network_address, "172.16.0.0");
        assert_eq!(res.broadcast_address, "172.16.255.255");
        assert_eq!(res.first_host, "172.16.0.1");
        assert_eq!(res.last_host, "172.16.255.254");
        assert_eq!(res.host_count, "65534");
        assert_eq!(res.netmask, "255.255.0.0");
        assert_eq!(res.wildcard_mask, "0.0.255.255");
        assert_eq!(res.cidr_notation, "/16");
    }

    #[test]
    fn test_calculate_non_octet_boundary_27() {
        let input = SubnetInput {
            ip_or_cidr: "192.168.1.130/27".to_string(),
            prefix: None,
        };
        let res = calculate(&input).unwrap();
        assert_eq!(res.network_address, "192.168.1.128");
        assert_eq!(res.broadcast_address, "192.168.1.159");
        assert_eq!(res.first_host, "192.168.1.129");
        assert_eq!(res.last_host, "192.168.1.158");
        assert_eq!(res.host_count, "30");
        assert_eq!(res.netmask, "255.255.255.224");
        assert_eq!(res.wildcard_mask, "0.0.0.31");
        assert_eq!(res.cidr_notation, "/27");
    }

    #[test]
    fn test_calculate_small_subnet_30() {
        let input = SubnetInput {
            ip_or_cidr: "10.0.0.5/30".to_string(),
            prefix: None,
        };
        let res = calculate(&input).unwrap();
        assert_eq!(res.network_address, "10.0.0.4");
        assert_eq!(res.broadcast_address, "10.0.0.7");
        assert_eq!(res.first_host, "10.0.0.5");
        assert_eq!(res.last_host, "10.0.0.6");
        assert_eq!(res.host_count, "2");
        assert_eq!(res.netmask, "255.255.255.252");
        assert_eq!(res.wildcard_mask, "0.0.0.3");
        assert_eq!(res.cidr_notation, "/30");
    }

    #[test]
    fn test_calculate_point_to_point_rfc3021_31() {
        let input = SubnetInput {
            ip_or_cidr: "192.168.10.15/31".to_string(),
            prefix: None,
        };
        let res = calculate(&input).unwrap();
        assert_eq!(res.network_address, "192.168.10.14");
        assert_eq!(res.broadcast_address, "192.168.10.15");
        assert_eq!(res.first_host, "192.168.10.14");
        assert_eq!(res.last_host, "192.168.10.15");
        assert_eq!(res.host_count, "2");
        assert_eq!(res.netmask, "255.255.255.254");
        assert_eq!(res.wildcard_mask, "0.0.0.1");
        assert_eq!(res.cidr_notation, "/31");
    }

    #[test]
    fn test_calculate_single_host_32() {
        let input = SubnetInput {
            ip_or_cidr: "192.168.1.1/32".to_string(),
            prefix: None,
        };
        let res = calculate(&input).unwrap();
        assert_eq!(res.network_address, "192.168.1.1");
        assert_eq!(res.broadcast_address, "192.168.1.1");
        assert_eq!(res.first_host, "192.168.1.1");
        assert_eq!(res.last_host, "192.168.1.1");
        assert_eq!(res.host_count, "1");
        assert_eq!(res.netmask, "255.255.255.255");
        assert_eq!(res.wildcard_mask, "0.0.0.0");
        assert_eq!(res.cidr_notation, "/32");
    }

    #[test]
    fn test_calculate_default_route_0() {
        let input = SubnetInput {
            ip_or_cidr: "0.0.0.0/0".to_string(),
            prefix: None,
        };
        let res = calculate(&input).unwrap();
        assert_eq!(res.network_address, "0.0.0.0");
        assert_eq!(res.broadcast_address, "255.255.255.255");
        assert_eq!(res.first_host, "0.0.0.1");
        assert_eq!(res.last_host, "255.255.255.254");
        assert_eq!(res.host_count, "4294967294");
        assert_eq!(res.netmask, "0.0.0.0");
        assert_eq!(res.wildcard_mask, "255.255.255.255");
        assert_eq!(res.cidr_notation, "/0");
    }

    #[test]
    fn test_calculate_with_whitespace() {
        let input = SubnetInput {
            ip_or_cidr: "  10.0.0.1 / 24  ".to_string(),
            prefix: None,
        };
        let res = calculate(&input).unwrap();
        assert_eq!(res.network_address, "10.0.0.0");
        assert_eq!(res.netmask, "255.255.255.0");
    }

    #[test]
    fn test_calculate_error_handling() {
        // Missing prefix
        let no_prefix = SubnetInput {
            ip_or_cidr: "192.168.1.1".to_string(),
            prefix: None,
        };
        assert_eq!(
            calculate(&no_prefix).unwrap_err(),
            SubnetError::InvalidPrefix
        );

        // Invalid prefix > 32
        let invalid_prefix = SubnetInput {
            ip_or_cidr: "192.168.1.1/33".to_string(),
            prefix: None,
        };
        assert_eq!(
            calculate(&invalid_prefix).unwrap_err(),
            SubnetError::InvalidPrefix
        );

        // Invalid IP address octets
        let invalid_ip = SubnetInput {
            ip_or_cidr: "300.168.1.1/24".to_string(),
            prefix: None,
        };
        assert_eq!(calculate(&invalid_ip).unwrap_err(), SubnetError::InvalidIp);

        // Malformed format
        let invalid_format = SubnetInput {
            ip_or_cidr: "192.168.1.1/24/32".to_string(),
            prefix: None,
        };
        assert_eq!(
            calculate(&invalid_format).unwrap_err(),
            SubnetError::InvalidFormat
        );

        // Non-IPv4 address (e.g. IPv6 input)
        let ipv6_input = SubnetInput {
            ip_or_cidr: "2001:db8::1/64".to_string(),
            prefix: None,
        };
        assert_eq!(calculate(&ipv6_input).unwrap_err(), SubnetError::InvalidIp);
    }
}
