// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! TLS Inspector: handshake with a server and report protocol, cipher, ALPN, timing,
//! trust verdict and the full certificate chain (subject, issuer, SANs, validity,
//! key, signature algorithm, SHA-256 fingerprint).
//!
//! The first handshake verifies against the Mozilla root store. If verification fails,
//! a second handshake with verification disabled is made purely to *display* the chain,
//! and the report carries the verification error so the UI can flag it.

use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, RootCertStore, SignatureScheme};
use sha2::{Digest, Sha256};
use x509_parser::prelude::*;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CertInfo {
    pub common_name: String,
    pub subject: String,
    pub issuer: String,
    pub serial: String,
    pub not_before: String,
    pub not_after: String,
    pub days_left: i64,
    pub sans: Vec<String>,
    pub signature_algorithm: String,
    pub public_key: String,
    pub sha256: String,
    pub is_ca: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TlsReport {
    pub host: String,
    pub port: u16,
    pub ip: String,
    pub protocol: String,
    pub cipher_suite: String,
    pub alpn: String,
    pub connect_ms: u64,
    pub handshake_ms: u64,
    pub trusted: bool,
    pub verify_error: String,
    pub hostname_matches: bool,
    pub chain: Vec<CertInfo>,
}

/// Parse "host", "host:port", "[v6]:port" or a full URL into (host, port).
pub fn parse_target(input: &str) -> Result<(String, u16), String> {
    let mut s = input.trim();
    if s.is_empty() {
        return Err("Enter a hostname, host:port or https:// URL".into());
    }
    let mut default_port = 443;
    if let Some((scheme, rest)) = s.split_once("://") {
        default_port = match scheme.to_ascii_lowercase().as_str() {
            "https" | "wss" => 443,
            "smtps" => 465,
            "imaps" => 993,
            "ldaps" => 636,
            _ => 443,
        };
        s = rest;
    }
    let authority = s.split(['/', '?', '#']).next().unwrap_or(s);
    let authority = authority.rsplit('@').next().unwrap_or(authority);

    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        let (h, tail) = rest.split_once(']').ok_or("Malformed IPv6 address")?;
        let port = match tail.strip_prefix(':') {
            Some(p) => p.parse::<u16>().map_err(|_| format!("Invalid port '{p}'"))?,
            None => default_port,
        };
        (h.to_string(), port)
    } else {
        match authority.rsplit_once(':') {
            Some((h, p)) if !h.contains(':') => {
                (h.to_string(), p.parse::<u16>().map_err(|_| format!("Invalid port '{p}'"))?)
            }
            _ => (authority.to_string(), default_port),
        }
    };
    if host.is_empty() {
        return Err("Missing hostname".into());
    }
    if port == 0 {
        return Err("Port must be between 1 and 65535".into());
    }
    Ok((host.to_ascii_lowercase(), port))
}

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// Accepts any certificate. Only used for the second, display-only handshake.
#[derive(Debug)]
struct AcceptAnyCert(Arc<CryptoProvider>);

impl ServerCertVerifier for AcceptAnyCert {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn client_config(verify: bool) -> Result<Arc<ClientConfig>, String> {
    let builder = ClientConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?;
    let mut config = if verify {
        let roots = RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
        builder.with_root_certificates(roots).with_no_client_auth()
    } else {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyCert(provider())))
            .with_no_client_auth()
    };
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

struct Handshake {
    conn: ClientConnection,
    connect_ms: u64,
    handshake_ms: u64,
    addr: SocketAddr,
}

fn handshake(host: &str, port: u16, addr: SocketAddr, verify: bool, timeout: Duration) -> Result<Handshake, String> {
    let server_name = ServerName::try_from(host.to_string()).map_err(|_| format!("Invalid server name '{host}'"))?;
    let t0 = Instant::now();
    let mut tcp = TcpStream::connect_timeout(&addr, timeout)
        .map_err(|e| format!("Could not connect to {host}:{port} ({e})"))?;
    let connect_ms = t0.elapsed().as_millis() as u64;
    tcp.set_read_timeout(Some(timeout)).ok();
    tcp.set_write_timeout(Some(timeout)).ok();
    tcp.set_nodelay(true).ok();

    let mut conn = ClientConnection::new(client_config(verify)?, server_name).map_err(|e| e.to_string())?;
    let t1 = Instant::now();
    while conn.is_handshaking() {
        match conn.complete_io(&mut tcp) {
            Ok((0, 0)) if conn.is_handshaking() => {
                return Err("Server closed the connection during the TLS handshake".into());
            }
            Ok(_) => {}
            Err(e) => return Err(describe_io_error(e)),
        }
    }
    // Close politely; failures here don't matter for the report.
    conn.send_close_notify();
    let _ = conn.write_tls(&mut tcp);
    Ok(Handshake { conn, connect_ms, handshake_ms: t1.elapsed().as_millis() as u64, addr })
}

