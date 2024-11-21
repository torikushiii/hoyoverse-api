use tracing::{debug, error};
use anyhow::Context;
use uuid::Uuid;

const LUA_SCRIPT: &str = include_str!("mutex.lua");

#[derive(Debug, thiserror::Error)]
pub enum MutexError {
    #[error("failed to acquire mutex after {0} attempts")]
    AcquireFailed(usize),
    #[error("mutex lock was lost")]
    LockLost,
    #[error("redis error: {0}")]
    Redis(#[from] fred::error::RedisError),
}

pub struct DistributedMutex {
    redis: fred::prelude::RedisClient,
    lock_fn: fred::types::Function,
    unlock_fn: fred::types::Function,
}

impl DistributedMutex {
    pub async fn new(redis: fred::prelude::RedisClient) -> anyhow::Result<Self> {
        let lib = fred::types::Library::from_code(&redis, LUA_SCRIPT)
            .await
            .context("failed to load mutex Lua script")?;

        let lock_fn = lib.functions()
            .get("mutex_lock")
            .context("failed to get mutex_lock function")?
            .clone();

        let unlock_fn = lib.functions()
            .get("mutex_unlock")
            .context("failed to get mutex_unlock function")?
            .clone();

        Ok(Self {
            redis,
            lock_fn,
            unlock_fn,
        })
    }

    pub async fn acquire<T, F, Fut>(&self, key: String, f: F) -> Result<T, MutexError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let lock_id = Uuid::new_v4().to_string();
        let mut acquired = false;
        let max_attempts = 10;

        // Try to acquire the lock
        for attempt in 0..max_attempts {
            let args: Vec<String> = vec![lock_id.clone(), "30".to_string()];
            match self.lock_fn
                .fcall::<i64, _, _, Vec<String>>(&self.redis, &[&key], args)
                .await
            {
                Ok(1) => {
                    acquired = true;
                    debug!("Acquired mutex lock for key '{}' on attempt {}", key, attempt + 1);
                    break;
                }
                Ok(0) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                Ok(_) => {
                    error!("Unexpected return value from mutex lock");
                    return Err(MutexError::Redis(fred::error::RedisError::new(
                        fred::error::RedisErrorKind::Parse,
                        "Unexpected return value from mutex lock",
                    )));
                }
                Err(e) => {
                    error!("Redis error while acquiring mutex: {}", e);
                    return Err(MutexError::Redis(e));
                }
            }
        }

        if !acquired {
            return Err(MutexError::AcquireFailed(max_attempts));
        }

        let result = f().await;

        // Release the lock
        let args: Vec<String> = vec![lock_id];
        match self.unlock_fn
            .fcall::<i64, _, _, Vec<String>>(&self.redis, &[&key], args)
            .await
        {
            Ok(1) => {
                debug!("Released mutex lock for key '{}'", key);
            }
            Ok(0) => {
                error!("Failed to release mutex lock for key '{}' - lock was lost", key);
                return Err(MutexError::LockLost);
            }
            Ok(_) => {
                error!("Unexpected return value from mutex unlock");
                return Err(MutexError::Redis(fred::error::RedisError::new(
                    fred::error::RedisErrorKind::Parse,
                    "Unexpected return value from mutex unlock",
                )));
            }
            Err(e) => {
                error!("Redis error while releasing mutex: {}", e);
                return Err(MutexError::Redis(e));
            }
        }

        Ok(result)
    }
}