use std::sync::Arc;
use tokio_cron_scheduler::{JobScheduler, Job};
use mongodb::{
    bson::doc,
    Cursor,
};
use futures_util::TryStreamExt;
use tracing::{info, error, debug};
use crate::{
    db::DatabaseConnections,
    types::{GameCode, HoyolabResponse},
    config::{GameAccount, Settings},
    error::{HoyolabRetcode, ValidationResult},
};
use std::future::Future;
use std::pin::Pin;

pub struct CodeValidationService {
    db: Arc<DatabaseConnections>,
    config: Arc<Settings>,
}

impl CodeValidationService {
    pub fn new(db: Arc<DatabaseConnections>, config: Arc<Settings>) -> Self {
        Self { db, config }
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

        sched.add(Job::new_async("0 */30 * * * *", move |_, _| {
            let db = db.clone();
            let config = config.clone();
            Box::pin(async move {
                let validator = CodeValidationService::new(db, config);
                validator.validate_all_codes().await;
            })
        })?).await?;

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
        let active_codes = match collection
            .find(doc! { "active": true })
            .await
        {
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
        ).await;
    }

    async fn validate_genshin_codes(&self) {
        let collection = self.db.mongo.collection::<GameCode>("genshin_codes");
        let active_codes = match collection
            .find(doc! { "active": true })
            .await
        {
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
        ).await;
    }

    async fn validate_zenless_codes(&self) {
        let collection = self.db.mongo.collection::<GameCode>("zenless_codes");
        let active_codes = match collection
            .find(doc! { "active": true })
            .await
        {
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
        ).await;
    }

    async fn validate_themis_codes(&self) {
        let collection = self.db.mongo.collection::<GameCode>("themis_codes");
        let active_codes = match collection
            .find(doc! { "active": true })
            .await
        {
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
        ).await;
    }

    pub async fn validate_starrail_code(&self, code: &str, account: &GameAccount) -> anyhow::Result<ValidationResult> {
        let client = reqwest::Client::new();
        let url = "https://sg-hkrpg-api.hoyoverse.com/common/apicdkey/api/webExchangeCdkey";
        let timestamp = chrono::Utc::now().timestamp_millis();

        let response = client
            .get(url)
            .header("User-Agent", &self.config.server.user_agent)
            .header("Cookie", format!(
                "cookie_token_v2={}; account_mid_v2={}; account_id_v2={}",
                account.cookie_token_v2, account.account_mid_v2, account.account_id_v2
            ))
            .query(&[
                ("cdkey", code),
                ("game_biz", "hkrpg_global"),
                ("lang", "en"),
                ("region", &account.region),
                ("t", &timestamp.to_string()),
                ("uid", &account.uid),
            ])
            .send()
            .await?;

        self.handle_hoyolab_response(response, code, "StarRail").await
    }

    pub async fn validate_zenless_code(&self, code: &str, account: &GameAccount) -> anyhow::Result<ValidationResult> {
        let client = reqwest::Client::new();
        let url = "https://public-operation-nap.hoyoverse.com/common/apicdkey/api/webExchangeCdkey";
        let timestamp = chrono::Utc::now().timestamp_millis();

        let response = client
            .get(url)
            .header("User-Agent", &self.config.server.user_agent)
            .header("Cookie", format!(
                "cookie_token_v2={}; account_mid_v2={}; account_id_v2={}",
                account.cookie_token_v2, account.account_mid_v2, account.account_id_v2
            ))
            .query(&[
                ("t", &timestamp.to_string()),
                ("lang", &String::from("en")),
                ("game_biz", &String::from("nap_global")),
                ("uid", &account.uid),
                ("region", &account.region),
                ("cdkey", &String::from(code)),
            ])
            .send()
            .await?;

        self.handle_hoyolab_response(response, code, "Zenless").await
    }

