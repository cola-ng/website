FROM rust:1.99-bookworm AS builder
WORKDIR /app
RUN rustup target add wasm32-unknown-unknown \
    && cargo install trunk --version 0.22.1 --locked
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY front ./front
WORKDIR /app/front
RUN trunk build --release --locked

FROM nginx:alpine
ENV API_UPSTREAM=colang:8119 \
    NGINX_ENVSUBST_OUTPUT_DIR=/etc/nginx \
    NGINX_ENVSUBST_FILTER=^API_UPSTREAM$
COPY --from=builder /app/front/dist /usr/share/nginx/html
COPY front/nginx.conf /etc/nginx/templates/nginx.conf.template
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
