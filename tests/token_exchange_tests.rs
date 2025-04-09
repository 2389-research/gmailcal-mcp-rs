/// OAuth Token Exchange Tests
///
/// These tests cover the token exchange functionality of the OAuth module,
/// focusing on error handling.
use std::env;

// Import the oauth module for testing
use mcp_gmailcal::oauth;
use mcp_gmailcal::config::Config;

// Import the helper module
mod helper;
use helper::EnvVarGuard;

/// -------------------------
/// Test Helpers and Fixtures
/// -------------------------

// Setup function for tests with a clean environment
fn setup_clean_test_env() -> EnvVarGuard {
    let mut guard = EnvVarGuard::new();
    
    // Remove any existing environment variables that might affect the tests
    guard.remove("GMAIL_CLIENT_ID");
    guard.remove("GMAIL_CLIENT_SECRET");
    guard.remove("GMAIL_REFRESH_TOKEN");
    guard.remove("GMAIL_ACCESS_TOKEN");
    guard.remove("TOKEN_CACHE_ENABLED");
    guard.remove("TOKEN_REFRESH_THRESHOLD");
    guard.remove("TOKEN_EXPIRY_BUFFER");
    
    // Now set our test values
    guard.set("GMAIL_CLIENT_ID", "test-client-id");
    guard.set("GMAIL_CLIENT_SECRET", "test-client-secret");
    guard.set("GMAIL_REFRESH_TOKEN", "test-refresh-token");
    guard.set("GMAIL_ACCESS_TOKEN", "test-access-token");
    
    // Disable token caching for tests
    guard.set("TOKEN_CACHE_ENABLED", "false");
    
    // Set token refresh threshold and expiry buffer for predictable behavior
    guard.set("TOKEN_REFRESH_THRESHOLD", "300");
    guard.set("TOKEN_EXPIRY_BUFFER", "60");
    
    guard
}

/// -------------------------
/// Test Environment Setup
/// -------------------------

#[test]
fn test_environment_variables() {
    let _guard = setup_clean_test_env();
    
    // Verify environment variables are set
    assert_eq!(env::var("GMAIL_CLIENT_ID").unwrap(), "test-client-id");
    assert_eq!(env::var("GMAIL_CLIENT_SECRET").unwrap(), "test-client-secret");
    assert_eq!(env::var("GMAIL_REFRESH_TOKEN").unwrap(), "test-refresh-token");
    assert_eq!(env::var("GMAIL_ACCESS_TOKEN").unwrap(), "test-access-token");
    assert_eq!(env::var("TOKEN_CACHE_ENABLED").unwrap(), "false");
    assert_eq!(env::var("TOKEN_REFRESH_THRESHOLD").unwrap(), "300");
    assert_eq!(env::var("TOKEN_EXPIRY_BUFFER").unwrap(), "60");
}

/// -------------------------
/// Test Config Loading
/// -------------------------

#[test]
fn test_config_from_env() {
    // Create guard and set environment variables explicitly
    let mut guard = EnvVarGuard::new();
    
    // Set required environment variables for config
    guard.set("GMAIL_CLIENT_ID", "test-client-id");
    guard.set("GMAIL_CLIENT_SECRET", "test-client-secret");
    guard.set("GMAIL_REFRESH_TOKEN", "test-refresh-token");
    guard.set("GMAIL_ACCESS_TOKEN", "test-access-token");
    guard.set("TOKEN_CACHE_ENABLED", "false");
    guard.set("TOKEN_REFRESH_THRESHOLD", "300");
    guard.set("TOKEN_EXPIRY_BUFFER", "60");
    
    // Load config from environment
    let config = Config::from_env().expect("Failed to load config from env");
    
    // Verify config values
    assert_eq!(config.client_id, "test-client-id");
    assert_eq!(config.client_secret, "test-client-secret");
    assert_eq!(config.refresh_token, "test-refresh-token");
    assert_eq!(config.access_token.unwrap(), "test-access-token");
    assert_eq!(config.token_refresh_threshold, 300);
    assert_eq!(config.token_expiry_buffer, 60);
}

/// -------------------------
/// Test API Credential Testing
/// -------------------------

#[tokio::test]
async fn test_credentials_validation_error() {
    let mut guard = EnvVarGuard::new();
    
    // Remove any existing environment variables that might affect the test
    guard.remove("GMAIL_CLIENT_ID");
    guard.remove("GMAIL_CLIENT_SECRET");
    guard.remove("GMAIL_REFRESH_TOKEN");
    guard.remove("GMAIL_ACCESS_TOKEN");
    
    // Set invalid credentials
    guard.set("GMAIL_CLIENT_ID", "invalid-id");
    guard.set("GMAIL_CLIENT_SECRET", "invalid-secret");
    guard.set("GMAIL_REFRESH_TOKEN", "invalid-refresh-token");
    guard.set("GMAIL_ACCESS_TOKEN", "invalid-access-token");
    
    // Set a deliberately invalid API URL to ensure it fails
    guard.set("GMAIL_API_BASE_URL", "https://invalid.example.com");
    
    // Call the function to test credentials
    let result = oauth::test_credentials().await;
    
    // Verify the result is an error
    assert!(result.is_err(), "Credentials test should fail with invalid API URL");
}

#[tokio::test]
async fn test_credentials_with_missing_env_vars() {
    let mut guard = EnvVarGuard::new();
    
    // Make sure all relevant environment variables are removed
    guard.remove("GMAIL_CLIENT_ID");
    guard.remove("GMAIL_CLIENT_SECRET");
    guard.remove("GMAIL_REFRESH_TOKEN");
    guard.remove("GMAIL_ACCESS_TOKEN");
    guard.remove("GMAIL_API_BASE_URL");
    
    // Call the function to test credentials
    let result = oauth::test_credentials().await;
    
    // It should fail because we don't have the required environment variables
    assert!(result.is_err(), "Credentials test should fail with missing environment variables");
}

/// -------------------------
/// Testing OAuth Flow Error Cases
/// -------------------------

#[tokio::test]
#[ignore = "This test will attempt to open a browser and may hang"]
async fn test_run_oauth_flow_missing_env_vars() {
    let mut guard = EnvVarGuard::new();
    
    // Remove OAuth environment variables to force prompting
    guard.remove("GMAIL_CLIENT_ID");
    guard.remove("GMAIL_CLIENT_SECRET");
    
    // This would normally hang waiting for user input, so we'll ignore this test
    let result = oauth::run_oauth_flow().await;
    
    // In an interactive environment, this might succeed or fail depending on user input
    println!("OAuth flow result: {:?}", result);
}

#[tokio::test]
#[ignore = "This test will attempt to open a browser and may hang"]
async fn test_run_oauth_flow_with_invalid_credentials() {
    let mut guard = EnvVarGuard::new();
    
    // Remove any existing environment variables that might affect the test
    guard.remove("GMAIL_CLIENT_ID");
    guard.remove("GMAIL_CLIENT_SECRET");
    guard.remove("GMAIL_REFRESH_TOKEN");
    guard.remove("GMAIL_ACCESS_TOKEN");
    
    // Set invalid credentials to force an error
    guard.set("GMAIL_CLIENT_ID", "invalid-id");
    guard.set("GMAIL_CLIENT_SECRET", "invalid-secret");
    guard.set("GMAIL_REFRESH_TOKEN", "invalid-refresh-token");
    
    // We expect this to fail because we don't have valid credentials
    // and we're not actually running a browser flow
    let result = oauth::run_oauth_flow().await;
    assert!(result.is_err(), "OAuth flow should fail with invalid credentials");
}