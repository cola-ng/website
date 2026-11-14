FROM rust:1.99-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends libpq-dev cmake && rm -rf /var/lib/apt/lists/*
WORKDIR /app/website
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY front ./front
COPY vendor ./vendor
RUN cargo build --locked --release -p colang --bin colang

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libpq5 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/website/target/release/colang /usr/local/bin/colang
EXPOSE 8119
CMD ["colang"]
