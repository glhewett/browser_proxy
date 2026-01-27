# Build stage
FROM rust:1.92 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY src ./src
COPY templates ./templates

# Build release binary
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/browser_proxy /app/browser_proxy

# Copy templates
COPY templates /app/templates

# Create config directory
RUN mkdir -p /app/config

# Expose port (will be overridden by env)
EXPOSE 3000

# Run the binary
CMD ["/app/browser_proxy"]
