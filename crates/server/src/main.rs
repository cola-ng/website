#[macro_use]
extern crate tracing;

pub mod auth;
mod config;
mod db;
mod error;
mod global;
mod hoops;
mod models;
mod routing;
mod services;
pub use global::*;
pub mod user;
pub use error::AppError;
use salvo::oapi::OpenApi;
use salvo::prelude::*;

use crate::config::AppConfig;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // Initialize tracing subscriber with env filter
    // Default to info level, can be overridden with RUST_LOG env var
    // Example: RUST_LOG=debug or RUST_LOG=colang=debug,tower_http=debug
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("Starting server...");

    AppConfig::init();
    let app_config = AppConfig::get();
    let bind_addr = app_config.bind_addr.clone();

    db::init(&app_config.database);

    salvo::http::request::set_global_secure_max_size(100_000_000);
    let router = routing::router();
    let doc = OpenApi::new("Cola API", env!("CARGO_PKG_VERSION")).merge_router(&router);

    let router = router
        .unshift(doc.into_router("/api-doc/openapi.json"))
        .unshift(Router::with_path("/api-doc/swagger-ui/{**}").get(
            salvo::oapi::swagger_ui::SwaggerUi::new("/api-doc/openapi.json"),
        ))
        .unshift(
            Router::with_path("/api-doc/scalar/{**}")
                .get(salvo::oapi::scalar::Scalar::new("/api-doc/openapi.json")),
        );

    println!("bind_addr: {:?}", bind_addr);
    println!(
        "OpenAPI docs available at: http://{}/api-doc/swagger-ui/",
        bind_addr
    );
    println!(
        "Scalar docs available at: http://{}/api-doc/scalar/",
        bind_addr
    );

    let acceptor = TcpListener::new(bind_addr).bind().await;
    println!("acceptor: {:?}", acceptor);
    Server::new(acceptor).serve(router).await;
}
