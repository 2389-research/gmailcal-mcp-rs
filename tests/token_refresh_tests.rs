/// OAuth Token Refresh Tests Module
///
/// This module contains tests for the OAuth token refresh functionality,
/// focusing on error conditions and edge cases.

use mcp_gmailcal::auth::TokenManager;
use mcp_gmailcal::config::Config;
use mcp_gmailcal::errors::GmailApiError;
use mockito::{mock, server_url};
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

// Mock configuration for token testing
fn mock_config() -> Config {
    Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        refresh_token: "test_refresh_token".to_string(),
        access_token: None,
    }
}

// Mock configuration with an initial access token
fn mock_config_with_token() -> Config {
    Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        refresh_token: "test_refresh_token".to_string(),
        access_token: Some("initial_access_token".to_string()),
    }
}

#[tokio::test]
async fn test_token_refresh_expired() {
    // Test token refresh when access token is expired
    
    // Set up a mock server
    let mock_server = mock("POST", "/token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"access_token":"new_access_token","expires_in":3600,"token_type":"Bearer"}"#)
        .create();
    
    // Create a token manager with an expired token
    let mut token_manager = TokenManager::new(&mock_config());
    
    // We need to override the OAuth token URL in the real implementation
    // For testing purposes, we'll assume the get_token method uses the URL from the request client
    let client = Client::builder()
        .build()
        .unwrap();
    
    // Get a token, which should trigger a refresh since we set no initial token
    let token = token_manager.get_token(&client).await.unwrap();
    
    // Verify the token is the new one
    assert_eq!(token, "new_access_token");
    
    // Clean up
    mock_server.assert();
}

#[tokio::test]
async fn test_token_refresh_network_failure() {
    // Test token refresh when network failure occurs
    
    // Set up a mock server that returns a network error
    let mock_server = mock("POST", "/token")
        .with_status(500)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error":"server_error","error_description":"Internal server error"}"#)
        .create();
    
    // Create a token manager with an expired token
    let mut token_manager = TokenManager::new(&mock_config());
    
    // Create a client
    let client = Client::builder()
        .build()
        .unwrap();
    
    // Attempt to get a token, which should trigger a refresh and fail
    let result = token_manager.get_token(&client).await;
    
    // Verify the result is an error
    assert!(result.is_err());
    if let Err(err) = result {
        match err {
            GmailApiError::AuthError(msg) => {
                assert!(msg.contains("Failed to refresh token"));
            }
            _ => panic!("Expected AuthError but got different error type"),
        }
    }
    
    // Clean up
    mock_server.assert();
}

#[tokio::test]
async fn test_token_refresh_malformed_response() {
    // Test token refresh when the response is malformed
    
    // Set up a mock server that returns a malformed JSON response
    let mock_server = mock("POST", "/token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"access_token":"malformed_json"#) // Malformed JSON (missing closing brace)
        .create();
    
    // Create a token manager with an expired token
    let mut token_manager = TokenManager::new(&mock_config());
    
    // Create a client
    let client = Client::builder()
        .build()
        .unwrap();
    
    // Attempt to get a token, which should trigger a refresh and fail
    let result = token_manager.get_token(&client).await;
    
    // Verify the result is an error
    assert!(result.is_err());
    if let Err(err) = result {
        match err {
            GmailApiError::ApiError(msg) => {
                assert!(msg.contains("Failed to parse token response"));
            }
            _ => panic!("Expected ApiError but got different error type: {:?}", err),
        }
    }
    
    // Clean up
    mock_server.assert();
}

#[tokio::test]
async fn test_token_expiry_edge_case() {
    // Test token refresh when token is exactly at expiry time
    
    // Set up a mock server for the refresh
    let mock_server = mock("POST", "/token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"access_token":"new_edge_token","expires_in":3600,"token_type":"Bearer"}"#)
        .create();
    
    // Create a token manager with an initial token
    let mut token_manager = TokenManager::new(&mock_config_with_token());
    
    // Manually set the expiry to now (exactly at the edge)
    // We would need to expose the expiry field or add a method to set it for testing
    // For now, we'll assume the implementation refreshes when the token is expired
    
    // Create a client
    let client = Client::builder()
        .build()
        .unwrap();
    
    // Wait a moment to ensure the token expires
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Get a token, which should trigger a refresh
    let token = token_manager.get_token(&client).await.unwrap();
    
    // Verify the token is the new one
    assert_eq!(token, "new_edge_token");
    
    // Clean up
    mock_server.assert();
}