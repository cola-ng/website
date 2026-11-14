use colang::{AppConfig, db, routing, tasks};
use salvo::oapi::OpenApi;
use salvo::prelude::*;

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

    db::init(&app_config.database).await;

    // Start background tasks
    let (stop_tasks, tasks_done) = tasks::start_all();

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
    let server = Server::new(acceptor);
    let handle = server.handle();
    tokio::spawn(async move {
        shutdown_signal().await;
        handle.stop_graceful(Some(std::time::Duration::from_secs(15)));
    });
    server.serve(router).await;
    let _ = stop_tasks.send(());
    let _ = tasks_done.await;
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
