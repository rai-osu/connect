//! TLS certificate generation and management for the HTTPS proxy.
//!
//! This module generates self-signed certificates at runtime for TLS termination.
//! The certificates are valid for localhost and common osu! domains (*.ppy.sh).

use std::sync::Arc;

use rcgen::{CertificateParams, DnType, KeyPair, SanType};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

/// Domains that the generated certificate should be valid for.
/// These cover localhost and common osu! API endpoints.
const CERTIFICATE_DOMAINS: &[&str] = &[
    "localhost",
    "127.0.0.1",
    "osu.ppy.sh",
    "a.ppy.sh",
    "b.ppy.sh",
    "c.ppy.sh",
    "c1.ppy.sh",
    "c2.ppy.sh",
    "c3.ppy.sh",
    "c4.ppy.sh",
    "c5.ppy.sh",
    "c6.ppy.sh",
    "ce.ppy.sh",
    "s.ppy.sh",
    "i.ppy.sh",
    "api.ppy.sh",
    "notify.ppy.sh",
];

/// Holds the generated certificate and key pair.
pub struct GeneratedCertificate {
    pub cert_der: CertificateDer<'static>,
    pub key_der: PrivateKeyDer<'static>,
}

/// Generates a self-signed certificate valid for osu! domains and localhost.
///
/// The certificate is generated fresh at runtime with:
/// - 1 year validity
/// - Subject Alternative Names for all configured domains
/// - RSA key for broad compatibility
pub fn generate_self_signed_cert() -> Result<GeneratedCertificate, rcgen::Error> {
    let mut params = CertificateParams::default();

    // Set the subject name
    params
        .distinguished_name
        .push(DnType::CommonName, "rai.moe Local Proxy");
    params
        .distinguished_name
        .push(DnType::OrganizationName, "rai.moe");

    // Add Subject Alternative Names for all domains
    params.subject_alt_names = CERTIFICATE_DOMAINS
        .iter()
        .map(|domain| {
            if domain.parse::<std::net::IpAddr>().is_ok() {
                SanType::IpAddress(domain.parse().unwrap())
            } else {
                SanType::DnsName((*domain).try_into().unwrap())
            }
        })
        .collect();

    // Generate a new key pair
    let key_pair = KeyPair::generate()?;

    // Create the self-signed certificate
    let cert = params.self_signed(&key_pair)?;

    // Get the DER-encoded certificate and key
    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_pair.serialize_der()));

    Ok(GeneratedCertificate { cert_der, key_der })
}

/// Creates a TLS acceptor configured for HTTPS connections.
///
/// The acceptor is configured with:
/// - No client authentication (we're a proxy, not verifying clients)
/// - ALPN protocols for HTTP/1.1 and HTTP/1.0
/// - The self-signed certificate for all configured domains
pub fn create_tls_acceptor() -> Result<TlsAcceptor, Box<dyn std::error::Error + Send + Sync>> {
    let GeneratedCertificate { cert_der, key_der } = generate_self_signed_cert()?;

    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)?;

    // Configure ALPN protocols (HTTP/1.1 and HTTP/1.0 - osu! client uses HTTP/1.1)
    server_config.alpn_protocols = vec![b"http/1.1".to_vec(), b"http/1.0".to_vec()];

    Ok(TlsAcceptor::from(Arc::new(server_config)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_self_signed_cert() {
        let result = generate_self_signed_cert();
        assert!(result.is_ok(), "Failed to generate certificate");

        let cert = result.unwrap();
        assert!(!cert.cert_der.is_empty(), "Certificate DER should not be empty");
    }

    #[test]
    fn test_create_tls_acceptor() {
        let result = create_tls_acceptor();
        assert!(result.is_ok(), "Failed to create TLS acceptor");
    }
}