fn describe_io_error(e: std::io::Error) -> String {
    if let Some(inner) = e.get_ref().and_then(|i| i.downcast_ref::<rustls::Error>()) {
        return describe_tls_error(inner);
    }
    match e.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => "TLS handshake timed out".into(),
        _ => format!("TLS handshake failed: {e}"),
    }
}

fn describe_tls_error(e: &rustls::Error) -> String {
    use rustls::{CertificateError, Error};
    match e {
        Error::InvalidCertificate(CertificateError::Expired) => "Certificate has expired".into(),
        Error::InvalidCertificate(CertificateError::NotValidYet) => "Certificate is not valid yet".into(),
        Error::InvalidCertificate(CertificateError::UnknownIssuer) => {
            "Untrusted issuer (self-signed, private CA or incomplete chain)".into()
        }
        Error::InvalidCertificate(CertificateError::NotValidForName) => {
            "Certificate is not valid for this hostname".into()
        }
        Error::InvalidCertificate(CertificateError::Revoked) => "Certificate has been revoked".into(),
        // rustls reports most failures through the *Context variants, which carry the
        // details we want to show instead of a raw debug dump.
        Error::InvalidCertificate(CertificateError::ExpiredContext { not_after, .. }) => {
            format!("Certificate expired on {}", fmt_time(not_after.as_secs() as i64))
        }
        Error::InvalidCertificate(CertificateError::NotValidYetContext { not_before, .. }) => {
            format!("Certificate is not valid until {}", fmt_time(not_before.as_secs() as i64))
        }
        Error::InvalidCertificate(CertificateError::NotValidForNameContext { .. }) => {
            "Certificate is not valid for this hostname".into()
        }
        Error::InvalidCertificate(other) => format!("Invalid certificate: {other}"),
        Error::AlertReceived(alert) => format!("Server sent TLS alert: {alert:?}"),
        other => format!("TLS error: {other}"),
    }
}

fn fmt_time(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| ts.to_string())
}

fn hex_colon(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
}

/// Parse one DER certificate into a display model.
pub fn describe_certificate(der: &[u8], now_ts: i64) -> CertInfo {
    let sha256 = hex_colon(&Sha256::digest(der));
    let Ok((_, cert)) = X509Certificate::from_der(der) else {
        return CertInfo { common_name: "(unparseable certificate)".into(), sha256, ..Default::default() };
    };

    let common_name = cert
        .subject()
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| cert.subject().to_string());

    let mut sans = Vec::new();
    if let Ok(Some(ext)) = cert.subject_alternative_name() {
        for name in &ext.value.general_names {
            match name {
                GeneralName::DNSName(d) => sans.push(d.to_string()),
                GeneralName::IPAddress(ip) => match ip.len() {
                    4 => sans.push(std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]).to_string()),
                    16 => {
                        let mut b = [0u8; 16];
                        b.copy_from_slice(ip);
                        sans.push(std::net::Ipv6Addr::from(b).to_string());
                    }
                    _ => {}
                },
                GeneralName::RFC822Name(m) => sans.push(format!("email:{m}")),
                GeneralName::URI(u) => sans.push(format!("uri:{u}")),
                _ => {}
            }
        }
    }

    let registry = x509_parser::objects::oid_registry();
    let sig_oid = &cert.signature_algorithm.algorithm;
    let signature_algorithm = x509_parser::objects::oid2sn(sig_oid, registry)
        .map(str::to_string)
        .unwrap_or_else(|_| sig_oid.to_id_string());

    let spki = cert.public_key();
    let public_key = match spki.parsed() {
        Ok(x509_parser::public_key::PublicKey::RSA(rsa)) => format!("RSA {} bit", rsa.key_size()),
        Ok(x509_parser::public_key::PublicKey::EC(ec)) => {
            let curve = spki
                .algorithm
                .parameters
                .as_ref()
                .and_then(|p| p.as_oid().ok())
                .and_then(|oid| match oid.to_id_string().as_str() {
                    "1.2.840.10045.3.1.7" => Some("P-256"),
                    "1.3.132.0.34" => Some("P-384"),
                    "1.3.132.0.35" => Some("P-521"),
                    _ => None,
                });
            match curve {
                Some(c) => format!("ECDSA {c}"),
                None => format!("EC {} bit", ec.key_size()),
            }
        }
        Ok(other) => {
            let alg = x509_parser::objects::oid2sn(&spki.algorithm.algorithm, registry).unwrap_or("key");
            format!("{alg} {} bit", other.key_size())
        }
        Err(_) => x509_parser::objects::oid2sn(&spki.algorithm.algorithm, registry)
            .map(str::to_string)
            .unwrap_or_else(|_| "Unknown".into()),
    };

    let nb = cert.validity().not_before.timestamp();
    let na = cert.validity().not_after.timestamp();

    CertInfo {
        common_name,
        subject: cert.subject().to_string(),
        issuer: cert.issuer().to_string(),
        serial: cert.raw_serial_as_string().to_uppercase(),
        not_before: fmt_time(nb),
        not_after: fmt_time(na),
        days_left: (na - now_ts).div_euclid(86_400),
        sans,
        signature_algorithm,
        public_key,
        sha256,
        is_ca: cert.is_ca(),
    }
}

