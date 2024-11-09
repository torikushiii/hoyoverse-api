use super::*;
use tokio;
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn test_game8_fetch() {
    let config = crate::config::Settings::new().unwrap();
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
async fn test_fetch_all_codes() {
    let config = crate::config::Settings::new().unwrap();
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
        
        // Source should be game8
        assert_eq!(code.source, "game8");
        
        // Should be marked as active
        assert!(code.active);
        
        // Verify no duplicate codes
        assert!(unique_codes.insert(&code.code), "Found duplicate code: {}", code.code);
        
        println!("Found code from {}: {} with rewards: {:?}", 
            code.source, code.code, code.rewards);
    }
    
    println!("Total unique codes found: {}", unique_codes.len());
} 