use axum::{
    body::Body,
    extract::{Host, Path, State},
    http::{Response, StatusCode},
    response::IntoResponse,
};
use std::sync::Arc;
use url::Url;

use crate::proxy::get_handler;
use crate::AppState;

pub async fn proxy_handler(
    State(state): State<Arc<AppState>>,
    Host(host): Host,
    Path((scheme, target_path)): Path<(String, String)>,
) -> impl IntoResponse {
    // 1. Construct target URL from scheme and path
    let target_url = format!("{}://{}", scheme, target_path);
    tracing::info!("Proxying request to: {}", target_url);

    let url = match Url::parse(&target_url) {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("Invalid URL: {}", e);
            return (StatusCode::BAD_REQUEST, format!("Invalid URL: {}", e)).into_response();
        }
    };

    let domain = url.host_str().unwrap_or("");

    // 2. Check domain filter
    if !state.domain_filter.is_allowed(domain) {
        tracing::warn!("Domain blocked: {}", domain);
        return (
            StatusCode::FORBIDDEN,
            format!("Domain '{}' is not allowed", domain),
        )
            .into_response();
    }

    // 3. Make request to target URL
    let response = match state.client.get(&target_url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to fetch: {}", e);
            return (StatusCode::BAD_GATEWAY, format!("Failed to fetch: {}", e)).into_response();
        }
    };

    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");

    tracing::debug!(
        "Response status: {}, content-type: {}",
        status,
        content_type
    );

    // 4. Select appropriate handler based on content-type
    let handler = get_handler(content_type);

    // 5. Process response with handler
    let proxy_base = format!("http://{}/proxy", host);
    let (body, content_type) = match handler.handle(response, &proxy_base, &url).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Processing error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Processing error: {}", e),
            )
                .into_response();
        }
    };

    // 6. Build response
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", content_type)
        .body(Body::from(body))
        .unwrap()
        .into_response()
}
