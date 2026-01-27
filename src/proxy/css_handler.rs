use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use reqwest::Response;
use url::Url;

use super::handler::ProxyHandler;

pub struct CssProxyHandler;

#[async_trait]
impl ProxyHandler for CssProxyHandler {
    async fn handle(
        &self,
        response: Response,
        proxy_base_url: &str,
        original_url: &Url,
    ) -> Result<(Vec<u8>, String)> {
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/css")
            .to_string();

        let css = response.text().await?;

        tracing::debug!("CssProxyHandler: processing CSS from {}", original_url);

        let rewritten = self.rewrite_urls(&css, proxy_base_url, original_url)?;

        Ok((rewritten.into_bytes(), content_type))
    }
}

impl CssProxyHandler {
    fn rewrite_urls(&self, css: &str, proxy_base: &str, original_url: &Url) -> Result<String> {
        // Match url() with optional quotes (single, double, or none)
        // Three separate patterns since regex crate doesn't support backreferences
        let patterns = [
            (Regex::new(r#"url\(\s*"([^"]+)"\s*\)"#)?, "\""),  // double quotes
            (Regex::new(r#"url\(\s*'([^']+)'\s*\)"#)?, "'"),   // single quotes
            (Regex::new(r#"url\(\s*([^'"\s)]+)\s*\)"#)?, ""),  // no quotes
        ];

        let mut result = css.to_string();
        let mut replacements: Vec<(usize, usize, String)> = Vec::new();

        for (url_regex, quote) in &patterns {
            for cap in url_regex.captures_iter(css) {
                let full_match = cap.get(0).unwrap();
                let url_value = cap.get(1).map(|m| m.as_str()).unwrap_or("");

                // Skip data URLs, fragments, and empty values
                if url_value.starts_with("data:")
                    || url_value.starts_with('#')
                    || url_value.is_empty()
                {
                    continue;
                }

                if let Some(rewritten_url) =
                    self.rewrite_single_url(url_value, proxy_base, original_url)
                {
                    let new_url_expr = format!("url({}{}{})", quote, rewritten_url, quote);
                    replacements.push((full_match.start(), full_match.end(), new_url_expr));
                }
            }
        }

        // Sort by position descending to maintain correct indices during replacement
        replacements.sort_by(|a, b| b.0.cmp(&a.0));

        let num_replacements = replacements.len();

        for (start, end, new_value) in replacements {
            result.replace_range(start..end, &new_value);
        }

        tracing::debug!("CssProxyHandler: rewrote {} URLs", num_replacements);

        Ok(result)
    }

    fn rewrite_single_url(
        &self,
        url_value: &str,
        proxy_base: &str,
        original_url: &Url,
    ) -> Option<String> {
        // Handle protocol-relative URLs (//example.com/path)
        if let Some(stripped) = url_value.strip_prefix("//") {
            let scheme = original_url.scheme();
            return Some(format!("{}/{}/{}", proxy_base, scheme, stripped));
        }

        // Handle absolute URLs (http://example.com/path or https://example.com/path)
        if (url_value.starts_with("http://") || url_value.starts_with("https://"))
            && let Ok(parsed) = Url::parse(url_value)
        {
            let scheme = parsed.scheme();
            let host = parsed.host_str()?;
            let port = if let Some(p) = parsed.port() {
                format!(":{}", p)
            } else {
                String::new()
            };
            let path = parsed.path();
            let query = if let Some(q) = parsed.query() {
                format!("?{}", q)
            } else {
                String::new()
            };
            let fragment = if let Some(f) = parsed.fragment() {
                format!("#{}", f)
            } else {
                String::new()
            };
            return Some(format!(
                "{}/{}/{}{}{}{}{}",
                proxy_base, scheme, host, port, path, query, fragment
            ));
        }

        // Handle root-relative paths (/path)
        if url_value.starts_with('/') {
            let scheme = original_url.scheme();
            let host = original_url.host_str()?;
            let port = if let Some(p) = original_url.port() {
                format!(":{}", p)
            } else {
                String::new()
            };
            return Some(format!(
                "{}/{}/{}{}{}",
                proxy_base, scheme, host, port, url_value
            ));
        }

        // Handle relative URLs (path/to/resource)
        if let Ok(absolute) = original_url.join(url_value) {
            let scheme = absolute.scheme();
            let host = absolute.host_str()?;
            let port = if let Some(p) = absolute.port() {
                format!(":{}", p)
            } else {
                String::new()
            };
            let path = absolute.path();
            let query = if let Some(q) = absolute.query() {
                format!("?{}", q)
            } else {
                String::new()
            };
            let fragment = if let Some(f) = absolute.fragment() {
                format!("#{}", f)
            } else {
                String::new()
            };
            return Some(format!(
                "{}/{}/{}{}{}{}{}",
                proxy_base, scheme, host, port, path, query, fragment
            ));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_absolute_url_in_css() {
        let handler = CssProxyHandler;
        let original_url = Url::parse("https://example.com/css/style.css").unwrap();
        let proxy_base = "http://localhost:3000/proxy";

        let result =
            handler.rewrite_single_url("https://cdn.example.com/font.woff2", proxy_base, &original_url);
        assert_eq!(
            result,
            Some("http://localhost:3000/proxy/https/cdn.example.com/font.woff2".to_string())
        );
    }

    #[test]
    fn test_rewrite_root_relative_url_in_css() {
        let handler = CssProxyHandler;
        let original_url = Url::parse("https://example.com/css/style.css").unwrap();
        let proxy_base = "http://localhost:3000/proxy";

        let result = handler.rewrite_single_url("/images/bg.png", proxy_base, &original_url);
        assert_eq!(
            result,
            Some("http://localhost:3000/proxy/https/example.com/images/bg.png".to_string())
        );
    }

    #[test]
    fn test_rewrite_relative_url_in_css() {
        let handler = CssProxyHandler;
        let original_url = Url::parse("https://example.com/css/style.css").unwrap();
        let proxy_base = "http://localhost:3000/proxy";

        let result = handler.rewrite_single_url("../images/bg.png", proxy_base, &original_url);
        assert_eq!(
            result,
            Some("http://localhost:3000/proxy/https/example.com/images/bg.png".to_string())
        );
    }

    #[test]
    fn test_rewrite_protocol_relative_url_in_css() {
        let handler = CssProxyHandler;
        let original_url = Url::parse("https://example.com/css/style.css").unwrap();
        let proxy_base = "http://localhost:3000/proxy";

        let result =
            handler.rewrite_single_url("//fonts.example.com/font.woff2", proxy_base, &original_url);
        assert_eq!(
            result,
            Some("http://localhost:3000/proxy/https/fonts.example.com/font.woff2".to_string())
        );
    }

    #[test]
    fn test_skip_data_url_in_css() {
        let handler = CssProxyHandler;
        let original_url = Url::parse("https://example.com/css/style.css").unwrap();
        let proxy_base = "http://localhost:3000/proxy";

        let result =
            handler.rewrite_single_url("data:image/png;base64,iVBOR", proxy_base, &original_url);
        assert_eq!(result, None);
    }

    #[test]
    fn test_rewrite_full_css() {
        let handler = CssProxyHandler;
        let original_url = Url::parse("https://example.com/css/style.css").unwrap();
        let proxy_base = "http://localhost:3000/proxy";

        let css = r#"
            .bg { background: url('/images/bg.png'); }
            .font { src: url("https://fonts.example.com/font.woff2"); }
            .icon { background-image: url(../icons/icon.svg); }
            .data { background: url(data:image/png;base64,abc); }
        "#;

        let result = handler.rewrite_urls(css, proxy_base, &original_url).unwrap();

        assert!(result.contains("url('http://localhost:3000/proxy/https/example.com/images/bg.png')"));
        assert!(result.contains("url(\"http://localhost:3000/proxy/https/fonts.example.com/font.woff2\")"));
        assert!(result.contains("url(http://localhost:3000/proxy/https/example.com/icons/icon.svg)"));
        // Data URL should remain unchanged
        assert!(result.contains("url(data:image/png;base64,abc)"));
    }
}
