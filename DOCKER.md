# Docker Deployment Guide

This guide covers deploying Browser Proxy using Docker.

## Quick Start

### Using Docker Compose (Recommended)

1. **Copy environment file:**
   ```bash
   cp .env.example .env
   ```

2. **Edit `.env` with your settings:**
   ```bash
   nano .env
   ```

   Important settings:
   - `DOMAIN_FILTER_ALLOWLIST`: Comma-separated list of domains (required)
   - `AUTH_PASSWORD`: Change from default `changeme`

3. **Build and start:**
   ```bash
   docker-compose up -d
   ```

4. **View logs:**
   ```bash
   docker-compose logs -f browser_proxy
   ```

5. **Access the application:**
   ```
   http://localhost:3000
   ```

6. **Stop:**
   ```bash
   docker-compose down
   ```

### Using Docker Directly

**Build the image:**
```bash
docker build -t browser_proxy:latest .
```

**Run the container:**
```bash
docker run -d \
  --name browser_proxy \
  -p 3000:3000 \
  -e SERVER_HOST=0.0.0.0 \
  -e SERVER_PORT=3000 \
  -e AUTH_USERNAME=admin \
  -e AUTH_PASSWORD=your-secure-password \
  -e DOMAIN_FILTER_ALLOWLIST=example.com,cdn.example.com \
  browser_proxy:latest
```

**View logs:**
```bash
docker logs -f browser_proxy
```

**Stop and remove:**
```bash
docker stop browser_proxy
docker rm browser_proxy
```

## Configuration

### Environment Variables

All configuration can be provided via environment variables:

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `SERVER_HOST` | Server bind address | `0.0.0.0` | No |
| `SERVER_PORT` | Server port | `3000` | No |
| `AUTH_USERNAME` | Login username | `admin` | No |
| `AUTH_PASSWORD` | Login password | `changeme` | **Yes** |
| `DOMAIN_FILTER_ALLOWLIST` | Allowed domains (comma-separated) | - | **Yes** |
| `DOMAIN_FILTER_BLOCKLIST` | Blocked domains (comma-separated) | - | No |
| `LOGGING_LEVEL` | Log level (trace/debug/info/warn/error) | `info` | No |
| `LOGGING_FORMAT` | Log format (pretty/json) | `pretty` | No |
| `LOGGING_LOG_REQUESTS` | Enable request logging | `true` | No |

### Example .env File

```bash
# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=3000

# Authentication - CHANGE THESE!
AUTH_USERNAME=admin
AUTH_PASSWORD=your-secure-password-here

# Domain Filter - REQUIRED
DOMAIN_FILTER_ALLOWLIST=example.com,cdn.example.com,fonts.googleapis.com

# Optional: Block specific domains
DOMAIN_FILTER_BLOCKLIST=ads.example.com,tracking.example.com

# Logging
LOGGING_LEVEL=info
LOGGING_FORMAT=pretty
LOGGING_LOG_REQUESTS=true
```

## Using config.toml in Docker

You can also use a `config.toml` file instead of environment variables:

1. Create `config.toml` with your settings
2. Mount it into the container:

**Docker Compose:**
```yaml
volumes:
  - ./config.toml:/app/config.toml:ro
```

**Docker run:**
```bash
docker run -d \
  --name browser_proxy \
  -p 3000:3000 \
  -v $(pwd)/config.toml:/app/config.toml:ro \
  browser_proxy:latest
```

## Deployment Examples

### Behind Nginx Reverse Proxy

**nginx.conf:**
```nginx
server {
    listen 80;
    server_name proxy.example.com;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### With HTTPS (Using Caddy)

**Caddyfile:**
```
proxy.example.com {
    reverse_proxy localhost:3000
}
```

### Production Docker Compose

**docker-compose.prod.yml:**
```yaml
version: '3.8'

services:
  browser_proxy:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: browser_proxy
    restart: always
    ports:
      - "127.0.0.1:3000:3000"  # Bind only to localhost
    env_file:
      - .env
    networks:
      - proxy_network
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

networks:
  proxy_network:
    driver: bridge
```

**Run:**
```bash
docker-compose -f docker-compose.prod.yml up -d
```

## Troubleshooting

### Container won't start

**Check logs:**
```bash
docker-compose logs browser_proxy
```

Common issues:
- **"Allowlist cannot be empty"**: Add at least one domain to `DOMAIN_FILTER_ALLOWLIST`
- **Port already in use**: Change `SERVER_PORT` in `.env`

### Building fails

**Clear build cache:**
```bash
docker-compose build --no-cache
```

### Can't connect to proxy

1. Check if container is running:
   ```bash
   docker ps | grep browser_proxy
   ```

2. Check port binding:
   ```bash
   docker port browser_proxy
   ```

3. Test from host:
   ```bash
   curl http://localhost:3000/
   ```

### Logs show blocked domains

This is normal! The allowlist-only mode blocks all domains not explicitly allowed.

**To fix:**
1. Check logs for blocked domain names
2. Add needed domains to `DOMAIN_FILTER_ALLOWLIST`
3. Restart container:
   ```bash
   docker-compose restart
   ```

## Updating

**Using Docker Compose:**
```bash
git pull
docker-compose build
docker-compose up -d
```

**Using Docker directly:**
```bash
git pull
docker build -t browser_proxy:latest .
docker stop browser_proxy
docker rm browser_proxy
# Run command from above
```

## Security Best Practices

1. **Change Default Password**: Never use `changeme` in production
2. **Use HTTPS**: Deploy behind a reverse proxy with SSL/TLS
3. **Restrict Access**: Bind to localhost (`127.0.0.1`) and use reverse proxy, or use firewall rules
4. **Keep Updated**: Regularly pull updates and rebuild
5. **Monitor Logs**: Review logs for suspicious activity
6. **Limit Allowlist**: Only add domains you trust

## Persistence

**Note:** Session data is stored in memory and will be lost on container restart. For production with multiple instances, consider:

1. Using sticky sessions in your load balancer
2. Implementing a persistent session store (Redis, PostgreSQL)
3. Using JWT tokens instead of server-side sessions

## Performance Tuning

For high-traffic deployments:

1. **Increase container resources:**
   ```yaml
   deploy:
     resources:
       limits:
         cpus: '2'
         memory: 2G
   ```

2. **Use production logging format:**
   ```bash
   LOGGING_FORMAT=json
   ```

3. **Adjust Rust stack size if needed** (in Dockerfile):
   ```dockerfile
   ENV RUST_MIN_STACK=8388608
   ```

## Monitoring

**View resource usage:**
```bash
docker stats browser_proxy
```

**Export logs:**
```bash
docker logs browser_proxy > proxy.log 2>&1
```

**Health check:**
```bash
curl -f http://localhost:3000/ || echo "Health check failed"
```
