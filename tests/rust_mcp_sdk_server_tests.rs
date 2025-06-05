// ABOUTME: Tests for the new rust-mcp-sdk Gmail MCP server implementation
// ABOUTME: Verifies server instantiation, tool registration, and basic functionality

use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

// Test that the new server modules can be imported and instantiated
#[tokio::test]
async fn test_new_server_imports() {
    // Test that we can import the new modules
    use mcp_gmailcal::oauth_tools::OAuthState;

    // Test that OAuthState can be created
    let oauth_state = OAuthState {
        tokens: Arc::new(RwLock::new(None)),
    };

    // Verify the state is initialized correctly
    let tokens = oauth_state.tokens.read().await;
    assert!(tokens.is_none());
}

#[test]
fn test_oauth_tools_instantiation() {
    use mcp_gmailcal::oauth_tools::{AuthStatusTool, CompleteOAuthTool, GetOAuthUrlTool};

    // Test that OAuth tool structs can be created
    let get_url_tool = GetOAuthUrlTool {
        client_id: "test_client_id".to_string(),
    };
    assert_eq!(get_url_tool.client_id, "test_client_id");

    let complete_oauth_tool = CompleteOAuthTool {
        auth_code: "test_code".to_string(),
        client_id: "test_client_id".to_string(),
        client_secret: "test_secret".to_string(),
    };
    assert_eq!(complete_oauth_tool.auth_code, "test_code");

    let auth_status_tool = AuthStatusTool {};
    // AuthStatusTool should be instantiable (no fields to check)
    let _ = auth_status_tool;
}

#[test]
fn test_gmail_tools_instantiation() {
    use mcp_gmailcal::gmail_tools::{GetEmailTool, ListEmailsTool, SearchEmailsTool};

    // Test Gmail tool struct creation
    let list_emails_tool = ListEmailsTool {
        query: Some("test query".to_string()),
        max_results: Some(Value::Number(serde_json::Number::from(10))),
    };
    assert_eq!(list_emails_tool.query, Some("test query".to_string()));
    assert_eq!(
        list_emails_tool.max_results,
        Some(Value::Number(serde_json::Number::from(10)))
    );

    let get_email_tool = GetEmailTool {
        message_id: "test_message_id".to_string(),
    };
    assert_eq!(get_email_tool.message_id, "test_message_id");

    let search_emails_tool = SearchEmailsTool {
        query: "from:test@example.com".to_string(),
        max_results: Some(Value::Number(serde_json::Number::from(5))),
    };
    assert_eq!(search_emails_tool.query, "from:test@example.com");
}

#[test]
fn test_serde_serialization() {
    use mcp_gmailcal::oauth_tools::GetOAuthUrlTool;
    use serde_json;

    // Test that tools can be serialized/deserialized
    let tool = GetOAuthUrlTool {
        client_id: "test_client_id".to_string(),
    };

    let serialized = serde_json::to_string(&tool).expect("Should serialize");
    let deserialized: GetOAuthUrlTool =
        serde_json::from_str(&serialized).expect("Should deserialize");

    assert_eq!(tool.client_id, deserialized.client_id);
}

#[test]
fn test_json_schema_generation() {
    use mcp_gmailcal::oauth_tools::GetOAuthUrlTool;
    use schemars::schema_for;

    // Test that JSON schema can be generated for tools
    let schema = schema_for!(GetOAuthUrlTool);

    // Verify the schema has the expected structure
    assert!(schema.schema.object.is_some());

    let obj = schema.schema.object.as_ref().unwrap();
    assert!(obj.properties.contains_key("client_id"));
}
