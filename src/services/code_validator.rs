use crate::{
    config::{GameAccount, Settings},
    db::DatabaseConnections,
    error::ValidationResult,
    services::validation::{
        GameValidator, GenshinValidator, StarRailValidator, ThemisValidator, ZenlessValidator,
    },
    services::webhook::WebhookService,
    types::GameCode,
};
use futures_util::TryStreamExt;
use mongodb::{bson::doc, Cursor};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, error, info, warn};

pub struct CodeValidationService {
    db: Arc<DatabaseConnections>,
    config: Arc<Settings>,
    client: reqwest::Client,
    webhook: WebhookService,
}

impl CodeValidationService {
    pub fn new(db: Arc<DatabaseConnections>, config: Arc<Settings>) -> Self {
        Self {
            db: db.clone(),
            config: config.clone(),
            client: reqwest::Client::builder()
                .user_agent(&config.server.user_agent)
                .build()
                .expect("Failed to create HTTP client"),
            webhook: WebhookService::new(config),
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        info!("Initializing code validation service");
        let sched = JobScheduler::new().await?;

        self.schedule_validation_jobs(&sched).await?;

        info!("Starting code validation scheduler");
        sched.start().await?;

        Ok(())
    }

    async fn schedule_validation_jobs(&self, sched: &JobScheduler) -> anyhow::Result<()> {
        let db = self.db.clone();
        let config = self.config.clone();

        sched
            .add(Job::new_async("0 */30 * * * *", move |_, _| {
                let db = db.clone();
                let config = config.clone();
                Box::pin(async move {
                    let validator = CodeValidationService::new(db, config);
                    validator.validate_all_codes().await;
                })
            })?)
            .await?;

        Ok(())
    }

    pub async fn validate_all_codes(&self) {
        info!("Running code validation for all games...");

        self.validate_starrail_codes().await;
        self.validate_genshin_codes().await;
        self.validate_zenless_codes().await;
        self.validate_themis_codes().await;
    }

    async fn validate_starrail_codes(&self) {
        let collection = self.db.mongo.collection::<GameCode>("starrail_codes");
        let active_codes = match collection.find(doc! { "active": true }).await {
            Ok(cursor) => cursor,
            Err(e) => {
                error!("[StarRail] Failed to fetch active codes: {}", e);
                return;
            }
        };

        let accounts = &self.config.game_accounts.starrail;
        if accounts.is_empty() {
            debug!("[StarRail] No accounts configured for validation");
            return;
        }

        self.process_codes(
            active_codes,
            accounts,
            "starrail_codes",
            |service, code, account| Box::pin(service.validate_starrail_code(code, account)),
            "[StarRail]",
        )
        .await;
    }

    async fn validate_genshin_codes(&self) {
        let collection = self.db.mongo.collection::<GameCode>("genshin_codes");
        let active_codes = match collection.find(doc! { "active": true }).await {
            Ok(cursor) => cursor,
            Err(e) => {
                error!("[Genshin] Failed to fetch active codes: {}", e);
                return;
            }
        };

        let accounts = &self.config.game_accounts.genshin;
        if accounts.is_empty() {
            debug!("[Genshin] No accounts configured for validation");
            return;
        }

        self.process_codes(
            active_codes,
            accounts,
            "genshin_codes",
            |service, code, account| Box::pin(service.validate_genshin_code(code, account)),
            "[Genshin]",
        )
        .await;
    }

    async fn validate_zenless_codes(&self) {
        let collection = self.db.mongo.collection::<GameCode>("zenless_codes");
        let active_codes = match collection.find(doc! { "active": true }).await {
            Ok(cursor) => cursor,
            Err(e) => {
                error!("[Zenless] Failed to fetch active codes: {}", e);
                return;
            }
        };

        let accounts = &self.config.game_accounts.zenless;
        if accounts.is_empty() {
            debug!("[Zenless] No accounts configured for validation");
            return;
        }

        self.process_codes(
            active_codes,
            accounts,
            "zenless_codes",
            |service, code, account| Box::pin(service.validate_zenless_code(code, account)),
            "[Zenless]",
        )
        .await;
    }

    async fn validate_themis_codes(&self) {
        let collection = self.db.mongo.collection::<GameCode>("themis_codes");
        let active_codes = match collection.find(doc! { "active": true }).await {
            Ok(cursor) => cursor,
            Err(e) => {
                error!("[Themis] Failed to fetch active codes: {}", e);
                return;
            }
        };

        let accounts = &self.config.game_accounts.themis;
        if accounts.is_empty() {
            debug!("[Themis] No accounts configured for validation");
            return;
        }

        self.process_codes(
            active_codes,
            accounts,
            "themis_codes",
            |service, code, account| Box::pin(service.validate_themis_code(code, account)),
            "[Themis]",
        )
        .await;
    }

    pub async fn validate_starrail_code(
        &self,
        code: &str,
        account: &GameAccount,
    ) -> anyhow::Result<ValidationResult> {
        StarRailValidator
            .validate_code(&self.client, code, account)
            .await
    }

    pub async fn validate_genshin_code(
        &self,
        code: &str,
        account: &GameAccount,
    ) -> anyhow::Result<ValidationResult> {
        GenshinValidator
            .validate_code(&self.client, code, account)
            .await
    }

    pub async fn validate_zenless_code(
        &self,
        code: &str,
        account: &GameAccount,
    ) -> anyhow::Result<ValidationResult> {
        ZenlessValidator
            .validate_code(&self.client, code, account)
            .await
    }

    pub async fn validate_themis_code(
        &self,
        code: &str,
        account: &GameAccount,
    ) -> anyhow::Result<ValidationResult> {
        ThemisValidator
            .validate_code(&self.client, code, account)
            .await
    }

    async fn process_codes(
        &self,
        mut cursor: Cursor<GameCode>,
        accounts: &[GameAccount],
        collection_name: &str,
        validator: for<'a> fn(
            &'a Self,
            &'a str,
            &'a GameAccount,
        ) -> Pin<
            Box<dyn Future<Output = anyhow::Result<ValidationResult>> + Send + 'a>,
        >,
        log_prefix: &str,
    ) {
        let test_account = &accounts[0];
        let mut codes_to_update = Vec::new();

        if let Some(first_code) = cursor.try_next().await.ok().flatten() {
            match validator(self, &first_code.code, test_account).await {
                Ok(ValidationResult::InvalidCredentials) => {
                    error!("{} Invalid account credentials detected", log_prefix);
                    if let Err(e) = self
                        .webhook
                        .send_invalid_credentials_notification(
                            collection_name.split('_').next().unwrap_or("unknown"),
                        )
                        .await
                    {
                        error!(
                            "{} Failed to send invalid credentials notification: {}",
                            log_prefix, e
                        );
                    }
                    return;
                }
                Ok(_) => {
                    cursor = self
                        .db
                        .mongo
                        .collection::<GameCode>(collection_name)
                        .find(doc! { "active": true })
                        .await
                        .expect("Failed to recreate cursor");
                }
                Err(e) => {
                    error!("{} Error validating first code: {}", log_prefix, e);
                    return;
                }
            }
        }

        while let Ok(Some(code)) = cursor.try_next().await {
            let code_clone = code.clone();
            let result = self
                .db
                .redis
                .create_mutex()
                .await
                .expect("Failed to create distributed mutex")
                .acquire(format!("code_validation:{}", code.code), || async {
                    match validator(self, &code.code, test_account).await {
                        Ok(result) => match result {
                            ValidationResult::Valid
                            | ValidationResult::AlreadyRedeemed
                            | ValidationResult::Cooldown => {
                                // Code is still considered valid
                            }
                            ValidationResult::Unknown(code, message) if code == -1009 => {
                                warn!(
                                    "{} Temporary system busy response for code {}: {}",
                                    log_prefix, code_clone.code, message
                                );
                            }
                            ValidationResult::InvalidCredentials => {
                                error!(
                                    "{} Invalid credentials detected during validation",
                                    log_prefix
                                );
                                return;
                            }
                            _ => {
                                info!(
                                    "{} Code {} is no longer valid: {:?}",
                                    log_prefix, code.code, result
                                );
                                codes_to_update.push(code);
                            }
                        },
                        Err(e) => {
                            error!(
                                "{} Failed to validate code {}: {}",
                                log_prefix, code.code, e
                            );
                        }
                    }

                    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
                })
                .await;

            if let Err(e) = result {
                error!(
                    "{} Mutex error while validating code {}: {}",
                    log_prefix, code_clone.code, e
                );
            }
        }

        // Update invalid codes in database
        let collection = self.db.mongo.collection::<GameCode>(collection_name);
        for code in codes_to_update {
            if let Err(e) = collection
                .update_one(
                    doc! { "code": &code.code },
                    doc! { "$set": { "active": false } },
                )
                .await
            {
                error!("{} Failed to update code status: {}", log_prefix, e);
            }
        }
    }
}
