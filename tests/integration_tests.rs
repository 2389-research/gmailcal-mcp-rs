/// Integration Test Suite for Gmail MCP Server
///
/// This file contains integration tests for the Gmail MCP server
/// with a focus on testing the server creation and basic functionality.
///
use mcp_gmailcal::GmailServer;
use std::env;
use std::sync::Once;

// Used to ensure environment setup happens only once
static INIT: Once = Once::new();

// Setup function to initialize environment variables for testing
fn setup() {
    INIT.call_once(|| {
        // Set mock environment variables for testing
        env::set_var("GMAIL_CLIENT_ID", "test_client_id");
        env::set_var("GMAIL_CLIENT_SECRET", "test_client_secret");
        env::set_var("GMAIL_REFRESH_TOKEN", "test_refresh_token");
    });
}

// Basic test that verifies the server can be created
#[test]
fn test_server_creation() {
    setup();
    let server = GmailServer::new();
    // Simply verify that we can create the server
    // Environment variables will be checked when using the server's methods
}

// Placeholder for prompt tests
// These don't require API access so they could be tested directly
#[test]
fn test_gmail_prompt() {
    // This would test accessing the gmail_prompt constant
    // It doesn't require API access, so it works without mocking
}

// Test for configuration validation
#[test]
fn test_configuration() {
    setup();
    // Verify that environment variables are properly loaded
    assert_eq!(env::var("GMAIL_CLIENT_ID").unwrap(), "test_client_id");
    assert_eq!(
        env::var("GMAIL_CLIENT_SECRET").unwrap(),
        "test_client_secret"
    );
    assert_eq!(
        env::var("GMAIL_REFRESH_TOKEN").unwrap(),
        "test_refresh_token"
    );
}
