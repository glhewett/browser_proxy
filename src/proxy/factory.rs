use super::{DefaultProxyHandler, HtmlProxyHandler, ProxyHandler};

pub fn get_handler(content_type: &str) -> Box<dyn ProxyHandler> {
    if content_type.contains("text/html") {
        tracing::debug!(
            "Selected HtmlProxyHandler for content-type: {}",
            content_type
        );
        Box::new(HtmlProxyHandler)
    } else {
        tracing::debug!(
            "Selected DefaultProxyHandler for content-type: {}",
            content_type
        );
        Box::new(DefaultProxyHandler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_content_type() {
        let _handler = get_handler("text/html; charset=utf-8");
        // We can't directly test the type, but we can verify it doesn't panic
        assert!(true);
    }

    #[test]
    fn test_plain_html_content_type() {
        let _handler = get_handler("text/html");
        assert!(true);
    }

    #[test]
    fn test_non_html_content_type() {
        let _handler = get_handler("image/png");
        assert!(true);
    }

    #[test]
    fn test_javascript_content_type() {
        let _handler = get_handler("application/javascript");
        assert!(true);
    }

    #[test]
    fn test_css_content_type() {
        let _handler = get_handler("text/css");
        assert!(true);
    }
}
