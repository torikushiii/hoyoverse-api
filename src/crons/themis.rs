use std::sync::Arc;
use tokio_cron_scheduler::{JobScheduler, Job};
use tracing::{info, error, debug};
use mongodb::{
    bson::{self, doc, Document},
    options::UpdateOptions,
    Collection,
};
use crate::{
    db::DatabaseConnections,
    resolvers::themis::{ThemisResolver, news::sources::hoyolab::{
        NewsResolver as ThemisNewsResolver,
        SUPPORTED_LANGUAGES as THEMIS_LANGUAGES
    }},
    config::Settings,
    services::code_verification::CodeVerificationService,
};

async fn check_collection_empty(collection: &Collection<Document>) -> bool {
    match collection.count_documents(doc! {}).await {
        Ok(count) => count == 0,
        Err(_) => true
    }
}

async fn schedule_codes(sched: &JobScheduler, db: Arc<DatabaseConnections>, config: Arc<Settings>) -> Result<(), Box<dyn std::error::Error>> {
    sched.add(Job::new_async("0 */5 * * * *", move |_, _| {
        let db = db.clone();
        let config = config.clone();
        Box::pin(async move {
            info!("[Themis][Codes] Running scheduled code scraping");
            match ThemisResolver::fetch_codes(&config).await {
                Ok(mut new_codes) => {
                    let collection = db.mongo.collection::<Document>("themis_codes");
                    let verifier = CodeVerificationService::new(db.clone(), config.clone());

                    let is_empty = check_collection_empty(&collection).await;
                    if is_empty {
                        info!("[Themis][Codes] Collection is empty, setting all codes as inactive");
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

                            if let Ok(_) = collection.insert_one(insert_doc).await {
                                info!(
                                    "[Themis][Codes] Inserted new code: {} (active: {})",
                                    code.code,
                                    code.active
                                );

                                if !is_empty {
                                    let mutex = db.redis.create_mutex().await.expect("Failed to create mutex");
                                    if let Err(e) = mutex.acquire(
                                        format!("themis_code_process:{}", code.code),
                                        || async {
                                            tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

                                            match verifier.verify_new_code(&code, "themis").await {
                                                Ok(is_active) => {
                                                    debug!(
                                                        "[Themis][Codes] Code {} verified: active = {}",
                                                        code.code,
                                                        is_active
                                                    );
                                                }
                                                Err(e) => {
                                                    error!(
                                                        "[Themis][Codes] Failed to verify code {}: {}",
                                                        code.code,
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                    ).await {
                                        error!("[Themis][Codes] Mutex error while processing code {}: {}", code.code, e);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("[Themis][Codes] Failed to scrape codes: {}", e);
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
                    error!("[Themis][News] Failed to create mutex: {}", e);
                    return;
                }
            };

            if let Err(e) = mutex.acquire(
                "themis_news_fetch".to_string(),
                || async {
                    info!("[Themis][News] Running scheduled news fetching");
                    let mut total_items = 0;
                    let mut new_items = 0;
                    let mut failed_items = 0;

                    let resolver = ThemisNewsResolver::new(&config);
                    for lang in THEMIS_LANGUAGES {
                        match resolver.fetch_news(lang).await {
                            Ok(news_items) => {
                                total_items += news_items.len();
                                let collection = db.mongo.collection::<Document>("themis_news");
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
                                                            "[Themis][News] Inserted new item: {} ({}) [{}]",
                                                            news_item.title,
                                                            news_item.news_type,
                                                            news_item.lang
                                                        );
                                                    }
                                                }
                                                Err(e) => {
                                                    failed_items += 1;
                                                    error!(
                                                        "[Themis][News] Failed to update item {}: {}",
                                                        news_item.external_id,
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            failed_items += 1;
                                            error!(
                                                "[Themis][News] Failed to serialize item {}: {}",
                                                news_item.external_id,
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("[Themis][News] Failed to fetch for language {}: {}", lang, e);
                            }
                        }
                    }

                    info!(
                        "[Themis][News] Fetch complete - Total: {}, New: {}, Failed: {}",
                        total_items,
                        new_items,
                        failed_items
                    );
                }
            ).await {
                error!("[Themis][News] Mutex error: {}", e);
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