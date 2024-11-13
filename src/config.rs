use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
    pub user_agent: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    pub max_requests: i64,
    pub window_seconds: i64,
    pub punishment_threshold: i64,
    pub punishment_duration: i64,
}

#[derive(Debug, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub database: u8,
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Deserialize)]
pub struct MongoConfig {
    pub url: String,
    pub database: String,
}

#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GameAccount {
    pub cookie_token_v2: String,
    pub account_mid_v2: String,
    pub account_id_v2: String,
    pub uid: String,
    pub nickname: String,
    pub region: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GameAccounts {
    pub starrail: Vec<GameAccount>,
    pub genshin: Vec<GameAccount>,
    pub zenless: Vec<GameAccount>,
    pub themis: Vec<GameAccount>,
}

#[derive(Debug, Deserialize)]
pub struct DiscordConfig {
    pub webhook_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub redis: RedisConfig,
    pub mongodb: MongoConfig,
    pub logging: LoggingConfig,
    pub game_accounts: GameAccounts,
    pub discord: DiscordConfig,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let environment = env::var("RUN_ENV").unwrap_or_else(|_| "development".into());

        let config_file = match environment.as_str() {
            "production" => "prod",
            "development" | _ => "dev",
        };

        let s = Config::builder()
            .add_source(File::with_name("config/default.yaml").required(false))
            .add_source(File::with_name(&format!("config/{}.yaml", config_file)).required(false))
            .add_source(File::with_name("config/local.yaml").required(false))
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?;

        s.try_deserialize()
    }
}