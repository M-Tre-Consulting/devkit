// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! LAN & Service Scout for developer workstations and local networks.
//!
//! Rapidly probes targets for open developer servers, databases, containers,
//! and remote shells (Vite, Next.js, Node, Docker, PostgreSQL, Redis, SSH).

use std::net::{SocketAddr, TcpStream};
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

/// Scan a single IP host for the standard developer service ports.
pub fn scan_host(host_ip: &str, timeout_ms: u64) -> Result<Vec<PortScanResult>, String> {
    let clean_ip = host_ip.trim();
    if clean_ip.is_empty() {
        return Err("Host IP address cannot be empty".to_string());
    }

    let ip: std::net::IpAddr = clean_ip
        .parse()
        .map_err(|e| format!("Invalid IP address '{clean_ip}': {e}"))?;

    let timeout = Duration::from_millis(timeout_ms.clamp(100, 3000));
    let mut results = Vec::new();

    for svc in KNOWN_SERVICES {
        let addr = SocketAddr::new(ip, svc.port);
        let start = Instant::now();
        let is_open = TcpStream::connect_timeout(&addr, timeout).is_ok();
        let elapsed = start.elapsed().as_millis() as u64;

        results.push(PortScanResult {
            port: svc.port,
            is_open,
            service_name: svc.name.to_string(),
            service_category: svc.category.to_string(),
            description: svc.description.to_string(),
            latency_ms: elapsed,
        });
    }

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
        let res = scan_host("invalid.ip.format", 200);
        assert!(res.is_err());
    }

    #[test]
    fn test_scan_localhost() {
        // Scanning 127.0.0.1 should execute quickly without panicking
        let res = scan_host("127.0.0.1", 100);
        assert!(res.is_ok());
        let ports = res.unwrap();
        assert_eq!(ports.len(), KNOWN_SERVICES.len());
    }
}
