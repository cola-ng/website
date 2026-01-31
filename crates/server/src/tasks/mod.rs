//! Background tasks module

pub mod study_time_flush;

/// Start all background tasks
pub fn start_all() {
    study_time_flush::start();
    tracing::info!("All background tasks started");
}
