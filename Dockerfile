# Use an official Rust runtime as a parent image
FROM rust:1.83-slim AS builder

# Set working directory
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the entire project
COPY . .

# Build the application in release mode
RUN cargo build --release

# Use a lightweight runtime image
FROM debian:bookworm-slim

# Install runtime dependencies (OpenSSL)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m appuser

# Copy the built binary from builder stage
COPY --from=builder /app/target/release/rust_chat /usr/local/bin/rust_chat

# Switch to non-root user
USER appuser

# Expose the port the app runs on
EXPOSE 3030

# Set environment variables
ENV PORT=3030
ENV HOST=0.0.0.0

# ... (after your builder stage)
FROM debian:bookworm-slim
WORKDIR /app

# Install SSL for bcrypt/warp
RUN apt-get update && apt-get install -y libssl3 && rm -rf /var/lib/apt/lists/*

# Copy the binary we built
COPY --from=builder /app/target/release/rust_chat /usr/local/bin/rust_chat

# COPY THE STATIC FILES - THIS IS WHAT FIXES THE 404
COPY static /app/static

CMD ["rust_chat"]
