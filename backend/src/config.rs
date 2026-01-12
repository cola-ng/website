use std::time::Duration;

#[derive(Clone)]
pub struct AppConfig {
    pub bind_addr: String,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_ttl: Duration,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:5800".into());
        let database_url =
            std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required".to_string())?;
        let jwt_secret =
            std::env::var("JWT_SECRET").map_err(|_| "JWT_SECRET is required".to_string())?;
        let jwt_ttl_seconds: u64 = std::env::var("JWT_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60 * 60 * 24 * 7);
        Ok(Self {
            bind_addr,
            database_url,
            jwt_secret,
            jwt_ttl: Duration::from_secs(jwt_ttl_seconds),
        })
    }
}
