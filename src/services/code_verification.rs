use std::sync::Arc;
use mongodb::bson::{doc, Document};
use tracing::{info, error, debug, warn};
use crate::{
    db::DatabaseConnections,
    types::GameCode,
    config::Settings,
    error::ValidationResult,
    services::code_validator::CodeValidationService,
    services::webhook::WebhookService,
};

pub struct CodeVerificationService {
    db: Arc<DatabaseConnections>,
    config: Arc<Settings>,
    validator: CodeValidationService,
    webhook: WebhookService,
}

impl CodeVerificationService {
    pub fn new(db: Arc<DatabaseConnections>, config: Arc<Settings>) -> Self {
        Self {
            db: db.clone(),
            config: config.clone(),
            validator: CodeValidationService::new(db, config.clone()),
            webhook: WebhookService::new(config),
        }
    }

    pub async fn verify_new_code(&self, code: &GameCode, game_type: &str) -> anyhow::Result<bool> {
        info!("[{}] Verifying new code: {}", game_type, code.code);

        let accounts = match game_type {
            "starrail" => &self.config.game_accounts.starrail,
            "genshin" => &self.config.game_accounts.genshin,
            "zenless" => &self.config.game_accounts.zenless,
            "themis" => &self.config.game_accounts.themis,
            _ => {
                error!("[{}] Invalid game type", game_type);
                return Ok(false);
            }
        };

        if accounts.is_empty() {
            debug!("[{}] No accounts configured for validation", game_type);
            return Ok(true); // Consider valid if no accounts are configured
        }

        let test_account = &accounts[0];

        let result = match game_type {
            "starrail" => self.validator.validate_starrail_code(&code.code, test_account).await?,
            "genshin" => self.validator.validate_genshin_code(&code.code, test_account).await?,
            "zenless" => self.validator.validate_zenless_code(&code.code, test_account).await?,
            "themis" => self.validator.validate_themis_code(&code.code, test_account).await?,
            _ => return Ok(false),
        };

        let is_active = match result {
            ValidationResult::Valid => true,
            ValidationResult::Cooldown => true, // Consider valid if in cooldown
            ValidationResult::AlreadyRedeemed => true, // Consider valid if already redeemed
            ValidationResult::Expired => false,
            ValidationResult::Invalid => false,
            ValidationResult::MaxUsageReached => false,
            ValidationResult::InvalidCredentials => {
                error!("[{}] Invalid credentials during verification", game_type);
                true // Consider valid if we can't verify due to credentials
            }
            ValidationResult::Unknown(_, _) => {
                warn!("[{}] Unknown validation result for code {}", game_type, code.code);
                true // Consider valid if unknown result
            }
        };

        let collection = self.db.mongo.collection::<Document>(&format!("{}_codes", game_type));

        if let Err(e) = collection
            .update_one(
                doc! { "code": &code.code },
                doc! { "$set": { "active": is_active } },
            )
            .await
        {
            error!("[{}] Failed to update code status: {}", game_type, e);
        }

        if is_active {
            if let Err(e) = self.webhook.send_new_code_notification(code, game_type).await {
                error!("[{}] Failed to send webhook notification: {}", game_type, e);
            }
        }

        Ok(is_active)
    }
}