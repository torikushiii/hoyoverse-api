use async_trait::async_trait;
use reqwest::Client;

use crate::{
    config::GameAccount,
    error::ValidationResult,
    types::HoyolabResponse,
};

#[async_trait]
pub trait GameValidator {
    fn api_endpoint(&self) -> &'static str;
    fn game_biz(&self) -> &'static str;
    fn game_name(&self) -> &'static str;

    fn build_query_params<'a>(&self, code: &'a str, account: &'a GameAccount, timestamp: i64) -> Vec<(&'static str, String)> {
        vec![
            ("cdkey", code.to_string()),
            ("uid", account.uid.clone()),
            ("region", account.region.clone()),
            ("lang", "en".to_string()),
            ("game_biz", self.game_biz().to_string()),
            ("t", timestamp.to_string()),
        ]
    }

    async fn validate_code(&self, client: &Client, code: &str, account: &GameAccount) -> anyhow::Result<ValidationResult> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let query_params = self.build_query_params(code, account, timestamp);

        let response = client
            .get(self.api_endpoint())
            .query(&query_params)
            .header("Cookie", format!(
                "cookie_token_v2={}; account_mid_v2={}; account_id_v2={}",
                account.cookie_token_v2, account.account_mid_v2, account.account_id_v2
            ))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            tracing::error!("[{}] Failed HTTP request for code {}: Status {}", self.game_name(), code, status);
            return Ok(ValidationResult::Unknown(
                status.as_u16() as i32,
                format!("Status {}", status),
            ));
        }

        let response_body: HoyolabResponse = response.json().await?;
        Ok(crate::error::HoyolabRetcode::from_code(response_body.retcode)
            .map(|rc| rc.into_validation_result())
            .unwrap_or_else(|| {
                tracing::error!(
                    "[{}] Unknown response code {} for code {}: {}",
                    self.game_name(),
                    response_body.retcode,
                    code,
                    response_body.message
                );
                ValidationResult::Unknown(response_body.retcode, response_body.message)
            }))
    }
}

pub struct StarRailValidator;
pub struct GenshinValidator;
pub struct ZenlessValidator;
pub struct ThemisValidator;

#[async_trait]
impl GameValidator for StarRailValidator {
    fn api_endpoint(&self) -> &'static str {
        "https://sg-hkrpg-api.hoyoverse.com/common/apicdkey/api/webExchangeCdkey"
    }

    fn game_biz(&self) -> &'static str {
        "hkrpg_global"
    }

    fn game_name(&self) -> &'static str {
        "StarRail"
    }
}

#[async_trait]
impl GameValidator for GenshinValidator {
    fn api_endpoint(&self) -> &'static str {
        "https://sg-hk4e-api.hoyoverse.com/common/apicdkey/api/webExchangeCdkey"
    }

    fn game_biz(&self) -> &'static str {
        "hk4e_global"
    }

    fn game_name(&self) -> &'static str {
        "Genshin"
    }

    fn build_query_params<'a>(&self, code: &'a str, account: &'a GameAccount, timestamp: i64) -> Vec<(&'static str, String)> {
        vec![
            ("cdkey", code.to_string()),
            ("uid", account.uid.clone()),
            ("region", account.region.clone()),
            ("lang", "en".to_string()),
            ("game_biz", self.game_biz().to_string()),
            ("t", timestamp.to_string()),
            ("sLangKey", "en-us".to_string()),
        ]
    }
}

#[async_trait]
impl GameValidator for ZenlessValidator {
    fn api_endpoint(&self) -> &'static str {
        "https://public-operation-nap.hoyoverse.com/common/apicdkey/api/webExchangeCdkey"
    }

    fn game_biz(&self) -> &'static str {
        "nap_global"
    }

    fn game_name(&self) -> &'static str {
        "Zenless"
    }
}

#[async_trait]
impl GameValidator for ThemisValidator {
    fn api_endpoint(&self) -> &'static str {
        "https://sg-public-api.hoyoverse.com/common/apicdkey/api/webExchangeCdkey"
    }

    fn game_biz(&self) -> &'static str {
        "nxx_global"
    }

    fn game_name(&self) -> &'static str {
        "Themis"
    }
}