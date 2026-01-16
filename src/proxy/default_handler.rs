use anyhow::Result;
use async_trait::async_trait;
use reqwest::Response;
use url::Url;

use super::handler::ProxyHandler;

pub struct DefaultProxyHandler;

#[async_trait]
impl ProxyHandler for DefaultProxyHandler {
    async fn handle(
        &self,
        response: Response,
        _proxy_base_url: &str,
        _original_url: &Url,
    ) -> Result<(Vec<u8>, String)> {
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        // Simple pass-through for non-HTML content
        let bytes = response.bytes().await?.to_vec();

        tracing::debug!(
            "DefaultProxyHandler: processed {} bytes of {}",
            bytes.len(),
            content_type
        );

        Ok((bytes, content_type))
    }
}
