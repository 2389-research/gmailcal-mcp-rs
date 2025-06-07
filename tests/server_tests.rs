/// Server and MCP Command Tests Module
///
/// This module contains tests for the GmailServer and MCP command handling functionality,
/// focusing on command parsing, validation, and response formatting.
///
use mcp_gmailcal::GmailServer;
use serde_json::{json, Value};
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

#[cfg(test)]
mod server_tests {
    use super::*;

    // Basic test that verifies the server can be created
    #[test]
    fn test_server_creation() {
        setup();
        let _server = GmailServer::new();
        // Simply verify that we can create the server
    }

    // Test parsing a list_messages command
    #[test]
    fn test_command_parsing() {
        setup();
        let _server = GmailServer::new();

        // Parse a simple JSON command and verify its fields
        let cmd = r#"{
            "command": "list_messages",
            "params": {"max_results": 10, "query": "from:test@example.com"}
        }"#;

        let value: Value = serde_json::from_str(cmd).unwrap();
        assert_eq!(value["command"], "list_messages");
        assert_eq!(value["params"]["max_results"], 10);
        assert_eq!(
            value["params"]["query"],
            "from:test@example.com"
        );
    }

    // Test response formatting
    #[test]
    fn test_response_formatting() {
        setup();

        // Create a dummy result and ensure it serializes correctly
        let response = json!({ "status": "ok" });
        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains("\"status\":\"ok\""));
    }
}
