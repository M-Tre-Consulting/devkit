// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! LAN & Service Scout for developer workstations and local networks.
//!
//! Rapidly probes targets for open developer servers, databases, containers,
//! and remote shells (Vite, Next.js, Node, Docker, PostgreSQL, Redis, SSH).

use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs};
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
    ServiceDefinition { port: 21, name: "FTP", category: "File Transfer", description: "File Transfer Protocol" },
    ServiceDefinition { port: 22, name: "SSH", category: "Remote Access", description: "Secure Shell" },
    ServiceDefinition { port: 23, name: "Telnet", category: "Remote Access", description: "Unencrypted remote shell" },
    ServiceDefinition { port: 53, name: "DNS", category: "Network", description: "Domain Name System (TCP)" },
    ServiceDefinition { port: 80, name: "HTTP", category: "Web Server", description: "Standard Web Traffic" },
    ServiceDefinition { port: 139, name: "NetBIOS", category: "File Sharing", description: "Windows NetBIOS session" },
    ServiceDefinition { port: 443, name: "HTTPS", category: "Web Server", description: "Secure TLS Web Traffic" },
    ServiceDefinition { port: 445, name: "SMB", category: "File Sharing", description: "Windows / Samba file shares" },
    ServiceDefinition { port: 1883, name: "MQTT", category: "IoT", description: "MQTT message broker" },
    ServiceDefinition { port: 2375, name: "Docker API", category: "Containers", description: "Docker daemon (plain TCP!)" },
    ServiceDefinition { port: 3000, name: "Dev Server", category: "Web Dev", description: "Node / React / Next.js / Rails" },
    ServiceDefinition { port: 3306, name: "MySQL", category: "Database", description: "MySQL / MariaDB Database" },
    ServiceDefinition { port: 3389, name: "RDP", category: "Remote Access", description: "Windows Remote Desktop" },
    ServiceDefinition { port: 4200, name: "Angular", category: "Web Dev", description: "Angular CLI dev server" },
    ServiceDefinition { port: 5000, name: "Flask / Docker", category: "Dev Server", description: "Python Flask / Docker Registry" },
    ServiceDefinition { port: 5173, name: "Vite", category: "Web Dev", description: "Vite / SvelteKit Dev Server" },
    ServiceDefinition { port: 5432, name: "PostgreSQL", category: "Database", description: "PostgreSQL Database Server" },
    ServiceDefinition { port: 5555, name: "ADB", category: "Android", description: "ADB over TCP/IP" },
    ServiceDefinition { port: 5672, name: "AMQP", category: "Messaging", description: "RabbitMQ / AMQP broker" },
    ServiceDefinition { port: 5900, name: "VNC", category: "Remote Access", description: "Virtual Network Computing" },
    ServiceDefinition { port: 6379, name: "Redis", category: "Database", description: "Redis In-Memory Key-Value" },
    ServiceDefinition { port: 6443, name: "Kubernetes", category: "Containers", description: "Kubernetes API server" },
    ServiceDefinition { port: 8000, name: "Dev Server", category: "Web Dev", description: "Django / FastAPI / Python HTTP" },
    ServiceDefinition { port: 8080, name: "Web Proxy", category: "Web Server", description: "Spring Boot / Tomcat / Proxy" },
    ServiceDefinition { port: 8081, name: "Metro", category: "Web Dev", description: "React Native Metro bundler" },
    ServiceDefinition { port: 8123, name: "Home Assistant", category: "IoT", description: "Home Assistant Web Interface" },
    ServiceDefinition { port: 8443, name: "HTTPS Alt", category: "Web Server", description: "Alternate TLS web / admin UI" },
    ServiceDefinition { port: 8888, name: "Jupyter", category: "Data", description: "Jupyter Notebook / Lab" },
    ServiceDefinition { port: 9000, name: "Portainer", category: "Containers", description: "Portainer / MinIO / SonarQube" },
    ServiceDefinition { port: 9090, name: "Prometheus", category: "Monitoring", description: "Prometheus / Cockpit" },
    ServiceDefinition { port: 9200, name: "Elasticsearch", category: "Database", description: "Elasticsearch / OpenSearch" },
    ServiceDefinition { port: 11434, name: "Ollama", category: "AI", description: "Ollama local LLM server" },
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
    pub prefix_len: Option<u8>,
    pub interface: Option<String>,
    pub dns_servers: Vec<String>,
}

