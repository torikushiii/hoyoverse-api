use super::*;
use crate::config::Settings;
use tokio;
use tracing_test::traced_test;

fn get_test_config() -> Settings {
    Settings::new().expect("Failed to load test configuration")
}

#[tokio::test]
#[traced_test]
async fn test_fandom_fetch() {
    let config = get_test_config();
    let codes = fandom::fetch_codes(&config).await.unwrap();

    if !codes.is_empty() {
        for code in &codes {
            assert!(
                !code.rewards.is_empty(),
                "Code should have rewards: {}",
                code.code
            );
            assert_eq!(code.source, "fandom");

            println!(
                "Found Fandom code: {} with rewards: {:?}, active: {}",
                code.code, code.rewards, code.active
            );
        }
    }
}

#[tokio::test]
#[traced_test]
async fn test_fetch_codes() {
    let config = get_test_config();
    let codes = fetch_codes(&config).await.unwrap();

    let mut unique_codes = std::collections::HashSet::new();

    if !codes.is_empty() {
        for code in &codes {
            assert!(
                !code.rewards.is_empty(),
                "Code should have rewards: {}",
                code.code
            );
            assert_eq!(code.source, "fandom");

            assert!(
                unique_codes.insert(&code.code),
                "Found duplicate code: {}",
                code.code
            );

            println!(
                "Found code from {}: {} with rewards: {:?}, active: {}",
                code.source, code.code, code.rewards, code.active
            );
        }

        println!("Total unique codes found: {}", unique_codes.len());
    }
}
