use mongodb::{Client as MongoClient, Database, options::ClientOptions};
use bson::Document;

pub struct MongoConnection {
    db: Database,
}

impl MongoConnection {
    pub async fn new(config: &crate::config::Settings) -> anyhow::Result<Self> {
        let mut options = ClientOptions::parse(&config.mongodb.url).await?;
        options.max_pool_size = Some(config.redis.pool_size as u32);

        let client = MongoClient::with_options(options)?;
        let db = client.database(&config.mongodb.database);

        tracing::info!("Connected to MongoDB database {}", config.mongodb.database);

        Ok(Self { db })
    }

    pub fn collection<T>(&self, name: &str) -> mongodb::Collection<T>
    where
        T: Send + Sync,
    {
        self.db.collection(name)
    }

    pub async fn get_document(&self, collection: &str, key: &str) -> anyhow::Result<Option<String>> {
        let collection = self.db.collection::<Document>(collection);

        if let Some(doc) = collection.find_one(bson::doc! { "_id": key }).await? {
            if let Ok(data) = bson::to_document(&doc) {
                if let Ok(json) = serde_json::to_string(&data) {
                    tracing::debug!("Retrieved document from MongoDB for key: {}", key);
                    return Ok(Some(json));
                }
            }
        }

        Ok(None)
    }
}