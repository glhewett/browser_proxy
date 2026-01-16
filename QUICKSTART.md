# Browser Proxy - Quick Start Guide

## 1-Minute Setup

### Native (Rust)

```bash
# 1. Copy example config
cp config.toml.example config.toml

# 2. Edit config - ADD YOUR DOMAINS!
nano config.toml
# Change: allowlist = ["your-domain.com"]

# 3. Run
cargo run --release

# 4. Open browser
# http://localhost:3000
# Login: admin / changeme
```

### Docker

```bash
# 1. Copy environment file
cp .env.example .env

# 2. Edit .env - ADD YOUR DOMAINS!
nano .env
# Change: DOMAIN_FILTER_ALLOWLIST=your-domain.com

# 3. Run
docker-compose up -d

# 4. Open browser
# http://localhost:3000
# Login: admin / changeme
```

## First Use

1. **Login** with default credentials
2. **Enter a URL** from your allowlist (e.g., `http://example.com`)
3. **Browse!** All links will route through the proxy

## Common Issues

### "Allowlist cannot be empty"
**Fix**: Add at least one domain to your allowlist

### External resources not loading
**Fix**: Check logs for blocked domains, add them to allowlist, restart

### Login not working
**Fix**: Check username/password in config, clear browser cookies

## Quick Commands

```bash
# Native
cargo run --release              # Start server
cargo test                        # Run tests
cargo build --release            # Build binary

# Docker
docker-compose up -d              # Start
docker-compose logs -f            # View logs
docker-compose restart            # Restart
docker-compose down               # Stop
```

## Files You Need to Edit

1. **config.toml** (native) or **.env** (Docker)
   - Set `allowlist` with your domains
   - Change default password

That's it! See README.md for full documentation.
