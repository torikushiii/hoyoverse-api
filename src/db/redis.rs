use crate::mutex::DistributedMutex;
use anyhow::Result;
use fred::interfaces::ClientLike;
use fred::prelude::*;
use fred::types::{Expiration, PerformanceConfig, ReconnectPolicy, RedisConfig};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone)]
pub struct RedisConnection {
    pub(crate) client: RedisClient,
    pub(crate) config: RedisConfig,
    pub(crate) rate_limit_config: crate::config::RateLimitConfig,
}

impl RedisConnection {
    pub async fn new(
        url: &str,
        database: u8,
        rate_limit: crate::config::RateLimitConfig,
    ) -> Result<Self> {
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

    pub async fn create_mutex(&self) -> Result<DistributedMutex> {
        DistributedMutex::new(self.client.clone()).await
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
        self.client
            .set::<(), _, _>(key, value, Some(Expiration::EX(expires_in)), None, false)
            .await?;
        tracing::debug!("Set cache for key: {}", key);
        Ok(())
    }

    pub async fn log_user_agent(&self, user_agent: &str) -> anyhow::Result<()> {
        let now = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_default();

        self.client
            .hincrby::<i64, _, _>("metrics:user_agents", user_agent, 1)
            .await?;

        let timestamp = time::OffsetDateTime::now_utc().unix_timestamp() as f64;
        let entry = vec![(timestamp, format!("{}:{}", now, user_agent))];

        self.client
            .zadd::<bool, _, _>(
                "metrics:user_agents:timeline",
                None,
                None,
                false,
                false,
                entry,
            )
            .await?;

        tracing::debug!("Logged user agent: {}", user_agent);
        Ok(())
    }

    pub async fn log_endpoint_hit(&self, endpoint: &str) -> anyhow::Result<()> {
        let now = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_default();

        self.client
            .hincrby::<i64, _, _>("metrics:endpoints", endpoint, 1)
            .await?;

        let timestamp = time::OffsetDateTime::now_utc().unix_timestamp() as f64;
        let entry = vec![(timestamp, format!("{}:{}", now, endpoint))];

        self.client
            .zadd::<bool, _, _>(
                "metrics:endpoints:timeline",
                None,
                None,
                false,
                false,
                entry,
            )
            .await?;

        tracing::debug!("Logged endpoint hit: {}", endpoint);
        Ok(())
    }

    pub fn from_client(
        client: RedisClient,
        rate_limit_config: crate::config::RateLimitConfig,
    ) -> Self {
        Self {
            client,
            config: RedisConfig::default(),
            rate_limit_config,
        }
    }
}
