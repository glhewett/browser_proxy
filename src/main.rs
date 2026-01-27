use axum::{
    response::Redirect,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_sessions::{MemoryStore, SessionManagerLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod middleware;
mod proxy;
mod routes;

use config::Config;
use middleware::{logging_middleware, DomainFilter};
use routes::{browse_handler, home_page, login_handler, login_page, proxy_handler, require_auth};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub client: reqwest::Client,
    pub domain_filter: Arc<DomainFilter>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Load configuration
    let config = Config::load("config.toml")?;

    // 2. Validate configuration (allowlist not empty)
    if config.domain_filter.allowlist.is_empty() {
        anyhow::bail!("Error: Allowlist cannot be empty. Add at least one domain to config.toml");
    }

    // 3. Setup logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}={}", env!("CARGO_PKG_NAME"), config.logging.level).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting browser_proxy server");
    tracing::info!("Loaded configuration:");
    tracing::info!("  Allowed domains: {:?}", config.domain_filter.allowlist);
    if !config.domain_filter.blocklist.is_empty() {
        tracing::info!("  Blocked domains: {:?}", config.domain_filter.blocklist);
    }

    // 4. Create domain filter
    let domain_filter = Arc::new(DomainFilter::new(&config.domain_filter)?);

    // 5. Create HTTP client for proxying
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    // 6. Create application state
    let state = Arc::new(AppState {
        config: config.clone(),
        client,
        domain_filter,
    });

    // 7. Setup session layer
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false); // Allow cookies over HTTP (set true if behind HTTPS proxy)

    // 8. Build router
    // Public routes
    let public_routes = Router::new()
        .route("/", get(|| async { Redirect::to("/login") }))
        .route("/login", get(login_page).post(login_handler));

    // Protected routes
    let protected_routes = Router::new()
        .route("/home", get(home_page))
        .route("/browse", post(browse_handler))
        .route("/proxy/:scheme/*path", get(proxy_handler))
        .route_layer(axum::middleware::from_fn(require_auth));

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(session_layer)
        .layer(axum::middleware::from_fn(logging_middleware))
        .with_state(state);

    // 9. Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
