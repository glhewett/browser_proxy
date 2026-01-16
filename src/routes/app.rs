use askama::Template;
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use std::sync::Arc;
use tower_sessions::Session;
use url::Url;

use crate::AppState;

const USER_ID_KEY: &str = "user_id";

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: String,
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    allowed_domains: Vec<String>,
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    error_message: String,
    blocked_domain: String,
    allowed_domains: Vec<String>,
}

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

#[derive(Deserialize)]
pub struct BrowseForm {
    url: String,
}

pub async fn login_page() -> impl IntoResponse {
    Html(
        LoginTemplate {
            error: String::new(),
        }
        .render()
        .unwrap(),
    )
}

pub async fn login_handler(
    session: Session,
    State(state): State<Arc<AppState>>,
    Form(credentials): Form<LoginForm>,
) -> impl IntoResponse {
    // Validate credentials
    if credentials.username == state.config.auth.username
        && credentials.password == state.config.auth.password
    {
        // Store user in session
        if let Err(e) = session.insert(USER_ID_KEY, &credentials.username).await {
            tracing::error!("Failed to create session: {}", e);
            return Html(
                LoginTemplate {
                    error: "Failed to create session".to_string(),
                }
                .render()
                .unwrap(),
            )
            .into_response();
        }

        tracing::info!("User {} logged in", credentials.username);
        Redirect::to("/home").into_response()
    } else {
        tracing::warn!("Failed login attempt for user: {}", credentials.username);
        Html(
            LoginTemplate {
                error: "Invalid username or password".to_string(),
            }
            .render()
            .unwrap(),
        )
        .into_response()
    }
}

pub async fn home_page(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Html(
        HomeTemplate {
            allowed_domains: state.config.domain_filter.allowlist.clone(),
        }
        .render()
        .unwrap(),
    )
}

pub async fn browse_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<BrowseForm>,
) -> impl IntoResponse {
    // Parse URL
    let url = match Url::parse(&form.url) {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("Invalid URL: {}", e);
            return Html(
                ErrorTemplate {
                    error_message: format!("Invalid URL: {}", e),
                    blocked_domain: String::new(),
                    allowed_domains: state.config.domain_filter.allowlist.clone(),
                }
                .render()
                .unwrap(),
            )
            .into_response();
        }
    };

    // Validate against allowlist
    if let Err(e) = state.domain_filter.validate_start_url(&url) {
        tracing::warn!("URL not in allowlist: {}", form.url);
        return Html(
            ErrorTemplate {
                error_message: e.to_string(),
                blocked_domain: url.host_str().unwrap_or("").to_string(),
                allowed_domains: state.config.domain_filter.allowlist.clone(),
            }
            .render()
            .unwrap(),
        )
        .into_response();
    }

    // Redirect to proxy
    let scheme = url.scheme();
    let path = format!("{}{}", url.host_str().unwrap_or(""), url.path());
    let query = if let Some(q) = url.query() {
        format!("?{}", q)
    } else {
        String::new()
    };

    Redirect::to(&format!("/proxy/{}/{}{}", scheme, path, query)).into_response()
}

// Auth middleware
pub async fn require_auth(
    session: Session,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    let user_id: Option<String> = session
        .get(USER_ID_KEY)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if user_id.is_some() {
        Ok(next.run(request).await)
    } else {
        Ok(Redirect::to("/login").into_response())
    }
}
