use anyhow::Result;
use fred::prelude::*;
use fred::types::{RedisConfig, PerformanceConfig, ReconnectPolicy, Expiration};
use fred::interfaces::ClientLike;

#[derive(Clone)]
pub struct RedisConnection {
    client: RedisClient,
    config: RedisConfig,
    rate_limit_config: crate::config::RateLimitConfig,
}

impl RedisConnection {
    pub async fn new(url: &str, database: u8, rate_limit: crate::config::RateLimitConfig) -> Result<Self> {
        let mut config = RedisConfig::from_url(url)?;
        
        config.database = Some(database);
        tracing::debug!("Using Redis database: {}", database);

        let client = RedisClient::new(
            config.clone(),
            Some(PerformanceConfig::default()),
            Some(ConnectionConfig::default()),
            Some(ReconnectPolicy::default()),
        );

        client.connect();
        client.wait_for_connect().await?;
        
        client.select(database).await?;
        tracing::debug!("Selected Redis database: {}", database);
        
        Ok(Self { 
            client,
            config,
            rate_limit_config: rate_limit,
        })
    }

    pub fn get_config(&self) -> &RedisConfig {
        &self.config
    }

    pub fn get_rate_limit_config(&self) -> &crate::config::RateLimitConfig {
        &self.rate_limit_config
    }

    pub async fn get_cached(&self, key: &str) -> anyhow::Result<Option<String>> {
        let result = self.client.get::<Option<String>, _>(key).await?;
        if result.is_some() {
            tracing::debug!("Cache hit for key: {}", key);
        }
        Ok(result)
    }

    pub async fn set_cached(&self, key: &str, value: &str, expires_in: i64) -> anyhow::Result<()> {
        self.client.set::<(), _, _>(
            key,
            value,
            Some(Expiration::EX(expires_in)),
            None,
            false
        ).await?;
        tracing::debug!("Set cache for key: {}", key);
        Ok(())
    }
} 