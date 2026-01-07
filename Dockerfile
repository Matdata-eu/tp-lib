# ============================================================================
# Stage 1: Builder - Full build environment with all dependencies
# ============================================================================
FROM rust:1.91-bookworm as builder

# Install system dependencies for building
RUN apt-get update && apt-get install -y \
    cmake \
    pkg-config \
    libsqlite3-dev \
    sqlite3 \
    libtiff-dev \
    libcurl4-openssl-dev \
    libclang-dev \
    clang \
    && rm -rf /var/lib/apt/lists/*

# Install PROJ from source (version 9.6.1 for compatibility)
RUN cd /tmp && \
    wget https://download.osgeo.org/proj/proj-9.6.1.tar.gz && \
    tar xzf proj-9.6.1.tar.gz && \
    cd proj-9.6.1 && \
    mkdir build && cd build && \
    cmake .. \
    -DCMAKE_BUILD_TYPE=Release \
    -DBUILD_SHARED_LIBS=ON \
    -DCMAKE_INSTALL_PREFIX=/usr/local && \
    make -j$(nproc) && \
    make install && \
    ldconfig && \
    cd / && rm -rf /tmp/proj-9.6.1*

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

# Build dependencies with crs-transform feature enabled (exclude benches in dummy build)
ENV CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_BENCH=false
ENV LIBCLANG_PATH=/usr/lib/llvm-14/lib
RUN cargo build --release --features crs-transform -p tp-cli -p tp-core

# Remove dummy sources
RUN rm -rf tp-core/src tp-core/benches tp-cli/src tp-py/src

# Copy actual source code
COPY tp-core/ tp-core/
COPY tp-cli/ tp-cli/
COPY tp-py/ tp-py/

# Build the actual application
RUN cargo build --release --features crs-transform -p tp-cli && \
    strip /build/target/release/tp-cli

# ============================================================================
# Stage 2: Runtime - Minimal image with only runtime dependencies
# ============================================================================
FROM debian:bookworm-slim

# Install only runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libsqlite3-0 \
    libtiff6 \
    libcurl4 \
    && rm -rf /var/lib/apt/lists/*

# Copy PROJ library from builder
COPY --from=builder /usr/local/lib/libproj.so* /usr/local/lib/
COPY --from=builder /usr/local/share/proj/ /usr/local/share/proj/
RUN ldconfig

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