/// Detect the active local IP, prefix and gateway.
///
/// Prefers what the OS reports (ConnectivityManager on Android, which works on Android 10+
/// where `/proc/net/route` is no longer readable by apps), then falls back to a routing
/// probe with a connected UDP socket (no packets are sent) and `/proc/net/route`.
pub fn detect_network_identity() -> NetworkIdentity {
    if let Some(info) = crate::platform::android::query_network_info() {
        if info.device_ip.is_some() {
            return NetworkIdentity {
                gateway_ip: info.gateway.or_else(|| info.device_ip.as_deref().and_then(guess_gateway)),
                device_ip: info.device_ip,
                prefix_len: info.prefix_len,
                interface: info.interface,
                dns_servers: info.dns_servers,
            };
        }
    }

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
        prefix_len: None,
        interface: None,
        dns_servers: Vec::new(),
    }
}

fn guess_gateway(ip: &str) -> Option<String> {
    let v4: std::net::Ipv4Addr = ip.parse().ok()?;
    let o = v4.octets();
    Some(format!("{}.{}.{}.1", o[0], o[1], o[2]))
}

/// Resolve an IP literal or hostname to one address.
pub fn resolve_host(host: &str) -> Result<IpAddr, String> {
    let clean = host.trim();
    if clean.is_empty() {
        return Err("Host IP address cannot be empty".to_string());
    }
    if let Ok(ip) = clean.parse::<IpAddr>() {
        return Ok(ip);
    }
    match (clean, 80u16).to_socket_addrs() {
        Ok(addrs) => addrs
            .min_by_key(|a| if a.is_ipv4() { 0 } else { 1 })
            .map(|a| a.ip())
            .ok_or_else(|| format!("Could not resolve host '{clean}'")),
        Err(e) => Err(format!("Invalid IP or unresolvable hostname '{clean}': {e}")),
    }
}

/// Parse a port specification such as "22, 80, 8000-8010".
pub fn parse_port_spec(spec: &str) -> Result<Vec<u16>, String> {
    let mut ports: Vec<u16> = Vec::new();
    for part in spec.split([',', ' ', ';']).map(str::trim).filter(|p| !p.is_empty()) {
        let parse = |s: &str| -> Result<u16, String> {
            s.trim()
                .parse::<u16>()
                .ok()
                .filter(|p| *p > 0)
                .ok_or_else(|| format!("'{s}' is not a valid port (1-65535)"))
        };
        if let Some((a, b)) = part.split_once('-') {
            let (a, b) = (parse(a)?, parse(b)?);
            if a > b {
                return Err(format!("Range '{part}' is reversed"));
            }
            ports.extend(a..=b);
        } else {
            ports.push(parse(part)?);
        }
        if ports.len() > MAX_CUSTOM_PORTS {
            return Err(format!("Too many ports: at most {MAX_CUSTOM_PORTS} per scan"));
        }
    }
    ports.sort_unstable();
    ports.dedup();
    if ports.is_empty() {
        return Err("Enter at least one port, e.g. 22,80,8000-8010".to_string());
    }
    Ok(ports)
}

pub const MAX_CUSTOM_PORTS: usize = 1024;
const SCAN_WORKERS: usize = 64;

/// Run `job` over `items` on a bounded pool of scoped threads, preserving order.
fn parallel_map<T: Sync, R: Send>(items: &[T], job: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<std::sync::Mutex<Option<R>>> = items.iter().map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..SCAN_WORKERS.min(items.len().max(1)) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if i >= items.len() {
                    break;
                }
                let r = job(&items[i]);
                *results[i].lock().unwrap() = Some(r);
            });
        }
    });
    results.into_iter().filter_map(|m| m.into_inner().ok().flatten()).collect()
}

/// Probe the given ports on `host` concurrently.
pub fn scan_ports(host: &str, ports: &[u16], timeout_ms: u64) -> Result<Vec<PortScanResult>, String> {
    let target_ip = resolve_host(host)?;
    let timeout = Duration::from_millis(timeout_ms.clamp(80, 3000));
    Ok(parallel_map(ports, |&port| {
        let addr = SocketAddr::new(target_ip, port);
        let start = Instant::now();
        let is_open = TcpStream::connect_timeout(&addr, timeout).is_ok();
        let (name, category, description) = lookup_service(port);
        PortScanResult {
            port,
            is_open,
            service_name: name.to_string(),
            service_category: category.to_string(),
            description: description.to_string(),
            latency_ms: start.elapsed().as_millis() as u64,
        }
    }))
}

