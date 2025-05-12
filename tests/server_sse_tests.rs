use mcp_attr::Result as McpResult;
use mcp_gmailcal::server::GmailServer;
use serde_json::json;

// Test the send_custom_event API
#[tokio::test]
async fn test_send_custom_event_api() -> McpResult<()> {
    // Initialize server
    let server = GmailServer::new();

    // Call the MCP API to send a custom event
    let response = server
        .send_custom_event("test-api".to_string(), json!({"message": "test from API"}))
        .await?;

    // Verify response format
    let parsed: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(parsed["status"], "success");
    assert_eq!(parsed["event"]["type"], "test-api");

    Ok(())
}
