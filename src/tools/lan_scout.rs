// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! LAN & Service Scout for developer workstations and local networks.
//!
//! Rapidly probes targets for open developer servers, databases, containers,
//! and remote shells (Vite, Next.js, Node, Docker, PostgreSQL, Redis, SSH).

use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// Known developer and infrastructure port definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDefinition {
    pub port: u16,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
}

pub const KNOWN_SERVICES: &[ServiceDefinition] = &[
    ServiceDefinition { port: 22, name: "SSH", category: "Remote Access", description: "Secure Shell" },
    ServiceDefinition { port: 80, name: "HTTP", category: "Web Server", description: "Standard Web Traffic" },
    ServiceDefinition { port: 443, name: "HTTPS", category: "Web Server", description: "Secure TLS Web Traffic" },
    ServiceDefinition { port: 3000, name: "Dev Server", category: "Web Dev", description: "Node / React / Next.js / Rails" },
    ServiceDefinition { port: 3306, name: "MySQL", category: "Database", description: "MySQL / MariaDB Database" },
    ServiceDefinition { port: 5000, name: "Flask / Docker", category: "Dev Server", description: "Python Flask / Docker Registry" },
    ServiceDefinition { port: 5173, name: "Vite", category: "Web Dev", description: "Vite / SvelteKit Dev Server" },
    ServiceDefinition { port: 5432, name: "PostgreSQL", category: "Database", description: "PostgreSQL Database Server" },
    ServiceDefinition { port: 6379, name: "Redis", category: "Database", description: "Redis In-Memory Key-Value" },
    ServiceDefinition { port: 8000, name: "Dev Server", category: "Web Dev", description: "Django / FastAPI / Python HTTP" },
    ServiceDefinition { port: 8080, name: "Web Proxy", category: "Web Server", description: "Spring Boot / Tomcat / Proxy" },
    ServiceDefinition { port: 8123, name: "Home Assistant", category: "IoT", description: "Home Assistant Web Interface" },
    ServiceDefinition { port: 9000, name: "Portainer", category: "Containers", description: "Portainer / MinIO / SonarQube" },
    ServiceDefinition { port: 27017, name: "MongoDB", category: "Database", description: "MongoDB Document Database" },
];

/// Result of checking a specific port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortScanResult {
    pub port: u16,
    pub is_open: bool,
    pub service_name: String,
    pub service_category: String,
    pub description: String,
    pub latency_ms: u64,
}

/// Structure holding discovered local network parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkIdentity {
    pub device_ip: Option<String>,
    pub gateway_ip: Option<String>,
}

/// Detect active local Wi-Fi / LAN IP and gateway without network delay.
pub fn detect_network_identity() -> NetworkIdentity {
    let mut device_ip = None;
    let mut gateway_ip = None;

    // 1. Query kernel route table using connected UDP socket (no packets transmitted)
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(local) = socket.local_addr() {
                if let std::net::IpAddr::V4(v4) = local.ip() {
                    let oct = v4.octets();
                    device_ip = Some(v4.to_string());
                    if oct[0] == 10 && oct[1] == 0 && oct[2] == 2 {
                        gateway_ip = Some("10.0.2.2".to_string());
                    } else {
                        gateway_ip = Some(format!("{}.{}.{}.1", oct[0], oct[1], oct[2]));
                    }
                }
            }
        }
    }

    // 2. Parse /proc/net/route if present (Linux / Android standard interface table)
    if let Ok(content) = std::fs::read_to_string("/proc/net/route") {
        for line in content.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 3 {
                let dest = fields[1];
                let gw = fields[2];
                if dest == "00000000" && gw != "00000000" {
                    if let Ok(num) = u32::from_str_radix(gw, 16) {
                        let b = num.to_le_bytes();
                        gateway_ip = Some(format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]));
                        break;
                    }
                }
            }
        }
    }

    // Default fallback to 192.168.0.1 if discovery yielded nothing
    let final_gateway = gateway_ip.or_else(|| Some("192.168.0.1".to_string()));
    let final_device = device_ip.or_else(|| Some("127.0.0.1".to_string()));

    NetworkIdentity {
        device_ip: final_device,
        gateway_ip: final_gateway,
    }
}

/// Scan a single IP host or hostname for the standard developer service ports in parallel.
pub fn scan_host(host_ip: &str, timeout_ms: u64) -> Result<Vec<PortScanResult>, String> {
    let clean_ip = host_ip.trim();
    if clean_ip.is_empty() {
        return Err("Host IP address cannot be empty".to_string());
    }

    // Resolve IP or hostname (e.g. "localhost", "192.168.0.1", "server.local")
    let target_ip = if let Ok(ip) = clean_ip.parse::<std::net::IpAddr>() {
        ip
    } else {
        let probe = format!("{clean_ip}:80");
        match probe.to_socket_addrs() {
            Ok(mut addrs) => {
                if let Some(first) = addrs.next() {
                    first.ip()
                } else {
                    return Err(format!("Could not resolve host '{clean_ip}'"));
                }
            }
            Err(e) => return Err(format!("Invalid IP or unresolvable hostname '{clean_ip}': {e}")),
        }
    };

    let timeout = Duration::from_millis(timeout_ms.clamp(80, 3000));

    // Parallel port probe using scoped threads for fast results
    let results: Vec<PortScanResult> = std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(KNOWN_SERVICES.len());
        for svc in KNOWN_SERVICES {
            handles.push(s.spawn(move || {
                let addr = SocketAddr::new(target_ip, svc.port);
                let start = Instant::now();
                let is_open = TcpStream::connect_timeout(&addr, timeout).is_ok();
                let elapsed = start.elapsed().as_millis() as u64;

                PortScanResult {
                    port: svc.port,
                    is_open,
                    service_name: svc.name.to_string(),
                    service_category: svc.category.to_string(),
                    description: svc.description.to_string(),
                    latency_ms: elapsed,
                }
            }));
        }

        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    Ok(results)
}

/// Helper to get service definition for an arbitrary port.
pub fn lookup_service_name(port: u16) -> (&'static str, &'static str) {
    for s in KNOWN_SERVICES {
        if s.port == port {
            return (s.name, s.description);
        }
    }
    ("Custom", "User-specified TCP port")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_services_count() {
        assert!(KNOWN_SERVICES.len() >= 12);
        assert!(KNOWN_SERVICES.iter().any(|s| s.port == 5173 && s.name == "Vite"));
        assert!(KNOWN_SERVICES.iter().any(|s| s.port == 5432 && s.name == "PostgreSQL"));
    }

    #[test]
    fn test_invalid_ip() {
        let res = scan_host("invalid.ip.format.nonexistent.xyz", 200);
        assert!(res.is_err());
    }

    #[test]
    fn test_scan_localhost() {
        let res = scan_host("127.0.0.1", 100);
        assert!(res.is_ok());
        let ports = res.unwrap();
        assert_eq!(ports.len(), KNOWN_SERVICES.len());
    }

    #[test]
    fn test_network_identity_detection() {
        let ident = detect_network_identity();
        assert!(ident.gateway_ip.is_some());
    }
}