/// Scan a single IP host or hostname for the known developer service ports.
pub fn scan_host(host_ip: &str, timeout_ms: u64) -> Result<Vec<PortScanResult>, String> {
    let ports: Vec<u16> = KNOWN_SERVICES.iter().map(|s| s.port).collect();
    scan_ports(host_ip, &ports, timeout_ms)
}

/// A live host found by [`discover_hosts`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostResult {
    pub ip: String,
    pub latency_ms: u64,
    /// Comma separated list of open probe ports, or "" if the host only answered with a reset.
    pub open_ports: String,
    pub is_self: bool,
    pub is_gateway: bool,
}

/// Ports used to wake up hosts during discovery. A TCP reset (connection refused) also
/// proves the host is alive, so phones and locked-down machines are found too.
const DISCOVERY_PORTS: &[u16] = &[80, 443, 22, 62078, 445];

/// Compute the IPv4 hosts to sweep for `ip/prefix`. Prefixes shorter than /24 are
/// narrowed to the /24 containing `ip` to keep the sweep fast on a phone.
pub fn sweep_targets(ip: &str, prefix: u8) -> Result<Vec<Ipv4Addr>, String> {
    let ip: Ipv4Addr = ip.trim().parse().map_err(|_| format!("'{ip}' is not an IPv4 address"))?;
    let prefix = prefix.clamp(24, 30);
    let mask: u32 = u32::MAX << (32 - prefix);
    let net = u32::from(ip) & mask;
    let broadcast = net | !mask;
    Ok(((net + 1)..broadcast).map(Ipv4Addr::from).collect())
}

/// Find live hosts in the subnet of `device_ip`.
pub fn discover_hosts(device_ip: &str, prefix: u8, gateway: &str, timeout_ms: u64) -> Result<Vec<HostResult>, String> {
    let targets = sweep_targets(device_ip, prefix)?;
    let timeout = Duration::from_millis(timeout_ms.clamp(100, 1500));
    let found = parallel_map(&targets, |ip| {
        let start = Instant::now();
        let mut alive = false;
        let mut open = Vec::new();
        for &port in DISCOVERY_PORTS {
            match TcpStream::connect_timeout(&SocketAddr::new(IpAddr::V4(*ip), port), timeout) {
                Ok(_) => {
                    alive = true;
                    open.push(port.to_string());
                }
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
                    alive = true;
                    break;
                }
                Err(_) => {}
            }
            if alive {
                break;
            }
        }
        alive.then(|| HostResult {
            ip: ip.to_string(),
            latency_ms: start.elapsed().as_millis() as u64,
            open_ports: open.join(", "),
            is_self: ip.to_string() == device_ip.trim(),
            is_gateway: ip.to_string() == gateway.trim(),
        })
    });
    Ok(found.into_iter().flatten().collect())
}

/// Name, category and description for a port.
pub fn lookup_service(port: u16) -> (&'static str, &'static str, &'static str) {
    KNOWN_SERVICES
        .iter()
        .find(|s| s.port == port)
        .map(|s| (s.name, s.category, s.description))
        .unwrap_or(("TCP", "Custom", "User-specified TCP port"))
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
    fn test_port_spec() {
        assert_eq!(parse_port_spec("22, 80,8000-8002").unwrap(), vec![22, 80, 8000, 8001, 8002]);
        assert_eq!(parse_port_spec("443 443").unwrap(), vec![443]);
        assert!(parse_port_spec("0").is_err());
        assert!(parse_port_spec("90-80").is_err());
        assert!(parse_port_spec("1-5000").is_err());
        assert!(parse_port_spec("").is_err());
    }

    #[test]
    fn test_sweep_targets() {
        let t = sweep_targets("192.168.1.42", 24).unwrap();
        assert_eq!(t.len(), 254);
        assert_eq!(t[0], Ipv4Addr::new(192, 168, 1, 1));
        // /16 is narrowed to the /24 around the device
        assert_eq!(sweep_targets("10.1.2.3", 16).unwrap().len(), 254);
        assert_eq!(sweep_targets("10.1.2.3", 30).unwrap().len(), 2);
    }

    #[test]
    fn test_custom_port_lookup() {
        assert_eq!(lookup_service(5432).0, "PostgreSQL");
        assert_eq!(lookup_service(41234).1, "Custom");
    }

    #[test]
    fn test_network_identity_detection() {
        let ident = detect_network_identity();
        assert!(ident.gateway_ip.is_some());
    }
}
