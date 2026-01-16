use anyhow::{bail, Result};
use url::Url;
use wildmatch::WildMatch;

use crate::config::DomainFilterConfig;

pub struct DomainFilter {
    allowlist: Vec<WildMatch>,
    blocklist: Vec<WildMatch>,
}

impl DomainFilter {
    pub fn new(config: &DomainFilterConfig) -> Result<Self> {
        // Validate: allowlist must not be empty
        if config.allowlist.is_empty() {
            bail!("Allowlist cannot be empty. Add at least one domain to config.toml");
        }

        Ok(Self {
            allowlist: config.allowlist.iter().map(|p| WildMatch::new(p)).collect(),
            blocklist: config.blocklist.iter().map(|p| WildMatch::new(p)).collect(),
        })
    }

    pub fn is_allowed(&self, domain: &str) -> bool {
        // 1. Check blocklist first - blocklist always takes precedence
        if self.blocklist.iter().any(|pattern| pattern.matches(domain)) {
            tracing::warn!("Domain blocked by blocklist: {}", domain);
            return false;
        }

        // 2. Check if domain is in allowlist (required)
        let allowed = self.allowlist.iter().any(|pattern| pattern.matches(domain));

        if !allowed {
            tracing::warn!("Domain not in allowlist: {}", domain);
        }

        allowed
    }

    pub fn validate_start_url(&self, url: &Url) -> Result<()> {
        let domain = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?;

        if !self.is_allowed(domain) {
            bail!(
                "Domain '{}' is not in allowlist. Add it to config.toml to proxy this site.",
                domain
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_allowlist_rejected() {
        let config = DomainFilterConfig {
            allowlist: vec![],
            blocklist: vec![],
        };

        let result = DomainFilter::new(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_exact_match() {
        let config = DomainFilterConfig {
            allowlist: vec!["example.com".to_string()],
            blocklist: vec![],
        };

        let filter = DomainFilter::new(&config).unwrap();

        assert!(filter.is_allowed("example.com"));
        assert!(!filter.is_allowed("other.com"));
    }

    #[test]
    fn test_wildcard_subdomain() {
        let config = DomainFilterConfig {
            allowlist: vec!["*.example.com".to_string()],
            blocklist: vec![],
        };

        let filter = DomainFilter::new(&config).unwrap();

        assert!(filter.is_allowed("sub.example.com"));
        assert!(filter.is_allowed("deep.sub.example.com"));
        assert!(!filter.is_allowed("example.com")); // Wildcard doesn't match base domain
    }

    #[test]
    fn test_blocklist_priority() {
        let config = DomainFilterConfig {
            allowlist: vec!["*.example.com".to_string()],
            blocklist: vec!["ads.example.com".to_string()],
        };

        let filter = DomainFilter::new(&config).unwrap();

        assert!(filter.is_allowed("www.example.com"));
        assert!(!filter.is_allowed("ads.example.com")); // Blocked even though matches allowlist
    }

    #[test]
    fn test_validate_start_url() {
        let config = DomainFilterConfig {
            allowlist: vec!["example.com".to_string()],
            blocklist: vec![],
        };

        let filter = DomainFilter::new(&config).unwrap();

        let allowed_url = Url::parse("https://example.com/path").unwrap();
        assert!(filter.validate_start_url(&allowed_url).is_ok());

        let blocked_url = Url::parse("https://other.com/path").unwrap();
        assert!(filter.validate_start_url(&blocked_url).is_err());
    }
}
