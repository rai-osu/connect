//! TLS configuration and self-signed certificate generation.
//!
//! This module provides TLS support for the HTTPS proxy, including
//! runtime generation of self-signed certificates for localhost.

use std::path::PathBuf;
use std::sync::Arc;

use rcgen::{CertificateParams, DnType, KeyPair, SanType};
use rustls::crypto::ring::default_provider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

/// Returns the directory where certificate files are stored.
fn get_cert_dir() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let app_data =
        dirs::data_local_dir().ok_or("Could not find local app data directory")?;
    let cert_dir = app_data.join("rai-connect");
    std::fs::create_dir_all(&cert_dir)?;
    Ok(cert_dir)
}

/// Returns the path where the certificate should be stored.
pub fn get_cert_path() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    Ok(get_cert_dir()?.join("localhost.cer"))
}

/// Returns the path where the private key should be stored.
fn get_key_path() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    Ok(get_cert_dir()?.join("localhost.key"))
}

/// Generates a new certificate and key pair, saving both to disk.
///
/// The certificate is valid for:
/// - `localhost`
/// - `*.localhost` (covers c.localhost, osu.localhost, a.localhost, etc.)
/// - `127.0.0.1` and `::1`
fn generate_and_save_cert(
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), Box<dyn std::error::Error + Send + Sync>>
{
    let cert_path = get_cert_path()?;
    let key_path = get_key_path()?;

    let mut params = CertificateParams::default();

    params
        .distinguished_name
        .push(DnType::CommonName, "rai!connect Local Proxy");
    params
        .distinguished_name
        .push(DnType::OrganizationName, "rai.moe");

    // Add Subject Alternative Names for localhost domains
    // With -devserver localhost, osu! connects to *.localhost (e.g., c.localhost, osu.localhost)
    // Include both wildcard and explicit subdomains for maximum compatibility
    params.subject_alt_names = vec![
        SanType::DnsName("localhost".try_into()?),
        SanType::DnsName("*.localhost".try_into()?),
        // Explicit subdomains (some clients don't handle wildcards correctly)
        SanType::DnsName("osu.localhost".try_into()?),
        SanType::DnsName("c.localhost".try_into()?),
        SanType::DnsName("a.localhost".try_into()?),
        SanType::DnsName("b.localhost".try_into()?),
        SanType::DnsName("i.localhost".try_into()?),
        // IP addresses
        SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
        SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)),
    ];

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    // Save certificate in DER format (.cer)
    std::fs::write(&cert_path, cert.der())?;
    tracing::info!("Certificate saved to: {}", cert_path.display());

    // Save private key in DER format
    let key_der_bytes = key_pair.serialize_der();
    std::fs::write(&key_path, &key_der_bytes)?;
    tracing::info!("Private key saved to: {}", key_path.display());

    // Convert to rustls types
    // rcgen serializes ECDSA keys in PKCS#8 format
    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der_bytes));

    Ok((vec![cert_der], key_der))
}

/// Loads an existing certificate and key from disk.
fn load_cert_from_disk(
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), Box<dyn std::error::Error + Send + Sync>>
{
    let cert_path = get_cert_path()?;
    let key_path = get_key_path()?;

    let cert_bytes = std::fs::read(&cert_path)?;
    let key_bytes = std::fs::read(&key_path)?;

    let cert_der = CertificateDer::from(cert_bytes);
    // rcgen serializes keys in PKCS#8 format, so explicitly use that type
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_bytes));

    tracing::debug!("Loaded certificate from disk");

    Ok((vec![cert_der], key_der))
}

/// Gets or creates the certificate and key pair.
///
/// If a certificate already exists on disk, it will be loaded.
/// Otherwise, a new certificate will be generated and saved.
pub fn get_or_create_cert(
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), Box<dyn std::error::Error + Send + Sync>>
{
    let cert_path = get_cert_path()?;
    let key_path = get_key_path()?;

    if cert_path.exists() && key_path.exists() {
        match load_cert_from_disk() {
            Ok(result) => return Ok(result),
            Err(e) => {
                tracing::warn!("Failed to load existing certificate, regenerating: {}", e);
            }
        }
    }

    generate_and_save_cert()
}

/// Creates a TLS acceptor configured with the certificate.
///
/// This acceptor can be used to accept HTTPS connections from the osu! client.
/// Uses the persisted certificate if available, otherwise generates a new one.
///
/// # Returns
///
/// A `TlsAcceptor` ready to accept connections.
///
/// # Errors
///
/// Returns an error if certificate generation or TLS configuration fails.
pub fn create_tls_acceptor() -> Result<TlsAcceptor, Box<dyn std::error::Error + Send + Sync>> {
    let (certs, key) = get_or_create_cert()?;

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

/// Generates (if needed) and installs the certificate into the Windows trusted root store.
///
/// This only needs to be done once. The certificate is saved to:
/// `%LOCALAPPDATA%/rai-connect/localhost.cer`
///
/// # Returns
///
/// Returns `Ok(true)` if the certificate was installed successfully,
/// `Ok(false)` if it was already installed, or an error if installation failed.
pub fn install_certificate() -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let _ = get_or_create_cert()?;
    let cert_path = get_cert_path()?;

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
                )
                .into())
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        tracing::warn!("Automatic certificate installation not supported on this OS");
        tracing::info!(
            "Please manually trust the certificate at: {}",
            cert_path.display()
        );
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
    fn test_get_or_create_cert() {
        let result = get_or_create_cert();
        assert!(
            result.is_ok(),
            "Failed to get/create certificate: {:?}",
            result.err()
        );

        let (certs, _key) = result.unwrap();
        assert_eq!(certs.len(), 1);
    }

    #[test]
    fn test_create_acceptor() {
        let result = create_tls_acceptor();
        assert!(
            result.is_ok(),
            "Failed to create TLS acceptor: {:?}",
            result.err()
        );
    }
}
