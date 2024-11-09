pub mod sources;

#[cfg(test)]
mod tests;

use crate::{types::GameCode, config::Settings};
use sources::game8;

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let mut codes = Vec::new();

    if let Ok(mut source_codes) = game8::fetch_codes(config).await {
        codes.append(&mut source_codes);
    }

    codes.sort_by(|a, b| a.code.cmp(&b.code));
    codes.dedup_by(|a, b| a.code == b.code);

    Ok(codes)
} 