    pub async fn validate_themis_code(&self, code: &str, account: &GameAccount) -> anyhow::Result<ValidationResult> {
        let client = reqwest::Client::new();
        let url = "https://sg-public-api.hoyoverse.com/common/apicdkey/api/webExchangeCdkey";
        let timestamp = chrono::Utc::now().timestamp_millis();

        let response = client
            .get(url)
            .header("User-Agent", &self.config.server.user_agent)
            .header("Cookie", format!(
                "cookie_token_v2={}; account_mid_v2={}; account_id_v2={}",
                account.cookie_token_v2, account.account_mid_v2, account.account_id_v2
            ))
            .query(&[
                ("t", &timestamp.to_string()),
                ("lang", &String::from("en")),
                ("game_biz", &String::from("nxx_global")),
                ("uid", &account.uid),
                ("region", &account.region),
                ("cdkey", &String::from(code)),
            ])
            .send()
            .await?;

        self.handle_hoyolab_response(response, code, "Themis").await
    }

    pub async fn validate_genshin_code(&self, code: &str, account: &GameAccount) -> anyhow::Result<ValidationResult> {
        let client = reqwest::Client::new();
        let url = "https://sg-hk4e-api.hoyoverse.com/common/apicdkey/api/webExchangeCdkey";
        let timestamp = chrono::Utc::now().timestamp_millis();

        let response = client
            .get(url)
            .header("User-Agent", &self.config.server.user_agent)
            .header("Cookie", format!(
                "cookie_token_v2={}; account_mid_v2={}; account_id_v2={}",
                account.cookie_token_v2, account.account_mid_v2, account.account_id_v2
            ))
            .query(&[
                ("uid", &account.uid),
                ("region", &account.region),
                ("lang", &String::from("en")),
                ("cdkey", &String::from(code)),
                ("game_biz", &String::from("hk4e_global")),
                ("sLangKey", &String::from("en-us")),
                ("t", &timestamp.to_string()),
            ])
            .send()
            .await?;

        self.handle_hoyolab_response(response, code, "Genshin").await
    }

    async fn process_codes(
        &self,
        mut cursor: Cursor<GameCode>,
        accounts: &[GameAccount],
        collection_name: &str,
        validator: for<'a> fn(&'a Self, &'a str, &'a GameAccount) -> Pin<Box<dyn Future<Output = anyhow::Result<ValidationResult>> + Send + 'a>>,
        log_prefix: &str,
    ) {
        let test_account = &accounts[0];
        let mut codes_to_update = Vec::new();

        while let Ok(Some(code)) = cursor.try_next().await {
            let code_clone = code.clone();
            let result = self.db.redis.create_mutex().await
                .expect("Failed to create distributed mutex")
                .acquire(
                    format!("code_validation:{}", code.code),
                    || async {
                        match validator(self, &code.code, test_account).await {
                            Ok(result) => {
                                match result {
                                    ValidationResult::Valid | ValidationResult::AlreadyRedeemed | ValidationResult::Cooldown => {
                                        // Code is still considered valid
                                    },
                                    _ => {
                                        debug!("{} Code {} is no longer valid: {:?}", log_prefix, code.code, result);
                                        codes_to_update.push(code);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("{} Failed to validate code {}: {}", log_prefix, code.code, e);
                            }
                        }

                        tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
                    }
                ).await;

            if let Err(e) = result {
                error!("{} Mutex error while validating code {}: {}", log_prefix, code_clone.code, e);
            }
        }

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

    async fn handle_hoyolab_response(&self, response: reqwest::Response, code: &str, game: &str) -> anyhow::Result<ValidationResult> {
        let status = response.status();

        if !status.is_success() {
            error!("[{}] Failed HTTP request for code {}: Status {}", game, code, status);
            return Ok(ValidationResult::Unknown(
                status.as_u16() as i32,
                format!("Status {}", status)
            ));
        }

        let response_body: HoyolabResponse = response.json().await?;

        Ok(HoyolabRetcode::from_code(response_body.retcode)
            .map(|rc| rc.into_validation_result())
            .unwrap_or_else(|| {
                error!(
                    "[{}] Unknown response code {} for code {}: {}",
                    game,
                    response_body.retcode,
                    code,
                    response_body.message
                );
                ValidationResult::Unknown(response_body.retcode, response_body.message)
            }))
    }
}