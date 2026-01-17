use std::sync::OnceLock;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub bind_addr: String,
    pub jwt_secret: String,
    pub jwt_ttl: Duration,
    pub database: DbConfig,
}

#[derive(Clone, Debug)]
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

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            pool_size: 15,
            min_idle: 5,
            connection_timeout: 30000,
            helper_threads: 4,
            statement_timeout: 5000,
            tcp_timeout: 30000,
            enforce_tls: false,
        }
    }
}

pub static APP_CONFIG: OnceLock<AppConfig> = OnceLock::new();
impl AppConfig {
    pub fn init() {
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8119".into());
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
        let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET is required");
        let jwt_ttl_seconds: u64 = std::env::var("JWT_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60 * 60 * 24 * 7);
        APP_CONFIG
            .set(Self {
                bind_addr,
                database: DbConfig {
                    url: database_url.clone(),
                    pool_size: 15,
                    min_idle: 5,
                    connection_timeout: 30000,
                    helper_threads: 4,
                    statement_timeout: 5000,
                    tcp_timeout: 30000,
                    enforce_tls: false,
                },
                jwt_secret,
                jwt_ttl: Duration::from_secs(jwt_ttl_seconds),
            })
            .expect("config should be set once");
    }
    pub fn get() -> &'static AppConfig {
        APP_CONFIG.get().unwrap()
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
