mod redis;
mod mongo;

pub use redis::RedisConnection;
pub use mongo::MongoConnection;

pub struct DatabaseConnections {
    pub mongo: MongoConnection,
    pub redis: RedisConnection,
}

impl DatabaseConnections {
    pub async fn new(config: &crate::config::Settings) -> anyhow::Result<Self> {
        let mongo = MongoConnection::new(config).await?;
        let redis = RedisConnection::new(
            &config.redis.url, 
            config.redis.database,
            config.redis.rate_limit.clone(),
        ).await?;
        
        Ok(Self { mongo, redis })
    }

    pub async fn get_cached_data(&self, collection: String, key: String) -> anyhow::Result<Option<String>> {
        let mutex = self.redis.create_mutex().await?;
        
        mutex.acquire(
            format!("cache_operation:{collection}:{key}"),
            || async {
                if let Some(data) = self.redis.get_cached(&key).await? {
                    return Ok(Some(data));
                }

                if let Some(data) = self.mongo.get_document(&collection, &key).await? {
                    self.redis.set_cached(&key, &data, 300).await?;
                    return Ok(Some(data));
                }

                Ok(None)
            }
        ).await?
    }
} 