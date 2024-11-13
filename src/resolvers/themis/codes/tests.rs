use super::*;
use crate::config::Settings;
use tokio;
use tracing_test::traced_test;

fn get_test_config() -> Settings {
    Settings::new().expect("Failed to load test configuration")
}

#[tokio::test]
#[traced_test]
async fn test_totwiki_fetch() {
    let config = get_test_config();
    let codes = totwiki::fetch_codes(&config).await.unwrap();

    assert!(!codes.is_empty(), "Should fetch at least one code from tot.wiki");

    for code in &codes {
        assert!(code.code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));

        assert!(!code.rewards.is_empty(), "Code should have rewards: {}", code.code);

        assert_eq!(code.source, "totwiki");

        assert!(code.active);

        println!("Found TotWiki code: {} with rewards: {:?}", code.code, code.rewards);
    }
}

#[tokio::test]
#[traced_test]
async fn test_fetch_codes() {
    let config = get_test_config();
    let codes = fetch_codes(&config).await.unwrap();

    assert!(!codes.is_empty(), "Should fetch codes from at least one source");

    let mut unique_codes = std::collections::HashSet::new();

    for code in &codes {
        assert!(code.code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));

        assert!(!code.rewards.is_empty(), "Code should have rewards: {}", code.code);

        assert_eq!(code.source, "totwiki");

        assert!(code.active);

        assert!(unique_codes.insert(&code.code), "Found duplicate code: {}", code.code);

        println!("Found code from {}: {} with rewards: {:?}",
            code.source, code.code, code.rewards);
    }

    println!("Total unique codes found: {}", unique_codes.len());
}