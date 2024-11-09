pub mod sources;

use crate::{types::GameCode, config::Settings};
use anyhow::Result;

pub async fn fetch_codes(config: &Settings) -> Result<Vec<GameCode>> {
    let codes = sources::fandom::fetch_codes(config).await?;
    Ok(codes)
} 