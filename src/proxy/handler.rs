use anyhow::Result;
use async_trait::async_trait;
use reqwest::Response;
use url::Url;

#[async_trait]
pub trait ProxyHandler: Send + Sync {
    async fn handle(
        &self,
        response: Response,
        proxy_base_url: &str,
        original_url: &Url,
    ) -> Result<(Vec<u8>, String)>;
}
