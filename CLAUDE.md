# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Browser Proxy is an allowlist-only HTTP/HTTPS proxy server built in Rust. It intercepts web requests, fetches content from target servers, rewrites HTML URLs to route through itself, and enforces strict domain access controls. This is a modern Rust reimplementation of the original Java browser_vpn application from 2004.

## Common Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release         # Optimized release build

# Test
cargo test                    # Run all tests
cargo test --verbose         # With output
cargo test domain_filter     # Run tests for specific module

# Lint & Format
cargo clippy -- -D warnings   # Lint (CI enforces this strictly)
cargo fmt --all               # Format code

# Security
cargo audit                   # Scan dependencies for vulnerabilities

# Docker
docker-compose up -d          # Start in background
docker-compose logs -f        # Watch logs
docker-compose restart        # Restart after config changes
docker-compose build --no-cache && docker-compose up -d  # Full rebuild
```

## Architecture

### Request Flow

1. User logs in via `/login` (session stored in memory)
2. User submits URL via `/browse` form on `/home`
3. Proxy validates domain against allowlist (blocklist has priority)
4. Request made to target via reqwest (redirects disabled, handled manually)
5. Response processed: HTML goes through URL rewriting, other content passes through
6. Rewritten response returned to user

### Key Modules

```
src/
├── main.rs              # Entry point, Axum router setup, app state initialization
├── config.rs            # TOML or environment variable configuration loading
├── routes/
│   ├── app.rs          # Login, home, browse form handlers
│   └── proxy.rs        # Core proxy handler: /proxy/:scheme/*path
├── proxy/
│   ├── handler.rs      # ProxyHandler trait (async)
│   ├── factory.rs      # Routes HTML vs other content to appropriate handler
│   ├── html_handler.rs # HTML URL rewriting using scraper (not regex)
│   └── default_handler.rs # Pass-through for non-HTML content
└── middleware/
    ├── domain_filter.rs # Allowlist/blocklist enforcement with wildmatch
    └── logging.rs       # Request/response timing
```

### URL Rewriting Strategy

The HTML handler (`proxy/html_handler.rs`) uses the `scraper` library for proper HTML5 parsing. It rewrites URLs in these attributes across multiple HTML tags:
- `href` (a, link, area, base)
- `src` (img, script, iframe, embed, source, track, audio, video)
- `action` (form)
- `codebase`, `data`, `poster`

URL types handled:
- Protocol-relative (`//example.com/path`) → `/proxy/https/example.com/path`
- Root-relative (`/path`) → `/proxy/{scheme}/{domain}/path`
- Absolute (`http://example.com/path`) → `/proxy/http/example.com/path`
- Relative (`../path`) → joined with original URL, then rewritten

Skipped: `javascript:`, `data:`, `mailto:`, `tel:`, fragments only, empty strings.

### Domain Filtering

Two-phase check in `middleware/domain_filter.rs`:
1. Blocklist checked first (deny takes precedence)
2. Allowlist checked second (must match to allow)

Supports wildcards via `wildmatch` (e.g., `*.example.com`). Server refuses to start if allowlist is empty.

### Configuration

Two methods, TOML file takes precedence:

**TOML** (`config.toml`):
```toml
[server]
host = "0.0.0.0"
port = 3000

[auth]
username = "admin"
password = "changeme"

[domain_filter]
allowlist = ["example.com"]
blocklist = ["ads.example.com"]

[logging]
level = "info"
```

**Environment variables** (for Docker):
- `SERVER_HOST`, `SERVER_PORT`
- `AUTH_USERNAME`, `AUTH_PASSWORD`
- `DOMAIN_FILTER_ALLOWLIST` (comma-separated)
- `DOMAIN_FILTER_BLOCKLIST` (comma-separated)
- `LOGGING_LEVEL`, `LOGGING_FORMAT`, `LOGGING_LOG_REQUESTS`

### Key Design Decisions

- **No redirect following**: reqwest configured with `redirect::Policy::none()` - proxy handles redirects to maintain URL rewriting
- **In-memory sessions**: Lost on restart; production needs sticky sessions or persistent store
- **rustls over native-tls**: Smaller binary, easier distribution
- **Async-first**: Built on Tokio + Axum for high concurrency
- **Strategy pattern**: ProxyHandler trait enables extensible content handling
