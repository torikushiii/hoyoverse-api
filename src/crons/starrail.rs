use std::sync::Arc;
use tokio_cron_scheduler::{JobScheduler, Job};
use tracing::{info, error};
use mongodb::{
    bson::{self, doc, Document},
    options::UpdateOptions,
    Collection,
};
use crate::{
    db::DatabaseConnections,
    resolvers::starrail::{StarRailResolver, news::sources::hoyolab::{
        NewsResolver as StarRailNewsResolver, 
        SUPPORTED_LANGUAGES as STARRAIL_LANGUAGES
    }},
    config::Settings,
};

async fn check_collection_empty(collection: &Collection<Document>) -> bool {
    match collection.count_documents(doc! {}).await {
        Ok(count) => count == 0,
        Err(_) => true // Treat errors as empty collection
    }
}

async fn schedule_codes(sched: &JobScheduler, db: Arc<DatabaseConnections>, config: Arc<Settings>) -> Result<(), Box<dyn std::error::Error>> {
    sched.add(Job::new_async("0 * * * * *", move |_, _| {
        let db = db.clone();
        let config = config.clone();
        Box::pin(async move {
            info!("[StarRail][Codes] Running scheduled code scraping");
            match StarRailResolver::fetch_codes(&config).await {
                Ok(mut new_codes) => {
                    let collection = db.mongo.collection::<Document>("starrail_codes");
                    
                    let is_empty = check_collection_empty(&collection).await;
                    if is_empty {
                        info!("[StarRail][Codes] Collection is empty, setting all codes as inactive");
                        for code in &mut new_codes {
                            code.active = false;
                        }
                    }

                    for code in new_codes {
                        let filter = doc! { "code": &code.code };
                        if let Ok(exists) = collection.find_one(filter.clone()).await {
                            if exists.is_some() {
                                continue;
                            }
                        }

                        if let Ok(mut doc) = bson::to_document(&code) {
                            doc.remove("date");
                            let insert_doc = doc! { 
                                "code": &code.code,
                                "active": code.active,
                                "date": bson::DateTime::now(),
                                "rewards": &code.rewards,
                                "source": &code.source
                            };
                            
                            if collection.insert_one(insert_doc).await.is_ok() {
                                info!(
                                    "[StarRail][Codes] Inserted new code: {} (active: {})", 
                                    code.code, 
                                    code.active
                                );
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

    Ok(())
}

async fn schedule_news(sched: &JobScheduler, db: Arc<DatabaseConnections>, config: Arc<Settings>) -> Result<(), Box<dyn std::error::Error>> {
    sched.add(Job::new_async("0 */15 * * * *", move |_, _| {
        let db = db.clone();
        let config = config.clone();
        Box::pin(async move {
            info!("[StarRail][News] Running scheduled news fetching");
            let mut total_items = 0;
            let mut new_items = 0;
            let mut failed_items = 0;

            let resolver = StarRailNewsResolver::new(&config);
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

pub async fn schedule_tasks(sched: &JobScheduler, db: Arc<DatabaseConnections>, config: Arc<Settings>) -> Result<(), Box<dyn std::error::Error>> {
    schedule_codes(sched, db.clone(), config.clone()).await?;
    schedule_news(sched, db.clone(), config.clone()).await?;
    Ok(())
} 