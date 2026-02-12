//! TLS configuration and self-signed certificate generation.
//!
//! This module provides TLS support for the HTTPS proxy, including
//! runtime generation of self-signed certificates for localhost.

use std::path::PathBuf;
use std::sync::Arc;

use rcgen::{CertificateParams, DnType, KeyPair, SanType};
use rustls::crypto::ring::default_provider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

/// Generates a self-signed certificate for localhost.
///
/// When osu! uses `-devserver localhost`, it replaces `ppy.sh` with `localhost`
/// in all URLs. So `c.ppy.sh` becomes `c.localhost`, etc.
///
/// The certificate is valid for:
/// - `localhost`
/// - `*.localhost` (covers c.localhost, osu.localhost, a.localhost, etc.)
/// - `127.0.0.1` and `::1`
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

    // Add Subject Alternative Names for localhost domains
    // With -devserver localhost, osu! connects to *.localhost (e.g., c.localhost, osu.localhost)
    params.subject_alt_names = vec![
        SanType::DnsName("localhost".try_into()?),
        SanType::DnsName("*.localhost".try_into()?),
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

/// Returns the path where the certificate should be stored.
pub fn get_cert_path() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let app_data = dirs::data_local_dir()
        .ok_or("Could not find local app data directory")?;
    let cert_dir = app_data.join("rai-connect");
    std::fs::create_dir_all(&cert_dir)?;
    Ok(cert_dir.join("localhost.cer"))
}

/// Generates and saves the certificate to disk, then installs it into the
/// Windows trusted root certificate store.
///
/// This only needs to be done once. The certificate is saved to:
/// `%LOCALAPPDATA%/rai-connect/localhost.cer`
///
/// # Returns
///
/// Returns `Ok(true)` if the certificate was installed successfully,
/// `Ok(false)` if it was already installed, or an error if installation failed.
pub fn install_certificate() -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let cert_path = get_cert_path()?;

    // Generate certificate
    let mut params = CertificateParams::default();
    params
        .distinguished_name
        .push(DnType::CommonName, "rai!connect Local Proxy");
    params
        .distinguished_name
        .push(DnType::OrganizationName, "rai.moe");

    params.subject_alt_names = vec![
        SanType::DnsName("localhost".try_into()?),
        SanType::DnsName("*.localhost".try_into()?),
        SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
        SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)),
    ];

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    // Save certificate in DER format (.cer)
    std::fs::write(&cert_path, cert.der())?;
    tracing::info!("Certificate saved to: {}", cert_path.display());

    // Install certificate using certutil (Windows)
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("certutil")
            .args(["-addstore", "-user", "Root", cert_path.to_str().unwrap()])
            .output()?;

        if output.status.success() {
            tracing::info!("Certificate installed to Windows trusted root store");
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already in store") || stderr.contains("Object already exists") {
                tracing::info!("Certificate already installed");
                Ok(false)
            } else {
                Err(format!(
                    "Failed to install certificate: {}",
                    String::from_utf8_lossy(&output.stderr)
                ).into())
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        tracing::warn!("Automatic certificate installation not supported on this OS");
        tracing::info!("Please manually trust the certificate at: {}", cert_path.display());
        Ok(false)
    }
}

/// Checks if the certificate is already installed in the Windows certificate store.
#[cfg(target_os = "windows")]
pub fn is_certificate_installed() -> bool {
    let output = std::process::Command::new("certutil")
        .args(["-store", "-user", "Root", "rai!connect"])
        .output();

    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_certificate_installed() -> bool {
    false
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
