use std::sync::Arc;
use anyhow::{Result, Context};
use tracing::{error, debug};
use fred::prelude::*;
use fred::types::{Function, Library, PerformanceConfig, ReconnectPolicy};
use fred::clients::RedisClient;
use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct RateLimitResponse {
    pub remaining: i64,
    #[allow(dead_code)]
    pub reset: i64,
    #[allow(dead_code)]
    pub limit: i64,
    #[allow(dead_code)]
    pub used: i64,
    #[allow(dead_code)]
    pub resource: String,
}

impl RateLimitResponse {
    pub fn is_allowed(&self) -> bool {
        self.remaining >= 0
    }
}

pub struct RateLimiter {
    redis_client: RedisClient,
    ratelimit: Function,
}

impl RateLimiter {
    pub async fn new(redis: Arc<crate::db::RedisConnection>) -> Result<Self> {
        debug!("Initializing rate limiter");

        let redis_client = RedisClient::new(
            redis.get_config().clone(),
            Some(PerformanceConfig::default()),
            Some(ConnectionConfig::default()),
            Some(ReconnectPolicy::default()),
        );
        redis_client.connect();
        redis_client.wait_for_connect().await?;

        let script_content = include_str!("limit.lua");

        let lib = Library::from_code(&redis_client, script_content)
            .await
            .context("Failed to create Lua library")?;

        let ratelimit = lib
            .functions()
            .get("api_ratelimit")
            .context("Failed to get api_ratelimit function")?
            .clone();

        debug!("Rate limiter initialized successfully");

        Ok(Self {
            redis_client,
            ratelimit,
        })
    }

    pub async fn check_rate_limit(
        &self,
        key: String,
        max_requests: i64,
        window_seconds: i64,
        punishment_threshold: i64,
        punishment_duration: i64,
    ) -> Result<RateLimitResponse> {
        let redis_key = format!("ratelimit:{}", key);
        debug!("Checking rate limit for key: {}", redis_key);

        let result: Vec<i64> = self.ratelimit
            .fcall(
                &self.redis_client,
                vec![redis_key.clone()],
                vec![
                    max_requests,
                    1, // ticket_count
                    window_seconds,
                    punishment_threshold,
                    punishment_duration,
                ],
            )
            .await
            .map_err(|e| {
                error!("Rate limit check failed: {}", e);
                anyhow::anyhow!("Failed to check rate limit: {}", e)
            })?;

        debug!("Rate limit result for {}: remaining={}, reset={}",
            key, result[0], result[1]);

        Ok(RateLimitResponse {
            remaining: result[0],
            reset: result[1],
            limit: max_requests,
            used: 1,
            resource: key,
        })
    }

    pub async fn check_rate_limit_with_ip(
        &self,
        resource: &str,
        ip: IpAddr,
        config: &crate::config::RateLimitConfig,
    ) -> Result<RateLimitResponse> {
        let normalized_ip = match ip {
            IpAddr::V6(ipv6) => {
                if let Some(ipv4) = ipv6.to_ipv4_mapped() {
                    IpAddr::V4(ipv4)
                } else {
                    IpAddr::V6(ipv6)
                }
            }
            IpAddr::V4(_) => ip,
        };

        let redis_key = format!("ratelimit:{}:{}", normalized_ip, resource);
        debug!("Checking rate limit for IP {} on resource {}", normalized_ip, resource);

        self.check_rate_limit(
            redis_key,
            config.max_requests,
            config.window_seconds,
            config.punishment_threshold,
            config.punishment_duration,
        )
        .await
    }
}
