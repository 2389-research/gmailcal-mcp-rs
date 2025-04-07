/// Config Module Tests
///
/// Tests for the config module functionality, focusing on
/// environment variable handling and error cases.
///
use mcp_gmailcal::config::{Config, get_token_expiry_seconds};
use mcp_gmailcal::errors::ConfigError;
use std::env;

/// Test that API URL constants are defined correctly
#[test]
fn test_api_url_constants() {
    assert_eq!(mcp_gmailcal::config::GMAIL_API_BASE_URL, "https://gmail.googleapis.com/gmail/v1");
    assert_eq!(mcp_gmailcal::config::OAUTH_TOKEN_URL, "https://oauth2.googleapis.com/token");
}

/// Test token expiry configuration
#[test]
fn test_token_expiry_seconds() {
    // Save original value if set
    let original = env::var("TOKEN_EXPIRY_SECONDS").ok();
    
    // Test default value
    env::remove_var("TOKEN_EXPIRY_SECONDS");
    assert_eq!(get_token_expiry_seconds(), 600); // Default is 10 minutes (600 seconds)
    
    // Test custom value
    env::set_var("TOKEN_EXPIRY_SECONDS", "300"); // 5 minutes
    assert_eq!(get_token_expiry_seconds(), 300);
    
    // Test invalid value (should return default)
    env::set_var("TOKEN_EXPIRY_SECONDS", "not_a_number");
    assert_eq!(get_token_expiry_seconds(), 600);
    
    // Restore original value if it existed
    match original {
        Some(val) => env::set_var("TOKEN_EXPIRY_SECONDS", val),
        None => env::remove_var("TOKEN_EXPIRY_SECONDS"),
    }
}

/// Test Config creation with direct instantiation
#[test]
fn test_config_direct_creation() {
    // Create a config directly using the struct
    let config = Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        refresh_token: "test_refresh_token".to_string(),
        access_token: None,
    };
    
    // Verify the values
    assert_eq!(config.client_id, "test_client_id");
    assert_eq!(config.client_secret, "test_client_secret");
    assert_eq!(config.refresh_token, "test_refresh_token");
    assert_eq!(config.access_token, None);
    
    // Create with access token
    let config_with_token = Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        refresh_token: "test_refresh_token".to_string(),
        access_token: Some("test_access_token".to_string()),
    };
    
    // Verify with access token
    assert_eq!(config_with_token.client_id, "test_client_id");
    assert_eq!(config_with_token.client_secret, "test_client_secret");
    assert_eq!(config_with_token.refresh_token, "test_refresh_token");
    assert_eq!(config_with_token.access_token, Some("test_access_token".to_string()));
}

/// Test error conversion in Config
#[test]
fn test_env_error_conversion() {
    // Verify that ConfigError properly implements conversion from env::VarError
    let var_error = env::VarError::NotPresent;
    let config_error = ConfigError::from(var_error);
    
    match config_error {
        ConfigError::EnvError(_) => {
            // Test passes if we get an EnvError
        },
        _ => panic!("Expected EnvError variant"),
    }
}