use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context as _;
use axum::body::Bytes;
use mongodb::bson::doc;
use mongodb::IndexModel;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::games::Game;

pub struct ResponseCache {
    store: Arc<RwLock<HashMap<String, (Bytes, Instant)>>>,
    ttl: Duration,
}

impl ResponseCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    pub async fn get(&self, key: &str) -> Option<Bytes> {
        let store = self.store.read().await;
        store.get(key).and_then(|(bytes, cached_at)| {
            if cached_at.elapsed() < self.ttl {
                Some(bytes.clone())
            } else {
                None
            }
        })
    }

    pub async fn insert(&self, key: String, bytes: Bytes) {
        self.store
            .write()
            .await
            .insert(key, (bytes, Instant::now()));
    }

    pub async fn remove(&self, key: &str) {
        self.store.write().await.remove(key);
    }
}

pub struct Global {
    pub config: Config,
    pub mongo: mongodb::Client,
    pub db: mongodb::Database,
    pub http_client: reqwest::Client,
    pub started_at: std::time::Instant,
    pub response_cache: ResponseCache,
}

impl Global {
    pub async fn init(config: Config) -> anyhow::Result<Arc<Self>> {
        let mongo = mongodb::Client::with_uri_str(&config.database.uri)
            .await
            .context("mongodb connect")?;

        tracing::info!("connected to mongodb");

        let db = mongo.database(&config.database.name);

        Self::ensure_indexes(&db).await?;

        let http_client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .context("http client")?;

        let response_cache = ResponseCache::new(Duration::from_secs(config.api.cache_ttl_secs));

        Ok(Arc::new(Self {
            config,
            mongo,
            db,
            http_client,
            started_at: std::time::Instant::now(),
            response_cache,
        }))
    }

    async fn ensure_indexes(db: &mongodb::Database) -> anyhow::Result<()> {
        let games = [
            Game::Genshin,
            Game::Starrail,
            Game::Zenless,
            Game::Honkai,
            Game::Themis,
        ];

        for game in games {
            let collection = db.collection::<mongodb::bson::Document>(game.collection_name());
            collection
                .create_index(
                    IndexModel::builder()
                        .keys(doc! { "code": 1 })
                        .options(
                            mongodb::options::IndexOptions::builder()
                                .unique(true)
                                .build(),
                        )
                        .build(),
                )
                .await
                .with_context(|| format!("creating unique index on {}", game.collection_name()))?;
        }

        tracing::info!("ensured unique indexes on code collections");
        Ok(())
    }
}
