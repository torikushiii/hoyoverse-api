use std::sync::Arc;
use tokio_cron_scheduler::{JobScheduler, Job};
use tracing::{info, error};
use crate::db::DatabaseConnections;
use crate::resolvers::starrail::StarRailResolver;
use mongodb::{
    bson::{self, doc, Document},
    options::UpdateOptions,
};
use crate::resolvers::starrail::news::sources::hoyolab::{NewsResolver as StarRailNewsResolver, SUPPORTED_LANGUAGES as STARRAIL_LANGUAGES};

pub struct Scheduler {
    db: Arc<DatabaseConnections>,
}

impl Scheduler {
    pub fn new(db: Arc<DatabaseConnections>) -> Self {
        Self { db }
    }

    async fn schedule_starrail_tasks(&self, sched: &JobScheduler) -> Result<(), Box<dyn std::error::Error>> {
        // Schedule StarRail code scraping
        let db_clone = self.db.clone();
        sched.add(Job::new_async("0 * * * * *", move |_, _| {
            let db = db_clone.clone();
            Box::pin(async move {
                info!("[StarRail][Codes] Running scheduled code scraping");
                match StarRailResolver::fetch_codes().await {
                    Ok(new_codes) => {
                        let collection = db.mongo.collection::<Document>("starrail_codes");
                        let options = UpdateOptions::builder().upsert(true).build();

                        for code in new_codes {
                            if let Ok(mut doc) = bson::to_document(&code) {
                                doc.remove("date");
                                
                                let filter = doc! { "code": &code.code };
                                let update = doc! { 
                                    "$set": doc,
                                    "$setOnInsert": {
                                        "date": bson::DateTime::now()
                                    }
                                };
                                
                                if let Ok(update_result) = collection
                                    .update_one(filter, update)
                                    .with_options(options.clone())
                                    .await 
                                {
                                    if update_result.upserted_id.is_some() {
                                        info!("[StarRail][Codes] Inserted new code: {}", code.code);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("[StarRail][Codes] Failed to scrape codes: {}", e);
                    }
                }
            })
        })?).await?;

        // Schedule StarRail news fetching
        let db_clone = self.db.clone();
        sched.add(Job::new_async("0 */15 * * * *", move |_, _| {
            let db = db_clone.clone();
            Box::pin(async move {
                info!("[StarRail][News] Running scheduled news fetching");
                let mut total_items = 0;
                let mut new_items = 0;
                let mut failed_items = 0;

                let resolver = StarRailNewsResolver::new();
                for lang in STARRAIL_LANGUAGES {
                    match resolver.fetch_news(lang).await {
                        Ok(news_items) => {
                            total_items += news_items.len();
                            let collection = db.mongo.collection::<Document>("starrail_news");
                            let options = UpdateOptions::builder().upsert(true).build();

                            for news_item in news_items {
                                match bson::to_document(&news_item) {
                                    Ok(doc) => {
                                        let filter = doc! { 
                                            "external_id": &news_item.external_id,
                                            "lang": &news_item.lang 
                                        };
                                        let update = doc! { "$set": &doc };
                                        
                                        match collection
                                            .update_one(filter, update)
                                            .with_options(options.clone())
                                            .await 
                                        {
                                            Ok(update_result) => {
                                                if update_result.upserted_id.is_some() {
                                                    new_items += 1;
                                                    info!(
                                                        "[StarRail][News] Inserted new item: {} ({}) [{}]", 
                                                        news_item.title, 
                                                        news_item.news_type,
                                                        news_item.lang
                                                    );
                                                }
                                            }
                                            Err(e) => {
                                                failed_items += 1;
                                                error!(
                                                    "[StarRail][News] Failed to update item {}: {}", 
                                                    news_item.external_id, 
                                                    e
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        failed_items += 1;
                                        error!(
                                            "[StarRail][News] Failed to serialize item {}: {}", 
                                            news_item.external_id, 
                                            e
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("[StarRail][News] Failed to fetch for language {}: {}", lang, e);
                        }
                    }
                }

                info!(
                    "[StarRail][News] Fetch complete - Total: {}, New: {}, Failed: {}", 
                    total_items, 
                    new_items, 
                    failed_items
                );
            })
        })?).await?;

        Ok(())
    }

    // TODO: Add similar methods for other games
    // async fn schedule_genshin_tasks(&self, sched: &JobScheduler) -> Result<(), Box<dyn std::error::Error>>
    // async fn schedule_zenless_tasks(&self, sched: &JobScheduler) -> Result<(), Box<dyn std::error::Error>>

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let sched = JobScheduler::new().await?;
        
        info!("Initializing game schedulers...");
        
        // Schedule StarRail tasks
        self.schedule_starrail_tasks(&sched).await?;
        
        // TODO: Schedule other game tasks
        // self.schedule_genshin_tasks(&sched).await?;
        // self.schedule_zenless_tasks(&sched).await?;

        info!("Starting scheduler");
        sched.start().await?;
        info!("Scheduler started successfully");

        Ok(())
    }
} 