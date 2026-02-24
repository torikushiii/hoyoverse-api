use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use axum::body::Bytes;
use moka::future::Cache;
use mongodb::bson::doc;
use mongodb::IndexModel;

use crate::config::Config;
use crate::games::Game;
use crate::http::error::ApiError;

pub struct ResponseCache {
    store: Cache<String, Bytes>,
}

impl ResponseCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            store: Cache::builder().time_to_live(ttl).build(),
        }
    }

    pub async fn get_or_try_insert<F>(&self, key: String, init: F) -> Result<Bytes, ApiError>
    where
        F: Future<Output = Result<Bytes, ApiError>>,
    {
        self.store
            .try_get_with(key, init)
            .await
            .map_err(|e| (*e).clone())
    }

    pub async fn get_or_insert<F>(&self, key: String, init: F) -> Bytes
    where
        F: Future<Output = Bytes>,
    {
        self.store.get_with(key, init).await
    }

    pub async fn remove(&self, key: &str) {
        self.store.invalidate(key).await;
    }
}

pub struct Global {
    pub config: Config,
    #[allow(dead_code)]
    pub mongo: mongodb::Client,
    pub db: mongodb::Database,
    pub http_client: reqwest::Client,
    pub started_at: std::time::Instant,
    pub response_cache: ResponseCache,
    pub fandom_image_cache: ResponseCache,
    pub news_cache: ResponseCache,
    pub discord_webhook: Option<String>,
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
        let fandom_image_cache = ResponseCache::new(Duration::from_secs(24 * 3600));
        let news_cache = ResponseCache::new(Duration::from_secs(15 * 60));

        let discord_webhook = if config.notifications.discord_webhook.is_empty() {
            None
        } else {
            Some(config.notifications.discord_webhook.clone())
        };

        Ok(Arc::new(Self {
            config,
            mongo,
            db,
            http_client,
            started_at: std::time::Instant::now(),
            response_cache,
            fandom_image_cache,
            news_cache,
            discord_webhook,
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
