use anyhow::anyhow;
use confique::Config;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use serde::Deserialize;
use std::path::PathBuf;

pub static CONFIG_PATH: &'static str = "/etc/jorge-a-day/config.toml";

#[derive(Config, Deserialize, Clone)]
pub struct AppConfig {
    #[config(default = ["."])]
    pub directories: Vec<String>,

    #[config(default = 300)]
    pub cache_age: i64,

    #[config(default = "0.0.0.0:8443")]
    pub address: String,

    #[config(default = false)]
    pub ssl: bool,

    #[config()]
    pub cert: Option<String>,

    #[config()]
    pub key: Option<String>,
}

impl AppConfig {
    pub fn check(&self) -> anyhow::Result<(String, String)> {
        let cert_missing = self.cert.is_none();
        let key_missing = self.key.is_none();

        if self.ssl && (cert_missing || key_missing) {
            let mut missing = Vec::new();
            if cert_missing {
                missing.push("certificate");
            }
            if key_missing {
                missing.push("key");
            }
            anyhow::bail!("SSL is enabled but missing {}!", missing.join(" and "));
        }

        let cert = self
            .cert
            .clone()
            .ok_or_else(|| anyhow!("Certificate is missing"))?;
        let key = self.key.clone().ok_or_else(|| anyhow!("Key is missing"))?;

        Ok((cert, key))
    }
}

// Takes in the certificate and key and generates an openssl instance.
// Most likely fail cause is missing certificates or incorrect permissions.
pub fn create_ssl_builder(
    cert_path: &str,
    key_path: &str,
) -> anyhow::Result<openssl::ssl::SslAcceptorBuilder> {
    match (
        PathBuf::from(&cert_path).exists(),
        PathBuf::from(&key_path).exists(),
    ) {
        (false, _) => return Err(anyhow!("Certificate does not exist.")),
        (_, false) => return Err(anyhow!("Certificate does not exist.")),
        (_, _) => {}
    }

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;

    builder.set_private_key_file(key_path, SslFiletype::PEM)?;
    builder.set_certificate_chain_file(cert_path)?;

    Ok(builder)
}
