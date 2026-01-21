# Stage 1: Build Rust binary and WASM
FROM rust:latest AS builder

# Install build dependencies for Bevy (wayland, x11, etc.)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libwayland-dev \
    libxkbcommon-dev \
    libasound2-dev \
    libudev-dev \
    libx11-dev \
    libxi-dev \
    libgl1-mesa-dev \
    && rm -rf /var/lib/apt/lists/*

# Install wasm tools
RUN rustup target add wasm32-unknown-unknown
RUN cargo install wasm-bindgen-cli --version 0.2.106

WORKDIR /app

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./
COPY vidi-server/Cargo.toml ./vidi-server/
COPY vidi-server/dashboard-template/Cargo.toml ./vidi-server/dashboard-template/

# Create dummy src files to build dependencies
RUN mkdir -p src && echo "pub fn dummy() {}" > src/lib.rs
RUN mkdir -p vidi-server/src && echo "fn main() {}" > vidi-server/src/main.rs
RUN mkdir -p vidi-server/dashboard-template/src && echo "pub fn dummy() {}" > vidi-server/dashboard-template/src/lib.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release -p vidi-server 2>/dev/null || true
RUN cargo build --release --target wasm32-unknown-unknown 2>/dev/null || true

# Now copy actual source code
COPY . .

# Touch source files to invalidate the previous dummy build
RUN touch src/lib.rs vidi-server/src/main.rs

# Build server binary
RUN cargo build --release -p vidi-server

# Build WASM
RUN cargo build --release --target wasm32-unknown-unknown
RUN wasm-bindgen target/wasm32-unknown-unknown/release/vidi.wasm \
    --target web \
    --out-dir vidi-server/wasm \
    --no-typescript

# Stage 2: Runtime (use trixie to match rust:latest glibc)
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/vidi-server /app/vidi-server

# Copy static assets
COPY --from=builder /app/vidi-server/static /app/static
COPY --from=builder /app/vidi-server/wasm /app/wasm

# Create data directory for SQLite
RUN mkdir -p /app/data

EXPOSE 8080

# Run with paths adjusted for container
CMD ["./vidi-server", "--port", "8080", "--static-dir", "/app/static", "--wasm-dir", "/app/wasm", "--db-path", "/app/data/dashboards.db"]
