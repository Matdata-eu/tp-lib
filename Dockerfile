# ============================================================================
# Stage 1: Builder - Full build environment
# ============================================================================
FROM rust:1.91-bookworm as builder

# Set working directory
WORKDIR /build

# Copy dependency manifests first (for layer caching)
COPY Cargo.toml Cargo.lock ./
COPY tp-core/Cargo.toml tp-core/
COPY tp-cli/Cargo.toml tp-cli/
COPY tp-py/Cargo.toml tp-py/

# Create dummy source files to build dependencies
RUN mkdir -p tp-core/src tp-core/benches tp-cli/src tp-py/src && \
    echo "fn main() {}" > tp-cli/src/main.rs && \
    echo "fn main() {}" > tp-py/src/lib.rs && \
    echo "pub fn dummy() {}" > tp-core/src/lib.rs && \
    echo "fn main() {}" > tp-core/benches/projection_bench.rs && \
    echo "fn main() {}" > tp-core/benches/naive_baseline_bench.rs

# Build dependencies (exclude benches in dummy build)
ENV CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_BENCH=false
RUN cargo build --release -p tp-cli -p tp-core

# Remove dummy sources
RUN rm -rf tp-core/src tp-core/benches tp-cli/src tp-py/src

# Copy actual source code
COPY tp-core/ tp-core/
COPY tp-cli/ tp-cli/
COPY tp-py/ tp-py/

# Build the actual application
RUN cargo build --release -p tp-cli && \
    strip /build/target/release/tp-cli

# ============================================================================
# Stage 2: Runtime - Minimal image
# ============================================================================
FROM debian:bookworm-slim

# Install only ca-certificates for HTTPS
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled binary from builder
COPY --from=builder /build/target/release/tp-cli /usr/local/bin/tp-cli

# Create non-root user
RUN useradd -m -u 1000 tplib && \
    mkdir -p /data && \
    chown -R tplib:tplib /data

USER tplib
WORKDIR /data

# Verify installation
RUN tp-cli --version

# Default command
ENTRYPOINT ["tp-cli"]
CMD ["--help"]

# ============================================================================
# Build instructions:
#   docker build -t tp-lib:latest .
#   
# Run examples:
#   docker run --rm tp-lib:latest --help
#   docker run --rm -v $(pwd)/data:/data tp-lib:latest \
#     --gnss-file /data/gnss.csv \
#     --crs EPSG:4326 \
#     --network-file /data/network.geojson \
#     --output-format csv
# ============================================================================
