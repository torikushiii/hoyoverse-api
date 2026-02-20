use std::sync::Arc;

use anyhow::Context as _;
use mongodb::bson::doc;
use mongodb::IndexModel;

use crate::config::Config;
use crate::games::Game;

pub struct Global {
    pub config: Config,
    pub mongo: mongodb::Client,
    pub db: mongodb::Database,
    pub http_client: reqwest::Client,
    pub started_at: std::time::Instant,
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

        Ok(Arc::new(Self {
            config,
            mongo,
            db,
            http_client,
            started_at: std::time::Instant::now(),
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
