use std::net::SocketAddr;

#[derive(Debug, Clone, smart_default::SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Config {
    /// API configuration
    pub api: ApiConfig,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Validator configuration
    pub validator: ValidatorConfig,

    /// Scraper configuration
    pub scraper: ScraperConfig,

    /// Log level
    #[default(std::env::var("RUST_LOG").unwrap_or("info".into()))]
    pub level: String,
}

#[derive(Debug, Clone, smart_default::SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ApiConfig {
    /// HTTP bind address
    #[default(SocketAddr::from(([0, 0, 0, 0], 8080)))]
    pub bind: SocketAddr,

    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone, smart_default::SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct RateLimitConfig {
    /// Token replenish rate per second
    #[default(2)]
    pub per_second: u64,

    /// Maximum burst size
    #[default(120)]
    pub burst_size: u32,
}

#[derive(Debug, Clone, smart_default::SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct DatabaseConfig {
    /// MongoDB connection URI
    #[default("mongodb://localhost:27017".into())]
    pub uri: String,

    /// Database name
    #[default("hoyoverse".into())]
    pub name: String,
}

#[derive(Debug, Clone, smart_default::SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ValidatorConfig {
    /// Whether active validation is enabled
    #[default(false)]
    pub enabled: bool,

    /// Validation interval in seconds
    #[default(1800)]
    pub interval_secs: u64,

    /// Per-game validator settings (only games with a known redeem endpoint)
    #[default(Default::default())]
    pub genshin: GameValidatorConfig,
    #[default(Default::default())]
    pub starrail: GameValidatorConfig,
    #[default(Default::default())]
    pub zenless: GameValidatorConfig,
    #[default(Default::default())]
    pub themis: GameValidatorConfig,
    // Note: Honkai Impact 3rd has no redemption API — codes are scraped only
}

#[derive(Debug, Clone, smart_default::SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct GameValidatorConfig {
    /// Whether validation is enabled for this game
    #[default(false)]
    pub enabled: bool,

    /// Full HoYoLab cookie string
    #[default("".into())]
    pub cookie: String,

    /// In-game UID for redemption
    #[default("".into())]
    pub uid: String,

    /// Server region (e.g. os_usa, os_euro, os_asia, os_cht)
    #[default("os_usa".into())]
    pub region: String,
}

impl ValidatorConfig {
    /// Get the per-game config for a given game.
    pub fn game_config(&self, game: crate::games::Game) -> Option<&GameValidatorConfig> {
        match game {
            crate::games::Game::Genshin => Some(&self.genshin),
            crate::games::Game::Starrail => Some(&self.starrail),
            crate::games::Game::Zenless => Some(&self.zenless),
            crate::games::Game::Honkai => None, // no redemption API
            crate::games::Game::Themis => Some(&self.themis),
        }
    }
}

#[derive(Debug, Clone, smart_default::SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ScraperConfig {
    /// Whether scraping is enabled
    #[default(false)]
    pub enabled: bool,

    /// Scrape interval in seconds
    #[default(300)]
    pub interval_secs: u64,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = std::path::PathBuf::from("config.toml");

        let mut config = if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            println!("loaded config from {}", path.display());
            config
        } else {
            println!("no config.toml found, using defaults");
            Config::default()
        };

        // Environment variable overrides
        if let Ok(v) = std::env::var("DATABASE_URI") {
            config.database.uri = v;
        }
        if let Ok(v) = std::env::var("DATABASE_NAME") {
            config.database.name = v;
        }
        if let Ok(v) = std::env::var("BIND") {
            config.api.bind = v.parse().expect("invalid BIND address");
        }

        Ok(config)
    }
}
