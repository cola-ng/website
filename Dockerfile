# Multi-stage Dockerfile for Cola-ng Website
#
# Build:
#   docker build -t colang .
#
# Run:
#   docker run -p 8119:8119 colang

# Stage 1: Build frontend
FROM node:22-alpine AS frontend-builder

WORKDIR /app/front

# Install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

# Copy frontend package files
COPY front/package.json front/pnpm-lock.yaml* front/package-lock.json* ./

# Install dependencies
RUN if [ -f pnpm-lock.yaml ]; then pnpm install --frozen-lockfile; \
    elif [ -f package-lock.json ]; then npm ci; \
    else npm install; fi

# Copy frontend source
COPY front/ ./

# Build frontend
RUN npm run build


# Stage 2: Build Rust backend
FROM rust:1.92 AS backend-builder

WORKDIR /usr/src/app

# Copy Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build the release binary
RUN cargo build --release -p colang


# Stage 3: Final runtime image
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /colang

# Copy backend binary
COPY --from=backend-builder /usr/src/app/target/release/colang ./

# Copy frontend static files
COPY --from=frontend-builder /app/front/dist ./static

EXPOSE 8119

CMD ["./colang"]
