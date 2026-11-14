//! Background tasks module

pub mod study_time_flush;

/// Start all background tasks
pub fn start_all() -> (
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let task = study_time_flush::start();
    tracing::info!("All background tasks started");
    task
}
