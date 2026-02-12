//! TLS configuration and self-signed certificate generation.
//!
//! This module provides TLS support for the HTTPS proxy, including
//! runtime generation of self-signed certificates for localhost.

use std::sync::Arc;

use rcgen::{CertificateParams, DnType, KeyPair, SanType};
use rustls::crypto::ring::default_provider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

/// Generates a self-signed certificate for localhost and osu! domains.
///
/// The certificate is valid for:
/// - localhost
/// - *.ppy.sh (wildcard for all osu! subdomains)
/// - Specific subdomains: c.ppy.sh, osu.ppy.sh, a.ppy.sh, b.ppy.sh, etc.
///
/// # Returns
///
/// A tuple of (certificate_chain, private_key) in DER format.
///
/// # Errors
///
/// Returns an error if certificate generation fails.
pub fn generate_self_signed_cert(
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), Box<dyn std::error::Error + Send + Sync>>
{
    let mut params = CertificateParams::default();

    // Set the common name
    params
        .distinguished_name
        .push(DnType::CommonName, "rai!connect Local Proxy");
    params
        .distinguished_name
        .push(DnType::OrganizationName, "rai.moe");

    // Add Subject Alternative Names for all domains osu! might connect to
    params.subject_alt_names = vec![
        SanType::DnsName("localhost".try_into()?),
        SanType::DnsName("*.localhost".try_into()?),
        // osu! main domains
        SanType::DnsName("ppy.sh".try_into()?),
        SanType::DnsName("*.ppy.sh".try_into()?),
        // Specific subdomains (in case wildcards don't work)
        SanType::DnsName("osu.ppy.sh".try_into()?),
        SanType::DnsName("c.ppy.sh".try_into()?),
        SanType::DnsName("c1.ppy.sh".try_into()?),
        SanType::DnsName("c4.ppy.sh".try_into()?),
        SanType::DnsName("ce.ppy.sh".try_into()?),
        SanType::DnsName("a.ppy.sh".try_into()?),
        SanType::DnsName("b.ppy.sh".try_into()?),
        SanType::DnsName("i.ppy.sh".try_into()?),
        SanType::DnsName("s.ppy.sh".try_into()?),
        // IP addresses
        SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
        SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)),
    ];

    // Generate key pair
    let key_pair = KeyPair::generate()?;

    // Generate certificate
    let cert = params.self_signed(&key_pair)?;

    // Convert to DER format
    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::try_from(key_pair.serialize_der())
        .map_err(|e| format!("Failed to serialize key: {:?}", e))?;

    tracing::info!("Generated self-signed certificate for localhost proxy");

    Ok((vec![cert_der], key_der))
}

/// Creates a TLS acceptor configured with a self-signed certificate.
///
/// This acceptor can be used to accept HTTPS connections from the osu! client.
///
/// # Returns
///
/// A `TlsAcceptor` ready to accept connections.
///
/// # Errors
///
/// Returns an error if certificate generation or TLS configuration fails.
pub fn create_tls_acceptor() -> Result<TlsAcceptor, Box<dyn std::error::Error + Send + Sync>> {
    let (certs, key) = generate_self_signed_cert()?;

    // Use ring crypto provider explicitly
    let provider = Arc::new(default_provider());

    let config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("Failed to set protocol versions: {}", e))?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("Failed to create TLS config: {}", e))?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_cert() {
        let result = generate_self_signed_cert();
        assert!(result.is_ok(), "Failed to generate certificate: {:?}", result.err());

        let (certs, _key) = result.unwrap();
        assert_eq!(certs.len(), 1);
    }

    #[test]
    fn test_create_acceptor() {
        let result = create_tls_acceptor();
        assert!(result.is_ok(), "Failed to create TLS acceptor: {:?}", result.err());
    }
}
