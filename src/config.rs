use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use anyhow::Context as _;

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

    /// Notifications configuration
    pub notifications: NotificationsConfig,

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

    /// Response cache TTL in seconds
    #[default(300)]
    pub cache_ttl_secs: u64,

    pub trusted_proxies: Vec<ProxyCidr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProxyCidr {
    addr: IpAddr,
    prefix: u8,
}

impl ProxyCidr {
    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.addr, ip.to_canonical()) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => {
                let mask = u32::MAX
                    .checked_shl(32 - u32::from(self.prefix))
                    .unwrap_or(0);
                u32::from(net) & mask == u32::from(ip) & mask
            }
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                let mask = u128::MAX
                    .checked_shl(128 - u32::from(self.prefix))
                    .unwrap_or(0);
                u128::from(net) & mask == u128::from(ip) & mask
            }
            _ => false,
        }
    }
}

impl FromStr for ProxyCidr {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let (addr, prefix) = match s.split_once('/') {
            Some((addr, prefix)) => (addr, Some(prefix)),
            None => (s, None),
        };
        let addr: IpAddr = addr
            .trim()
            .parse::<IpAddr>()
            .with_context(|| format!("invalid proxy address {s:?}"))?
            .to_canonical();
        let max = if addr.is_ipv4() { 32 } else { 128 };
        let prefix = match prefix {
            Some(p) => p
                .trim()
                .parse::<u8>()
                .ok()
                .filter(|&p| p <= max)
                .with_context(|| format!("invalid prefix length in {s:?}"))?,
            None => max,
        };
        Ok(Self { addr, prefix })
    }
}

impl TryFrom<String> for ProxyCidr {
    type Error = anyhow::Error;

    fn try_from(s: String) -> anyhow::Result<Self> {
        s.parse()
    }
}

impl fmt::Display for ProxyCidr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.addr, self.prefix)
    }
}

impl From<ProxyCidr> for String {
    fn from(cidr: ProxyCidr) -> Self {
        cidr.to_string()
    }
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

#[derive(Debug, Clone, smart_default::SmartDefault, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct NotificationsConfig {
    /// Discord webhook URL for new code notifications (empty = disabled)
    #[default("".into())]
    pub discord_webhook: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn matches_ipv4_range() {
        let cidr: ProxyCidr = "173.245.48.0/20".parse().unwrap();

        assert!(cidr.contains(ip("173.245.48.1")));
        assert!(cidr.contains(ip("173.245.63.255")));
        assert!(!cidr.contains(ip("173.245.64.0")));
        assert!(cidr.contains(ip("::ffff:173.245.48.1")));
    }

    #[test]
    fn matches_ipv6_range() {
        let cidr: ProxyCidr = "2400:cb00::/32".parse().unwrap();

        assert!(cidr.contains(ip("2400:cb00:1::1")));
        assert!(!cidr.contains(ip("2400:cb01::1")));
        assert!(!cidr.contains(ip("173.245.48.1")));
    }

    #[test]
    fn bare_address_and_zero_prefix() {
        let single: ProxyCidr = "127.0.0.1".parse().unwrap();
        let any: ProxyCidr = "0.0.0.0/0".parse().unwrap();

        assert!(single.contains(ip("127.0.0.1")));
        assert!(!single.contains(ip("127.0.0.2")));
        assert!(any.contains(ip("8.8.8.8")));
    }

    #[test]
    fn rejects_invalid_entries() {
        assert!("10.0.0.0/33".parse::<ProxyCidr>().is_err());
        assert!("not-an-ip".parse::<ProxyCidr>().is_err());
        assert!("::/129".parse::<ProxyCidr>().is_err());
    }

    #[test]
    fn deserializes_from_toml() {
        let config: ApiConfig =
            toml::from_str(r#"trusted_proxies = ["10.0.0.0/8", "::1"]"#).unwrap();

        assert_eq!(config.trusted_proxies.len(), 2);
        assert!(config.trusted_proxies[1].contains(ip("::1")));
    }
}
