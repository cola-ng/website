FROM rust:1.92 AS chef
RUN cargo install cargo-chef
WORKDIR /usr/src/app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /usr/src/app/recipe.json recipe.json
# Build dependencies only — this layer is cached unless Cargo.toml/Cargo.lock change
RUN cargo chef cook --release --recipe-path recipe.json
# Now copy the real source and build — only project code recompiles
COPY . .
RUN cargo build --release

FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/colang /usr/local/bin/

EXPOSE 8119

CMD ["colang"]