/// Does `pattern` (possibly a `*.` wildcard) cover `host`?
fn name_matches(pattern: &str, host: &str) -> bool {
    let pattern = pattern.to_ascii_lowercase();
    if let Some(suffix) = pattern.strip_prefix("*.") {
        match host.split_once('.') {
            Some((label, rest)) => !label.is_empty() && rest == suffix,
            None => false,
        }
    } else {
        pattern == host
    }
}

fn protocol_name(v: Option<rustls::ProtocolVersion>) -> String {
    match v {
        Some(rustls::ProtocolVersion::TLSv1_3) => "TLS 1.3".into(),
        Some(rustls::ProtocolVersion::TLSv1_2) => "TLS 1.2".into(),
        Some(other) => format!("{other:?}"),
        None => "Unknown".into(),
    }
}

/// Run the full inspection. Blocking: call it from a worker thread.
pub fn inspect(target: &str, timeout_ms: u64) -> Result<TlsReport, String> {
    let (host, port) = parse_target(target)?;
    let timeout = Duration::from_millis(timeout_ms.clamp(1000, 20_000));

    let addr = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| format!("Could not resolve '{host}': {e}"))?
        .min_by_key(|a| if a.is_ipv4() { 0 } else { 1 })
        .ok_or_else(|| format!("No addresses found for '{host}'"))?;

    let (hs, trusted, verify_error) = match handshake(&host, port, addr, true, timeout) {
        Ok(hs) => (hs, true, String::new()),
        Err(err) => {
            // Connection-level failures will fail again; only retry for trust problems.
            if err.starts_with("Could not connect") || err.contains("timed out") {
                return Err(err);
            }
            let hs = handshake(&host, port, addr, false, timeout).map_err(|e2| format!("{err}. {e2}"))?;
            (hs, false, err)
        }
    };

    let now = chrono::Utc::now().timestamp();
    let chain: Vec<CertInfo> = hs
        .conn
        .peer_certificates()
        .map(|certs| certs.iter().map(|c| describe_certificate(c.as_ref(), now)).collect())
        .unwrap_or_default();

    let hostname_matches = chain
        .first()
        .map(|leaf| {
            if leaf.sans.is_empty() {
                name_matches(&leaf.common_name, &host)
            } else {
                leaf.sans.iter().any(|s| name_matches(s, &host))
            }
        })
        .unwrap_or(false);

    Ok(TlsReport {
        host,
        port,
        ip: hs.addr.ip().to_string(),
        protocol: protocol_name(hs.conn.protocol_version()),
        cipher_suite: hs
            .conn
            .negotiated_cipher_suite()
            .map(|c| format!("{:?}", c.suite()).trim_start_matches("TLS13_").trim_start_matches("TLS_").to_string())
            .unwrap_or_else(|| "Unknown".into()),
        alpn: hs
            .conn
            .alpn_protocol()
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .unwrap_or_else(|| "none".into()),
        connect_ms: hs.connect_ms,
        handshake_ms: hs.handshake_ms,
        trusted,
        verify_error,
        hostname_matches,
        chain,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_targets() {
        assert_eq!(parse_target("example.com").unwrap(), ("example.com".into(), 443));
        assert_eq!(parse_target("Example.com:8443").unwrap(), ("example.com".into(), 8443));
        assert_eq!(parse_target("https://a.b/c?d").unwrap(), ("a.b".into(), 443));
        assert_eq!(parse_target("https://user@a.b:9443/").unwrap(), ("a.b".into(), 9443));
        assert_eq!(parse_target("imaps://mail.x.org").unwrap(), ("mail.x.org".into(), 993));
        assert_eq!(parse_target("[::1]:8443").unwrap(), ("::1".into(), 8443));
        assert!(parse_target("").is_err());
        assert!(parse_target("host:99999").is_err());
    }

    #[test]
    fn wildcard_matching() {
        assert!(name_matches("*.example.com", "www.example.com"));
        assert!(!name_matches("*.example.com", "example.com"));
        assert!(!name_matches("*.example.com", "a.b.example.com"));
        assert!(name_matches("Example.com", "example.com"));
    }

    #[test]
    fn tolerates_garbage_certificates() {
        let info = describe_certificate(b"not a cert", 0);
        assert_eq!(info.common_name, "(unparseable certificate)");
        assert_eq!(info.sha256.len(), 32 * 3 - 1);
    }
}
