
use std::sync::{ OnceLock};
use std::time::Duration;

#[derive(Clone)]
pub struct AppConfig {
    pub bind_addr: String,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_ttl: Duration,
}

#[derive(Clone)]
pub struct DbConfig {
    pub url: String,
    pub pool_size: u32,
    pub min_idle: u32,
    pub connection_timeout: u64,
    pub helper_threads: usize,
    pub statement_timeout: u64,
    pub tcp_timeout: u64,
    pub enforce_tls: bool,
}

pub static CONFIG: OnceLock<AppConfig> = OnceLock::new();
impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:6019".into());
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
    pub fn get() -> &'static AppConfig {
        CONFIG.get().unwrap()
    }
}

// pub fn init(config_path: impl AsRef<Path>) {
//     let config_path = config_path.as_ref();
//     if !config_path.exists() {
//         panic!("config file not found: `{}`", config_path.display());
//     }

//     let raw_conf = figment_from_path(config_path).merge(Env::prefixed("COLANG_").global());
//     let conf = match raw_conf.extract::<AppConfig>() {
//         Ok(s) => s,
//         Err(e) => {
//             eprintln!("it looks like your config is invalid. The following error occurred: {e}");
//             std::process::exit(1);
//         }
//     };

//     CONFIG.set(conf).expect("config should be set once");
// }
