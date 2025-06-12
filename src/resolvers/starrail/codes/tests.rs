use super::*;
use crate::config::Settings;
use tokio;
use tracing_test::traced_test;

fn get_test_config() -> Settings {
    Settings::new().expect("Failed to load test configuration")
}

#[tokio::test]
#[traced_test]
async fn test_eurogamer_fetch() {
    let config = get_test_config();
    let codes = eurogamer::fetch_codes(&config).await.unwrap();

    // Basic validation
    assert!(!codes.is_empty(), "Should fetch at least one code from Eurogamer");

    // Validate code structure
    for code in &codes {
        // Codes should be uppercase and contain only letters and numbers
        assert!(code.code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));

        // Each code should have at least one reward
        assert!(!code.rewards.is_empty(), "Code should have rewards: {}", code.code);

        // Source should be correct
        assert_eq!(code.source, "eurogamer");

        // Should be marked as active
        assert!(code.active);

        println!("Found Eurogamer code: {} with rewards: {:?}", code.code, code.rewards);
    }
}

#[tokio::test]
#[traced_test]
async fn test_game8_fetch() {
    let config = get_test_config();
    let codes = game8::fetch_codes(&config).await.unwrap();

    // Basic validation
    assert!(!codes.is_empty(), "Should fetch at least one code from Game8");

    // Validate code structure
    for code in &codes {
        // Codes should be uppercase and contain only letters and numbers
        assert!(code.code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));

        // Each code should have at least one reward
        assert!(!code.rewards.is_empty(), "Code should have rewards: {}", code.code);

        // Source should be correct
        assert_eq!(code.source, "game8");

        // Should be marked as active
        assert!(code.active);

        println!("Found Game8 code: {} with rewards: {:?}", code.code, code.rewards);
    }
}

#[tokio::test]
#[traced_test]
async fn test_fetch_codes() {
    let config = get_test_config();
    let codes = fetch_codes(&config).await.unwrap();

    // Basic validation
    assert!(!codes.is_empty(), "Should fetch codes from at least one source");

    // Track unique codes to verify deduplication
    let mut unique_codes = std::collections::HashSet::new();

    for code in &codes {
        // Codes should be uppercase and contain only letters and numbers
        assert!(code.code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));

        // Each code should have at least one reward
        assert!(!code.rewards.is_empty(), "Code should have rewards: {}", code.code);

        // Source should be one of our known sources
        assert!(
            ["game8", "eurogamer", "prydwen", "hoyolab", "fandom", "pcgamer"].contains(&code.source.as_str()),
            "Invalid source: {}", code.source
        );

        // Should be marked as active
        assert!(code.active);

        // Verify no duplicate codes
        assert!(unique_codes.insert(&code.code), "Found duplicate code: {}", code.code);

        println!("Found code from {}: {} with rewards: {:?}",
            code.source, code.code, code.rewards);
    }

    println!("Total unique codes found: {}", unique_codes.len());
}

#[tokio::test]
#[traced_test]
async fn test_hoyolab_fetch() {
    let config = get_test_config();
    let codes = hoyolab::fetch_codes(&config).await.unwrap();

    if !codes.is_empty() {
        for code in &codes {
            // Codes should be uppercase and contain only letters and numbers
            assert!(code.code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));

            // Each code should have at least one reward
            assert!(!code.rewards.is_empty(), "Code should have rewards: {}", code.code);

            // Source should be correct
            assert_eq!(code.source, "hoyolab");

            // Should be marked as active
            assert!(code.active);

            println!("Found HoyoLab code: {} with rewards: {:?}", code.code, code.rewards);
        }
    }
}

#[tokio::test]
#[traced_test]
async fn test_prydwen_fetch() {
    let config = get_test_config();
    let codes = prydwen::fetch_codes(&config).await.unwrap();

    if !codes.is_empty() {
        for code in &codes {
            // Codes should be uppercase and contain only letters and numbers
            assert!(code.code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));

            // Each code should have at least one reward
            assert!(!code.rewards.is_empty(), "Code should have rewards: {}", code.code);

            // Source should be correct
            assert_eq!(code.source, "prydwen");

            // Should be marked as active
            assert!(code.active);

            println!("Found Prydwen code: {} with rewards: {:?}", code.code, code.rewards);
        }
    }
}

#[tokio::test]
#[traced_test]
async fn test_fandom_fetch() {
    let config = get_test_config();
    let codes = fandom::fetch_codes(&config).await.unwrap();

    if !codes.is_empty() {
        for code in &codes {
            // Codes should be uppercase and contain only letters and numbers
            assert!(code.code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));

            // Each code should have at least one reward
            assert!(!code.rewards.is_empty(), "Code should have rewards: {}", code.code);

            // Source should be correct
            assert_eq!(code.source, "fandom");

            // Should be marked as active
            assert!(code.active);

            println!("Found Fandom code: {} with rewards: {:?}", code.code, code.rewards);
        }
    }
}

#[tokio::test]
#[traced_test]
async fn test_pcgamer_fetch() {
    let config = get_test_config();
    let codes = pcgamer::fetch_codes(&config).await.unwrap();

    if !codes.is_empty() {
        for code in &codes {
            // Codes should be uppercase and contain only letters and numbers
            assert!(code.code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));

            // Each code should have at least one reward
            assert!(!code.rewards.is_empty(), "Code should have rewards: {}", code.code);

            // Source should be correct
            assert_eq!(code.source, "pcgamer");

            // Should be marked as active
            assert!(code.active);

            println!("Found PCGamer code: {} with rewards: {:?}", code.code, code.rewards);
        }
    }
}