/// Config Module Tests
///
/// This file contains comprehensive tests for the config module,
/// focusing on environment variable handling, error cases, and
/// token expiry configuration.
///
use mcp_gmailcal::config::{Config, get_token_expiry_seconds};
use mcp_gmailcal::errors::ConfigError;
use std::env;

mod helper;
#[macro_use]
mod test_macros;

/// Helper function to create an environment guard and set required variables
fn setup_test_env() -> helper::EnvVarGuard {
    let mut guard = helper::EnvVarGuard::new();
    
    // Set required variables with test values
    guard.set("GMAIL_CLIENT_ID", "test_client_id");
    guard.set("GMAIL_CLIENT_SECRET", "test_client_secret");
    guard.set("GMAIL_REFRESH_TOKEN", "test_refresh_token");
    
    guard
}

/// Test successful Config creation from environment
#[test]
fn test_config_from_env_success() {
    // Setup test environment with clean slate
    env::remove_var("GMAIL_CLIENT_ID");
    env::remove_var("GMAIL_CLIENT_SECRET");
    env::remove_var("GMAIL_REFRESH_TOKEN");
    env::remove_var("GMAIL_ACCESS_TOKEN");
    
    let guard = setup_test_env();
    
    // Try to create config from environment
    let config = assert_ok!(Config::from_env());
    
    // Verify config values
    assert_eq!(config.client_id, "test_client_id");
    assert_eq!(config.client_secret, "test_client_secret");
    assert_eq!(config.refresh_token, "test_refresh_token");
    assert_eq!(config.access_token, None);
    
    // Keep guard in scope until end of test
    let _ = guard;
}

/// Test Config creation with optional access token
#[test]
fn test_config_with_access_token() {
    // Clear previous environment variables first
    env::remove_var("GMAIL_CLIENT_ID");
    env::remove_var("GMAIL_CLIENT_SECRET");
    env::remove_var("GMAIL_REFRESH_TOKEN");
    env::remove_var("GMAIL_ACCESS_TOKEN");
    
    // Setup test environment
    let mut guard = setup_test_env();
    
    // Set optional access token
    guard.set("GMAIL_ACCESS_TOKEN", "test_access_token");
    
    // Try to create config from environment
    let config = assert_ok!(Config::from_env());
    
    // Verify config values including access token
    assert_eq!(config.client_id, "test_client_id");
    assert_eq!(config.client_secret, "test_client_secret");
    assert_eq!(config.refresh_token, "test_refresh_token");
    assert_eq!(config.access_token, Some("test_access_token".to_string()));
    
    // Keep guard in scope until end of test
    let _ = guard;
}

/// Test error when missing client ID
#[test]
fn test_missing_client_id() {
    // Setup partial environment (missing client ID)
    // First clear all variables
    env::remove_var("GMAIL_CLIENT_ID");
    env::remove_var("GMAIL_CLIENT_SECRET");
    env::remove_var("GMAIL_REFRESH_TOKEN");
    env::remove_var("GMAIL_ACCESS_TOKEN");
    
    // Then set only the ones we want
    let mut guard = helper::EnvVarGuard::new();
    guard.set("GMAIL_CLIENT_SECRET", "test_client_secret");
    guard.set("GMAIL_REFRESH_TOKEN", "test_refresh_token");
    
    // Try to create config without client ID
    let result = Config::from_env();
    
    // Verify the error
    let err = assert_err!(result);
    match err {
        ConfigError::MissingEnvVar(var) => {
            assert_eq!(var, "GMAIL_CLIENT_ID");
        },
        _ => panic!("Expected MissingEnvVar error, got {:?}", err),
    }
    
    // Keep guard in scope until end of test
    let _ = guard;
}

/// Test error when missing client secret
#[test]
fn test_missing_client_secret() {
    // Setup partial environment (missing client secret)
    // First clear all variables
    env::remove_var("GMAIL_CLIENT_ID");
    env::remove_var("GMAIL_CLIENT_SECRET");
    env::remove_var("GMAIL_REFRESH_TOKEN");
    env::remove_var("GMAIL_ACCESS_TOKEN");
    
    // Then set only the ones we want
    let mut guard = helper::EnvVarGuard::new();
    guard.set("GMAIL_CLIENT_ID", "test_client_id");
    guard.set("GMAIL_REFRESH_TOKEN", "test_refresh_token");
    
    // Try to create config without client secret
    let result = Config::from_env();
    
    // Verify the error
    let err = assert_err!(result);
    match err {
        ConfigError::MissingEnvVar(var) => {
            assert_eq!(var, "GMAIL_CLIENT_SECRET");
        },
        _ => panic!("Expected MissingEnvVar error, got {:?}", err),
    }
    
    // Keep guard in scope until end of test
    let _ = guard;
}

