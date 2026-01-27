use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub domain_filter: DomainFilterConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DomainFilterConfig {
    #[serde(default)]
    pub allowlist: Vec<String>,
    #[serde(default)]
    pub blocklist: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub log_requests: bool,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        // Try to load from file first
        if std::path::Path::new(path).is_file() {
            let content = std::fs::read_to_string(path)?;
            let config: Config = toml::from_str(&content)?;
            return Ok(config);
        }

        // Fall back to environment variables (for Docker)
        tracing::info!("Config file not found, loading from environment variables");
        Self::from_env()
    }

    pub fn from_env() -> Result<Self> {
        let allowlist_str = env::var("DOMAIN_FILTER_ALLOWLIST").unwrap_or_default();
        let allowlist: Vec<String> = allowlist_str
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string())
            .collect();

        let blocklist_str = env::var("DOMAIN_FILTER_BLOCKLIST").unwrap_or_default();
        let blocklist: Vec<String> = blocklist_str
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string())
            .collect();

        Ok(Config {
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "3000".to_string())
                    .parse()?,
            },
            auth: AuthConfig {
                username: env::var("AUTH_USERNAME").unwrap_or_else(|_| "admin".to_string()),
                password: env::var("AUTH_PASSWORD").unwrap_or_else(|_| "changeme".to_string()),
            },
            domain_filter: DomainFilterConfig {
                allowlist,
                blocklist,
            },
            logging: LoggingConfig {
                level: env::var("LOGGING_LEVEL").unwrap_or_else(|_| "info".to_string()),
                format: env::var("LOGGING_FORMAT").unwrap_or_else(|_| "pretty".to_string()),
                log_requests: env::var("LOGGING_LOG_REQUESTS")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
            },
        })
    }
}
