use std::sync::Arc;
use tokio_cron_scheduler::{JobScheduler, Job};
use tracing::{info, error};
use mongodb::{
    bson::{self, doc, Document},
    options::UpdateOptions,
};
use crate::{
    db::DatabaseConnections,
    resolvers::honkai::{HonkaiResolver, news::sources::hoyolab::{
        NewsResolver as HonkaiNewsResolver,
        SUPPORTED_LANGUAGES as HONKAI_LANGUAGES
    }},
    config::Settings,
};

async fn schedule_codes(sched: &JobScheduler, db: Arc<DatabaseConnections>, config: Arc<Settings>) -> Result<(), Box<dyn std::error::Error>> {
    sched.add(Job::new_async("0 */5 * * * *", move |_, _| {
        let db = db.clone();
        let config = config.clone();
        Box::pin(async move {
            info!("[Honkai][Codes] Running scheduled code scraping");
            match HonkaiResolver::fetch_codes(&config).await {
                Ok(new_codes) => {
                    let collection = db.mongo.collection::<Document>("honkai_codes");
                    let options = UpdateOptions::builder().upsert(true).build();

                    for code in new_codes {
                        if let Ok(mut doc) = bson::to_document(&code) {
                            doc.remove("date");
                            let filter = doc! { "code": &code.code };
                            let update = doc! {
                                "$set": {
                                    "code": &code.code,
                                    "active": code.active,
                                    "rewards": &code.rewards,
                                    "source": &code.source
                                },
                                "$setOnInsert": {
                                    "date": bson::DateTime::now()
                                }
                            };

                            match collection.update_one(filter, update)
                                .with_options(options.clone())
                                .await
                            {
                                Ok(result) => {
                                    if result.upserted_id.is_some() {
                                        info!(
                                            "[Honkai][Codes] Inserted new code: {} (active: {})",
                                            code.code,
                                            code.active
                                        );
                                    } else if result.modified_count > 0 {
                                        info!(
                                            "[Honkai][Codes] Updated code status: {} (active: {})",
                                            code.code,
                                            code.active
                                        );
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "[Honkai][Codes] Failed to update code {}: {}",
                                        code.code,
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("[Honkai][Codes] Failed to scrape codes: {}", e);
                }
            }
        })
    })?).await?;

    Ok(())
}

async fn schedule_news(sched: &JobScheduler, db: Arc<DatabaseConnections>, config: Arc<Settings>) -> Result<(), Box<dyn std::error::Error>> {
    sched.add(Job::new_async("0 */20 * * * *", move |_, _| {
        let db = db.clone();
        let config = config.clone();
        Box::pin(async move {
            let mutex = match db.redis.create_mutex().await {
                Ok(mutex) => mutex,
                Err(e) => {
                    error!("[Honkai][News] Failed to create mutex: {}", e);
                    return;
                }
            };

            if let Err(e) = mutex.acquire(
                "honkai_news_fetch".to_string(),
                || async {
                    info!("[Honkai][News] Running scheduled news fetching");
                    let mut total_items = 0;
                    let mut new_items = 0;
                    let mut failed_items = 0;

                    let resolver = HonkaiNewsResolver::new(&config);
                    for lang in HONKAI_LANGUAGES {
                        match resolver.fetch_news(lang).await {
                            Ok(news_items) => {
                                total_items += news_items.len();
                                let collection = db.mongo.collection::<Document>("honkai_news");
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
                                                            "[Honkai][News] Inserted new item: {} ({}) [{}]",
                                                            news_item.title,
                                                            news_item.news_type,
                                                            news_item.lang
                                                        );
                                                    }
                                                }
                                                Err(e) => {
                                                    failed_items += 1;
                                                    error!(
                                                        "[Honkai][News] Failed to update item {}: {}",
                                                        news_item.external_id,
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            failed_items += 1;
                                            error!(
                                                "[Honkai][News] Failed to serialize item {}: {}",
                                                news_item.external_id,
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("[Honkai][News] Failed to fetch for language {}: {}", lang, e);
                            }
                        }
                    }

                    info!(
                        "[Honkai][News] Fetch complete - Total: {}, New: {}, Failed: {}",
                        total_items,
                        new_items,
                        failed_items
                    );
                }
            ).await {
                error!("[Honkai][News] Mutex error: {}", e);
            }
        })
    })?).await?;

    Ok(())
}

pub async fn schedule_tasks(sched: &JobScheduler, db: Arc<DatabaseConnections>, config: Arc<Settings>) -> Result<(), Box<dyn std::error::Error>> {
    schedule_codes(sched, db.clone(), config.clone()).await?;
    schedule_news(sched, db.clone(), config.clone()).await?;
    Ok(())
}