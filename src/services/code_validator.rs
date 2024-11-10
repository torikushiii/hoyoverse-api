use std::sync::Arc;
use tokio_cron_scheduler::{JobScheduler, Job};
use mongodb::{
    bson::doc,
    Cursor,
};
use futures_util::TryStreamExt;
use tracing::{info, error, debug, warn};
use crate::{
    db::DatabaseConnections,
    types::GameCode,
    config::{GameAccount, Settings},
};
use std::future::Future;
use std::pin::Pin;
use serde::Deserialize;

#[derive(Debug)]
pub enum ValidationResult {
    Valid,
    AlreadyRedeemed,
    Expired,
    Invalid,
    Cooldown,
    InvalidCredentials,
    MaxUsageReached,
    Unknown(i32, String),
}

#[derive(Debug, Deserialize)]
struct HoyolabResponse {
    retcode: i32,
    message: String,
}

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

    async fn validate_zenless_code(&self, code: &str, account: &GameAccount) -> anyhow::Result<ValidationResult> {
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
                ("cdkey", &code.to_string()),
            ])
            .send()
            .await?;

        let status = response.status();
        
        if !status.is_success() {
            error!("[Zenless] Failed HTTP request for code {}: Status {}", code, status);
            return Ok(ValidationResult::Unknown(status.as_u16() as i32, format!("Status {}", status)));
        }

        let response_body: HoyolabResponse = response.json().await?;
        
        let result = match response_body.retcode {
            0 => ValidationResult::Valid,
            -2017 | -2018 => {
                debug!("[Zenless] Code {} is already redeemed", code);
                ValidationResult::AlreadyRedeemed
            },
            -2001 => {
                info!("[Zenless] Code {} is expired", code);
                ValidationResult::Expired
            },
            -2003 => {
                info!("[Zenless] Code {} is invalid", code);
                ValidationResult::Invalid
            },
            -2016 => {
                warn!("[Zenless] Code {} is in cooldown", code);
                ValidationResult::Cooldown
            },
            -2011 => {
                debug!("[Zenless] Code {} requires higher Inter-Knot Level", code);
                ValidationResult::Valid
            },
            -2006 => {
                info!("[Zenless] Code {} has reached maximum usage limit", code);
                ValidationResult::MaxUsageReached
            },
            -1071 => {
                error!("[Zenless] Invalid account credentials");
                ValidationResult::InvalidCredentials
            },
            code => {
                error!("[Zenless] Unknown response code {} for code {}: {}", 
                    code, code, response_body.message);
                ValidationResult::Unknown(code, response_body.message)
            }
        };

        debug!("[Zenless] Validation result for code {}: {:?}", code, result);
        Ok(result)
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

    async fn validate_starrail_code(&self, code: &str, account: &GameAccount) -> anyhow::Result<ValidationResult> {
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

        let status = response.status();
        
        if !status.is_success() {
            error!("[StarRail] Failed HTTP request for code {}: Status {}", code, status);
            return Ok(ValidationResult::Unknown(status.as_u16() as i32, format!("Status {}", status)));
        }

        let response_body: HoyolabResponse = response.json().await?;
        
        let result = match response_body.retcode {
            0 => ValidationResult::Valid,
            -2017 | -2018 => {
                debug!("[StarRail] Code {} is already redeemed", code);
                ValidationResult::AlreadyRedeemed
            },
            -2001 => {
                info!("[StarRail] Code {} is expired", code);
                ValidationResult::Expired
            },
            -2003 => {
                info!("[StarRail] Code {} is invalid", code);
                ValidationResult::Invalid
            },
            -2016 => {
                warn!("[StarRail] Code {} is in cooldown", code);
                ValidationResult::Cooldown
            },
            -2006 => {
                info!("[StarRail] Code {} has reached maximum usage limit", code);
                ValidationResult::MaxUsageReached
            },
            -1071 => {
                error!("[StarRail] Invalid account credentials");
                ValidationResult::InvalidCredentials
            },
            code => {
                error!("[StarRail] Unknown response code {} for code {}: {}", 
                    code, code, response_body.message);
                ValidationResult::Unknown(code, response_body.message)
            }
        };

        debug!("[StarRail] Validation result for code {}: {:?}", code, result);
        Ok(result)
    }

    async fn validate_genshin_code(&self, code: &str, account: &GameAccount) -> anyhow::Result<ValidationResult> {
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
                ("cdkey", &code.to_string()),
                ("game_biz", &String::from("hk4e_global")),
                ("sLangKey", &String::from("en-us")),
                ("t", &timestamp.to_string()),
            ])
            .send()
            .await?;

        let status = response.status();
        
        if !status.is_success() {
            error!("[Genshin] Failed HTTP request for code {}: Status {}", code, status);
            return Ok(ValidationResult::Unknown(status.as_u16() as i32, format!("Status {}", status)));
        }

        let response_body: HoyolabResponse = response.json().await?;
        
        let result = match response_body.retcode {
            0 => ValidationResult::Valid,
            -2017 | -2018 => {
                debug!("[Genshin] Code {} is already redeemed", code);
                ValidationResult::AlreadyRedeemed
            },
            -2001 => {
                info!("[Genshin] Code {} is expired", code);
                ValidationResult::Expired
            },
            -2003 => {
                info!("[Genshin] Code {} is invalid", code);
                ValidationResult::Invalid
            },
            -2016 => {
                warn!("[Genshin] Code {} is in cooldown", code);
                ValidationResult::Cooldown
            },
            -2006 => {
                info!("[Genshin] Code {} has reached maximum usage limit", code);
                ValidationResult::MaxUsageReached
            },
            -1071 => {
                error!("[Genshin] Invalid account credentials");
                ValidationResult::InvalidCredentials
            },
            code => {
                error!("[Genshin] Unknown response code {} for code {}: {}", 
                    code, code, response_body.message);
                ValidationResult::Unknown(code, response_body.message)
            }
        };

        debug!("[Genshin] Validation result for code {}: {:?}", code, result);
        Ok(result)
    }
} 