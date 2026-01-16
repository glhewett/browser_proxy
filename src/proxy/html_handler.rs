use anyhow::Result;
use async_trait::async_trait;
use reqwest::Response;
use scraper::{Html, Selector};
use url::Url;

use super::handler::ProxyHandler;

pub struct HtmlProxyHandler;

#[async_trait]
impl ProxyHandler for HtmlProxyHandler {
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
            .unwrap_or("text/html")
            .to_string();

        let html = response.text().await?;

        tracing::debug!("HtmlProxyHandler: processing HTML from {}", original_url);

        // Rewrite URLs in HTML
        let rewritten = self.rewrite_urls(&html, proxy_base_url, original_url)?;

        Ok((rewritten.into_bytes(), content_type))
    }
}

impl HtmlProxyHandler {
    fn rewrite_urls(&self, html: &str, proxy_base: &str, original_url: &Url) -> Result<String> {
        let document = Html::parse_document(html);
        let mut modified_html = html.to_string();

        // Define attributes that contain URLs
        let url_attributes = vec![
            ("href", vec!["a", "link", "area", "base"]),
            (
                "src",
                vec![
                    "img", "script", "iframe", "embed", "source", "track", "audio", "video",
                ],
            ),
            ("action", vec!["form"]),
            ("codebase", vec!["object", "applet"]),
            ("data", vec!["object"]),
            ("poster", vec!["video"]),
        ];

        // Collect all URLs to rewrite (from end to start to maintain positions)
        let mut replacements: Vec<(usize, usize, String)> = Vec::new();

        for (attr_name, tag_names) in url_attributes {
            for tag_name in tag_names {
                let selector_str = format!("{}[{}]", tag_name, attr_name);
                let selector = match Selector::parse(&selector_str) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                for element in document.select(&selector) {
                    if let Some(url_value) = element.value().attr(attr_name) {
                        // Skip javascript:, data:, mailto:, tel:, etc.
                        if url_value.starts_with("javascript:")
                            || url_value.starts_with("data:")
                            || url_value.starts_with("mailto:")
                            || url_value.starts_with("tel:")
                            || url_value.starts_with("#")
                            || url_value.is_empty()
                        {
                            continue;
                        }

                        // Rewrite the URL
                        if let Some(rewritten) =
                            self.rewrite_single_url(url_value, proxy_base, original_url)
                        {
                            // Find the position of this attribute in the HTML
                            // We need to find the exact position to replace
                            let search_pattern = format!("{}=\"{}\"", attr_name, url_value);
                            if let Some(pos) = modified_html.find(&search_pattern) {
                                let start = pos + attr_name.len() + 2; // Position after attr="
                                let end = start + url_value.len();
                                replacements.push((start, end, rewritten));
                            }
                        }
                    }
                }
            }
        }

        // Sort replacements by position (descending) to maintain positions
        replacements.sort_by(|a, b| b.0.cmp(&a.0));

        let num_replacements = replacements.len();

        // Apply replacements
        for (start, end, new_url) in replacements {
            modified_html.replace_range(start..end, &new_url);
        }

        tracing::debug!("HtmlProxyHandler: rewrote {} URLs", num_replacements);

        Ok(modified_html)
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

        // Handle absolute URLs (http://example.com/path or https://example.com/path)
        if url_value.starts_with("http://") || url_value.starts_with("https://") {
            if let Ok(parsed) = Url::parse(url_value) {
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
        }

        // Handle relative URLs (path/to/resource)
        if !url_value.starts_with("http") && !url_value.starts_with("//") {
            // Join with original URL to make absolute
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
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_protocol_relative_url() {
        let handler = HtmlProxyHandler;
        let original_url = Url::parse("https://example.com/page").unwrap();
        let proxy_base = "http://localhost:3000/proxy";

        let result =
            handler.rewrite_single_url("//cdn.example.com/script.js", proxy_base, &original_url);
        assert_eq!(
            result,
            Some("http://localhost:3000/proxy/https/cdn.example.com/script.js".to_string())
        );
    }

    #[test]
    fn test_rewrite_root_relative_url() {
        let handler = HtmlProxyHandler;
        let original_url = Url::parse("https://example.com/page").unwrap();
        let proxy_base = "http://localhost:3000/proxy";

        let result = handler.rewrite_single_url("/images/logo.png", proxy_base, &original_url);
        assert_eq!(
            result,
            Some("http://localhost:3000/proxy/https/example.com/images/logo.png".to_string())
        );
    }

    #[test]
    fn test_rewrite_absolute_url() {
        let handler = HtmlProxyHandler;
        let original_url = Url::parse("https://example.com/page").unwrap();
        let proxy_base = "http://localhost:3000/proxy";

        let result = handler.rewrite_single_url("http://other.com/path", proxy_base, &original_url);
        assert_eq!(
            result,
            Some("http://localhost:3000/proxy/http/other.com/path".to_string())
        );
    }

    #[test]
    fn test_skip_javascript_urls() {
        let handler = HtmlProxyHandler;
        let original_url = Url::parse("https://example.com/page").unwrap();
        let proxy_base = "http://localhost:3000/proxy";

        let result = handler.rewrite_single_url("javascript:void(0)", proxy_base, &original_url);
        assert_eq!(result, None);
    }

    #[test]
    fn test_skip_data_urls() {
        let handler = HtmlProxyHandler;
        let original_url = Url::parse("https://example.com/page").unwrap();
        let proxy_base = "http://localhost:3000/proxy";

        let result =
            handler.rewrite_single_url("data:image/png;base64,iVBOR", proxy_base, &original_url);
        assert_eq!(result, None);
    }

    #[test]
    fn test_rewrite_relative_url() {
        let handler = HtmlProxyHandler;
        let original_url = Url::parse("https://example.com/path/page.html").unwrap();
        let proxy_base = "http://localhost:3000/proxy";

        let result = handler.rewrite_single_url("../other.html", proxy_base, &original_url);
        assert_eq!(
            result,
            Some("http://localhost:3000/proxy/https/example.com/other.html".to_string())
        );
    }

    #[test]
    fn test_preserve_query_and_fragment() {
        let handler = HtmlProxyHandler;
        let original_url = Url::parse("https://example.com/page").unwrap();
        let proxy_base = "http://localhost:3000/proxy";

        let result = handler.rewrite_single_url(
            "http://example.com/page?q=test#section",
            proxy_base,
            &original_url,
        );
        assert_eq!(
            result,
            Some("http://localhost:3000/proxy/http/example.com/page?q=test#section".to_string())
        );
    }
}
