# Use the official Rust image as the build stage
FROM rust:1.84.1 AS builder

# Set the working directory inside the builder
WORKDIR /usr/src/artemis-network

# Copy dependencies and source files
COPY Cargo.toml Cargo.lock ./
COPY . .

# Build the application in release mode
RUN cargo build --release

# Use Ubuntu 22.04 for the final runtime stage (contains a newer glibc version)
FROM ubuntu:22.04

# Install runtime dependencies (e.g., OpenSSL, glibc)
RUN apt-get update && apt-get install -y --no-install-recommends \
    openssl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory in the container
WORKDIR /app

# Copy compiled binary from builder stage
COPY --from=builder /usr/src/artemis-network/target/release/artemis-network .

# Make sure the binary is executable
RUN chmod +x artemis-network

# Expose the application port (if required)
EXPOSE 8080

ENTRYPOINT ["./artemis-network"]
