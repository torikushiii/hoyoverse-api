pub mod sources;

#[cfg(test)]
mod tests;

use crate::{config::Settings, types::GameCode};
use sources::{eurogamer, fandom, game8, hoyolab, pcgamer, prydwen};

pub async fn fetch_codes(config: &Settings) -> anyhow::Result<Vec<GameCode>> {
    let mut codes = Vec::new();

    // Fetch from multiple sources concurrently
    let (eurogamer_codes, game8_codes, hoyolab_codes, prydwen_codes, fandom_codes, pcgamer_codes) = tokio::join!(
        eurogamer::fetch_codes(config),
        game8::fetch_codes(config),
        hoyolab::fetch_codes(config),
        prydwen::fetch_codes(config),
        fandom::fetch_codes(config),
        pcgamer::fetch_codes(config),
    );

    // Combine results, ignoring errors from individual sources
    if let Ok(mut source_codes) = eurogamer_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = game8_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = hoyolab_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = prydwen_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = fandom_codes {
        codes.append(&mut source_codes);
    }

    if let Ok(mut source_codes) = pcgamer_codes {
        codes.append(&mut source_codes);
    }

    codes.sort_by(|a, b| a.code.to_lowercase().cmp(&b.code.to_lowercase()));
    codes.dedup_by(|a, b| a.code.to_lowercase() == b.code.to_lowercase());

    Ok(codes)
}
