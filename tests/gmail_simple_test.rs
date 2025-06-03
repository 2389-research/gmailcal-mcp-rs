// A simple test to check the Gmail API token refresh
use mcp_gmailcal::config::Config;
use mcp_gmailcal::gmail_api::GmailService;
use mockito;
use serde_json::json;
use std::env;

// Create a mock config for testing
fn create_test_config() -> Config {
    Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        refresh_token: "test_refresh_token".to_string(),
        access_token: Some("test_access_token".to_string()),
        token_refresh_threshold: 300,
        token_expiry_buffer: 60,
    }
}

// Test the check_connection function
#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn test_check_connection() {
    // Create a mock server
    let mut server = mockito::Server::new();

    // Create a mock response
    let response = json!({
        "emailAddress": "test@example.com",
        "messagesTotal": 100,
        "threadsTotal": 50,
        "historyId": "12345"
    });

    // Mock the profile endpoint
    let mock = server
        .mock("GET", "/gmail/v1/users/me/profile")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response.to_string())
        .create();

    // Override the Gmail API URL to use our mock server
    env::set_var("GMAIL_API_BASE_URL", server.url());

    // Create a GmailService with our test config
    let mut gmail_service = GmailService::new(&create_test_config()).unwrap();

    // Call the function to test
    let result = gmail_service.check_connection().await;

    // Verify the mock was called
    mock.assert();

    // Check the result
    assert!(result.is_ok());
    let (email, count) = result.unwrap();
    assert_eq!(email, "test@example.com");
    assert_eq!(count, 100);
}
