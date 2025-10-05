pub mod sources;

#[cfg(test)]
mod tests;

use crate::{config::Settings, types::GameCode};
use sources::totwiki;

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let mut codes = Vec::new();

    // For now we only have tot-wiki as a source
    if let Ok(mut source_codes) = totwiki::fetch_codes(config).await {
        codes.append(&mut source_codes);
    }

    codes.sort_by(|a, b| a.code.to_lowercase().cmp(&b.code.to_lowercase()));
    codes.dedup_by(|a, b| a.code.to_lowercase() == b.code.to_lowercase());

    Ok(codes)
}
