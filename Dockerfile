# Combined API + static frontend image; compose uses the separate images.
FROM rust:1.99-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends libpq-dev cmake && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown && cargo install trunk --version 0.22.1 --locked
WORKDIR /app/website
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY front ./front
COPY vendor ./vendor
RUN cargo build --release --locked -p colang --bin colang
WORKDIR /app/website/front
RUN trunk build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libpq5 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/website/target/release/colang /usr/local/bin/colang
COPY --from=builder /app/website/front/dist /app/static
EXPOSE 8119
CMD ["colang"]
