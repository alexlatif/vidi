//! Server configuration

use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use rustls::pki_types::CertificateDer;

/// Vidi XP Dashboard Server
#[derive(Parser, Clone, Debug)]
#[command(name = "vidi-server")]
#[command(about = "A server for hosting Vidi dashboards with real-time streaming")]
pub struct Config {
    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// Port to listen on
    #[arg(short, long, default_value = "8080")]
    pub port: u16,

    /// Path to SQLite database
    #[arg(long, default_value = "dashboards.db")]
    pub db_path: String,

    /// Path to static files directory
    #[arg(long, default_value = "vidi-server/static")]
    pub static_dir: String,

    /// Path to WASM artifacts directory
    #[arg(long, default_value = "vidi-server/wasm")]
    pub wasm_dir: String,

    /// TLS certificate path (PEM format)
    #[arg(long)]
    pub tls_cert: Option<String>,

    /// TLS private key path (PEM format)
    #[arg(long)]
    pub tls_key: Option<String>,

    /// Default TTL for temporary dashboards in seconds
    #[arg(long, default_value = "86400")]
    pub default_ttl: u64,

    /// Cleanup interval in seconds
    #[arg(long, default_value = "300")]
    pub cleanup_interval: u64,
}

/// Load TLS configuration from cert and key files
pub fn load_tls_config(cert_path: &str, key_path: &str) -> anyhow::Result<RustlsConfig> {
    let cert_file = File::open(cert_path)?;
    let key_file = File::open(key_path)?;

    let mut cert_reader = BufReader::new(cert_file);
    let mut key_reader = BufReader::new(key_file);

    let certs: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut cert_reader).collect::<Result<Vec<_>, _>>()?;

    let key = rustls_pemfile::private_key(&mut key_reader)?
        .ok_or_else(|| anyhow::anyhow!("No private key found in {}", key_path))?;

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(RustlsConfig::from_config(Arc::new(config)))
}
