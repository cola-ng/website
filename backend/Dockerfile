FROM rust:1.84 AS builder

WORKDIR /usr/src/app
COPY ./backend .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/colang_backend /usr/local/bin/

EXPOSE 5800

CMD ["colang_backend"]