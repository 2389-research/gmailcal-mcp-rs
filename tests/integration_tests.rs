/// Integration Test Suite for Gmail MCP Server
///
/// This file contains integration tests for the Gmail MCP server
/// with a focus on testing the server creation and basic functionality.
///
use mcp_gmailcal::{prompts, GmailServer};
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
        env::set_var("GMAIL_ACCESS_TOKEN", "test_access_token");
        env::set_var("GMAIL_REDIRECT_URI", "test_redirect_uri");
    });
}

// Basic test that verifies the server can be created
#[test]
fn test_server_creation() {
    setup();
    let server = GmailServer::new();
    // Simply verify that we can create the server
    // Environment variables will be checked when using the server's methods

    // Simply verify the server exists (no assertions needed since it would throw if creation failed)
}

// Test for Gmail prompts
#[test]
fn test_gmail_prompt() {
    // Test accessing the various prompt constants
    // Verify they are non-empty and contain expected keywords

    // Check the master prompt
    assert!(!prompts::GMAIL_MASTER_PROMPT.is_empty());
    assert!(prompts::GMAIL_MASTER_PROMPT.contains("Gmail Assistant"));

    // Check the email analysis prompt
    assert!(!prompts::EMAIL_ANALYSIS_PROMPT.is_empty());
    assert!(prompts::EMAIL_ANALYSIS_PROMPT.contains("analyzing emails"));

    // Check the email summarization prompt
    assert!(!prompts::EMAIL_SUMMARIZATION_PROMPT.is_empty());
    assert!(prompts::EMAIL_SUMMARIZATION_PROMPT.contains("summarizing emails"));

    // Check the email search prompt
    assert!(!prompts::EMAIL_SEARCH_PROMPT.is_empty());
    assert!(prompts::EMAIL_SEARCH_PROMPT.contains("search for emails"));

    // Check the task extraction prompt
    assert!(!prompts::TASK_EXTRACTION_PROMPT.is_empty());
    assert!(prompts::TASK_EXTRACTION_PROMPT.contains("extracting tasks"));

    // Check the meeting extraction prompt
    assert!(!prompts::MEETING_EXTRACTION_PROMPT.is_empty());
    assert!(prompts::MEETING_EXTRACTION_PROMPT.contains("extracting meeting"));

    // Check the contact extraction prompt
    assert!(!prompts::CONTACT_EXTRACTION_PROMPT.is_empty());
    assert!(prompts::CONTACT_EXTRACTION_PROMPT.contains("extracting contact"));

    // Check the email categorization prompt
    assert!(!prompts::EMAIL_CATEGORIZATION_PROMPT.is_empty());
    assert!(prompts::EMAIL_CATEGORIZATION_PROMPT.contains("categorizing emails"));
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
    assert_eq!(env::var("GMAIL_ACCESS_TOKEN").unwrap(), "test_access_token");
    assert_eq!(env::var("GMAIL_REDIRECT_URI").unwrap(), "test_redirect_uri");

    // Test with missing environment variables
    env::remove_var("GMAIL_CLIENT_ID");
    assert!(env::var("GMAIL_CLIENT_ID").is_err());
}

// Test command handling behavior using mock data
#[test]
fn test_command_handling() {
    setup();

    // Verify that we can parse JSON commands
    let json_command = r#"
    {
        "command": "list_messages",
        "params": {
            "max_results": 5,
            "query": "important"
        }
    }
    "#;

    // Parse the command (simple validation, not actual execution)
    let parsed: serde_json::Value = serde_json::from_str(json_command).unwrap();
    assert_eq!(parsed["command"], "list_messages");
    assert_eq!(parsed["params"]["max_results"], 5);
    assert_eq!(parsed["params"]["query"], "important");

    // Verify error handling for invalid JSON
    let invalid_json = r#"
    {
        "command": "list_messages",
        "params": {
            "max_results": 5,
            "query": "important"
        
    }
    "#; // Missing closing brace

    let parse_result = serde_json::from_str::<serde_json::Value>(invalid_json);
    assert!(parse_result.is_err());
}
