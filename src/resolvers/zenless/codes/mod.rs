pub mod sources;

#[cfg(test)]
mod tests;

use crate::{types::GameCode, config::Settings};
use sources::{game8, gamerant, pcgamesn, hoyolab};

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let mut codes = Vec::new();

    let (game8_codes, gamerant_codes, pcgamesn_codes, hoyolab_codes) = tokio::join!(
        game8::fetch_codes(config),
        gamerant::fetch_codes(config),
        pcgamesn::fetch_codes(config),
        hoyolab::fetch_codes(config),
    );

    if let Ok(mut source_codes) = game8_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = gamerant_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = pcgamesn_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = hoyolab_codes {
        codes.append(&mut source_codes);
    }

    codes.sort_by(|a, b| a.code.cmp(&b.code));
    codes.dedup_by(|a, b| a.code == b.code);

    Ok(codes)
} 