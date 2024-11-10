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
    let codes = sources::fandom::fetch_codes(&config).await.unwrap();
    
    assert!(!codes.is_empty(), "Should fetch at least one code from Fandom");
    
    for code in &codes {
        assert!(code.code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
        
        assert!(!code.rewards.is_empty(), "Code should have rewards: {}", code.code);
        
        assert_eq!(code.source, "fandom");
        
        assert!(code.active);
        
        println!("Found Fandom code: {} with rewards: {:?}", code.code, code.rewards);
    }
}

#[tokio::test]
#[traced_test]
async fn test_game8_fetch() {
    let config = get_test_config();
    let codes = sources::game8::fetch_codes(&config).await.unwrap();
    
    assert!(!codes.is_empty(), "Should fetch at least one code from Game8");
    
    for code in &codes {
        assert!(code.code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
        
        assert!(!code.rewards.is_empty(), "Code should have rewards: {}", code.code);
        
        assert_eq!(code.source, "game8");
        
        assert!(code.active);
        
        println!("Found Game8 code: {} with rewards: {:?}", code.code, code.rewards);
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
        
        assert!(["fandom", "game8"].contains(&code.source.as_str()));
        
        assert!(code.active);
        
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
            assert!(code.code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
            
            assert!(!code.rewards.is_empty(), "Code should have rewards: {}", code.code);
            
            assert_eq!(code.source, "hoyolab");
            
            assert!(code.active);
            
            println!("Found HoyoLab code: {} with rewards: {:?}", code.code, code.rewards);
        }
    }
} 