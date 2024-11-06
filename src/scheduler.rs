use std::sync::Arc;
use tokio_cron_scheduler::{JobScheduler, Job};
use tracing::{info, error};
use crate::db::DatabaseConnections;
use crate::resolvers::starrail::StarRailResolver;
use mongodb::{
    bson::{self, doc, Document},
    options::UpdateOptions,
};

pub struct Scheduler {
    db: Arc<DatabaseConnections>,
}

impl Scheduler {
    pub fn new(db: Arc<DatabaseConnections>) -> Self {
        Self { db }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let sched = JobScheduler::new().await?;
        
        let db_clone = self.db.clone();
        sched.add(Job::new_async("0 * * * * *", move |_, _| {
            let db = db_clone.clone();
            Box::pin(async move {
                info!("Running scheduled code scraping");
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
                                
                                let result = collection
                                    .update_one(filter, update)
                                    .with_options(options.clone())
                                    .await;

                                if let Ok(update_result) = result {
                                    if update_result.upserted_id.is_some() {
                                        info!("Inserted new code: {}", code.code);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to scrape codes: {}", e);
                    }
                }
            })
        })?).await?;

        let db_clone = self.db.clone();
        sched.add(Job::new_async("0 */15 * * * *", move |_, _| {
            let db = db_clone.clone();
            Box::pin(async move {
                info!("Running scheduled news fetching");
                let categories = ["notices", "events", "info"];
                let mut new_items = 0;

                for category in &categories {
                    match StarRailResolver::fetch_news(category).await {
                        Ok(new_news) => {
                            let collection = db.mongo.collection::<Document>("starrail_news");
                            let options = UpdateOptions::builder().upsert(true).build();

                            for news_item in new_news {
                                if let Ok(doc) = bson::to_document(&news_item) {
                                    let filter = doc! { "id": &news_item.id };
                                    let update = doc! { "$set": &doc };
                                    
                                    if let Ok(update_result) = collection
                                        .update_one(filter, update)
                                        .with_options(options.clone())
                                        .await
                                    {
                                        if update_result.upserted_id.is_some() {
                                            new_items += 1;
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to fetch {} news: {}", category, e);
                        }
                    }
                }

                if new_items > 0 {
                    info!("Inserted {} new news items", new_items);
                }
            })
        })?).await?;

        info!("Starting scheduler");
        sched.start().await?;
        info!("Scheduler started successfully");

        Ok(())
    }
} 