/// Test error when missing refresh token
#[test]
fn test_missing_refresh_token() {
    // Setup partial environment (missing refresh token)
    // First clear all variables
    env::remove_var("GMAIL_CLIENT_ID");
    env::remove_var("GMAIL_CLIENT_SECRET");
    env::remove_var("GMAIL_REFRESH_TOKEN");
    env::remove_var("GMAIL_ACCESS_TOKEN");
    
    // Then set only the ones we want
    let mut guard = helper::EnvVarGuard::new();
    guard.set("GMAIL_CLIENT_ID", "test_client_id");
    guard.set("GMAIL_CLIENT_SECRET", "test_client_secret");
    
    // Try to create config without refresh token
    let result = Config::from_env();
    
    // Verify the error
    let err = assert_err!(result);
    match err {
        ConfigError::MissingEnvVar(var) => {
            assert_eq!(var, "GMAIL_REFRESH_TOKEN");
        },
        _ => panic!("Expected MissingEnvVar error, got {:?}", err),
    }
    
    // Keep guard in scope until end of test
    let _ = guard;
}

/// Test token expiry configuration
#[test]
fn test_token_expiry_seconds() {
    // Test default value
    env::remove_var("TOKEN_EXPIRY_SECONDS");
    assert_eq!(get_token_expiry_seconds(), 600); // Default is 10 minutes (600 seconds)
    
    // Test custom value
    env::set_var("TOKEN_EXPIRY_SECONDS", "300"); // 5 minutes
    assert_eq!(get_token_expiry_seconds(), 300);
    
    // Test invalid value (should return default)
    env::set_var("TOKEN_EXPIRY_SECONDS", "not_a_number");
    assert_eq!(get_token_expiry_seconds(), 600);
    
    // Reset environment
    env::remove_var("TOKEN_EXPIRY_SECONDS");
}

/// Test that API URL constants are defined correctly
#[test]
fn test_api_url_constants() {
    assert_eq!(mcp_gmailcal::config::GMAIL_API_BASE_URL, "https://gmail.googleapis.com/gmail/v1");
    assert_eq!(mcp_gmailcal::config::OAUTH_TOKEN_URL, "https://oauth2.googleapis.com/token");
}

/// Individual test for each environment configuration
#[test]
fn test_full_config() {
    // Clear all variables first
    env::remove_var("GMAIL_CLIENT_ID");
    env::remove_var("GMAIL_CLIENT_SECRET");
    env::remove_var("GMAIL_REFRESH_TOKEN");
    env::remove_var("GMAIL_ACCESS_TOKEN");
    
    // Setup with all variables including access token
    let mut guard = helper::EnvVarGuard::new();
    guard.set("GMAIL_CLIENT_ID", "client1");
    guard.set("GMAIL_CLIENT_SECRET", "secret1");
    guard.set("GMAIL_REFRESH_TOKEN", "refresh1");
    guard.set("GMAIL_ACCESS_TOKEN", "access1");
    
    // Test config creation
    let config = assert_ok!(Config::from_env());
    
    // Verify the values
    assert_eq!(config.client_id, "client1");
    assert_eq!(config.client_secret, "secret1");
    assert_eq!(config.refresh_token, "refresh1");
    assert_eq!(config.access_token, Some("access1".to_string()));
    
    // Keep guard in scope
    let _ = guard;
}

#[test]
fn test_minimum_config() {
    // Clear all variables first
    env::remove_var("GMAIL_CLIENT_ID");
    env::remove_var("GMAIL_CLIENT_SECRET");
    env::remove_var("GMAIL_REFRESH_TOKEN");
    env::remove_var("GMAIL_ACCESS_TOKEN");
    
    // Setup with minimum required variables
    let mut guard = helper::EnvVarGuard::new();
    guard.set("GMAIL_CLIENT_ID", "client2");
    guard.set("GMAIL_CLIENT_SECRET", "secret2");
    guard.set("GMAIL_REFRESH_TOKEN", "refresh2");
    
    // Test config creation
    let config = assert_ok!(Config::from_env());
    
    // Verify the values
    assert_eq!(config.client_id, "client2");
    assert_eq!(config.client_secret, "secret2");
    assert_eq!(config.refresh_token, "refresh2");
    assert_eq!(config.access_token, None);
    
    // Keep guard in scope
    let _ = guard;
}