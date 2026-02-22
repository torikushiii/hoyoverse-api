use anyhow::{Context, Result};
use axum::http::HeaderMap;
use fred::clients::RedisClient;
use fred::prelude::*;
use fred::types::{Function, Library, PerformanceConfig, ReconnectPolicy};
use std::net::IpAddr;
use std::sync::Arc;
use tracing::{debug, error};

#[derive(Debug, Clone)]
pub struct RateLimitResponse {
    pub remaining: i64,
    pub reset: i64,
    pub limit: i64,
    pub used: i64,
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
        let redis_key = key.clone();
        debug!("Checking rate limit for key: {}", redis_key);

        let result: Vec<i64> = self
            .ratelimit
            .fcall(
                &self.redis_client,
                vec![redis_key],
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

        debug!(
            "Rate limit result for {}: remaining={}, reset={}",
            key, result[0], result[1]
        );

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
        debug!(
            "Checking rate limit for IP {} on resource {}",
            normalized_ip, resource
        );

        self.check_rate_limit(
            redis_key,
            config.max_requests,
            config.window_seconds,
            config.punishment_threshold,
            config.punishment_duration,
        )
        .await
    }

    fn get_real_ip(headers: &HeaderMap) -> Option<IpAddr> {
        if let Some(ip) = headers
            .get("CF-Connecting-IP")
            .and_then(|h| h.to_str().ok())
            .and_then(|ip| ip.parse::<IpAddr>().ok())
        {
            return Some(ip.to_canonical());
        }

        if let Some(ip) = headers
            .get("X-Real-IP")
            .and_then(|h| h.to_str().ok())
            .and_then(|ip| ip.parse::<IpAddr>().ok())
        {
            return Some(ip.to_canonical());
        }

        if let Some(forwarded) = headers.get("X-Forwarded-For").and_then(|h| h.to_str().ok()) {
            if let Some(ip) = forwarded
                .split(',')
                .next()
                .and_then(|ip| ip.trim().parse::<IpAddr>().ok())
            {
                return Some(ip.to_canonical());
            }
        }

        None
    }

    fn get_user_agent(headers: &HeaderMap) -> Option<String> {
        headers
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    }

    pub async fn check_rate_limit_with_headers(
        &self,
        resource: &str,
        headers: &HeaderMap,
        config: &crate::config::RateLimitConfig,
    ) -> Result<RateLimitResponse> {
        let ip = Self::get_real_ip(headers).unwrap_or_else(|| "0.0.0.0".parse().unwrap());

        if let Some(user_agent) = Self::get_user_agent(headers) {
            let redis_conn =
                crate::db::RedisConnection::from_client(self.redis_client.clone(), config.clone());

            if let Err(e) = redis_conn.log_user_agent(&user_agent).await {
                error!("Failed to log user agent: {}", e);
            }
        }

        debug!(
            "Rate limit check for IP {} (from headers) on resource {}",
            ip, resource
        );

        self.check_rate_limit_with_ip(resource, ip, config).await
    }
}
