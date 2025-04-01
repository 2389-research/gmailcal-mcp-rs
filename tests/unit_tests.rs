/// Unit Tests for Gmail MCP Server
///
/// These tests focus on testing individual components and methods
/// of the Gmail MCP server without requiring actual API access.
///
mod mock_client;

use mock_client::{create_mock_client, MockGmailClient, TestEmail};
use serde_json::json;
use std::env;
use std::sync::{Arc, Mutex, Once};

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

#[test]
fn test_mock_client_list_messages() {
    let client = create_mock_client();

    // Test listing all messages
    let result = client.list_messages(Some(10), None, None);
    assert!(result.is_ok(), "Failed to list messages");

    let messages = result.unwrap();
    let messages_arr = messages.as_array().unwrap();
    assert_eq!(messages_arr.len(), 5, "Should return 5 messages");

    // Test with a query filter
    let result = client.list_messages(Some(10), Some("meeting".to_string()), None);
    assert!(result.is_ok(), "Failed to list filtered messages");

    let messages = result.unwrap();
    let messages_arr = messages.as_array().unwrap();
    assert!(
        messages_arr.len() > 0,
        "Should return at least one message with 'meeting'"
    );

    // Check that the filtered message contains the query term
    let first_message = &messages_arr[0];
    let subject = first_message["subject"].as_str().unwrap();
    assert!(
        subject.to_lowercase().contains("meeting"),
        "Subject should contain 'meeting'"
    );
}

#[test]
fn test_mock_client_get_message() {
    let client = create_mock_client();

    // Test getting an existing message
    let result = client.get_message("123");
    assert!(result.is_ok(), "Failed to get message with ID 123");

    let message = result.unwrap();
    assert_eq!(
        message["id"].as_str().unwrap(),
        "123",
        "Message ID should be 123"
    );

    // Test getting a non-existent message
    let result = client.get_message("nonexistent");
    assert!(result.is_err(), "Should fail for non-existent message ID");
    assert!(
        result.unwrap_err().contains("not found"),
        "Error should indicate message not found"
    );
}

#[test]
fn test_mock_client_list_labels() {
    let client = create_mock_client();

    // Test listing labels
    let result = client.list_labels();
    assert!(result.is_ok(), "Failed to list labels");

    let labels = result.unwrap();
    let labels_arr = labels.as_array().unwrap();
    assert_eq!(labels_arr.len(), 10, "Should return 10 labels");

    // Check that the INBOX label exists
    let inbox_label = labels_arr.iter().find(|&l| l["id"] == "INBOX").unwrap();
    assert_eq!(
        inbox_label["name"].as_str().unwrap(),
        "Inbox",
        "INBOX label should be named 'Inbox'"
    );
}

#[test]
fn test_mock_client_get_profile() {
    let client = create_mock_client();

    // Test getting profile
    let result = client.get_profile();
    assert!(result.is_ok(), "Failed to get profile");

    let profile = result.unwrap();
    assert_eq!(
        profile["email"].as_str().unwrap(),
        "test@example.com",
        "Email should be test@example.com"
    );
    assert!(
        profile["messagesTotal"].as_u64().is_some(),
        "Should include messagesTotal"
    );
    assert!(
        profile["threadsTotal"].as_u64().is_some(),
        "Should include threadsTotal"
    );
}

#[test]
fn test_mock_client_create_draft() {
    let client = create_mock_client();

    // Test creating a draft
    let result = client.create_draft(
        "recipient@example.com".to_string(),
        "Test Subject".to_string(),
        "This is a test email body".to_string(),
        None,
        None,
    );
    assert!(result.is_ok(), "Failed to create draft");

    let draft = result.unwrap();
    assert!(draft["id"].as_str().is_some(), "Should include draft ID");
    assert!(
        draft["message"].is_object(),
        "Should include message object"
    );
    assert_eq!(
        draft["message"]["subject"].as_str().unwrap(),
        "Test Subject",
        "Draft subject should match"
    );
}

#[test]
fn test_email_conversion() {
    // Test the TestEmail to_json method
    let email = TestEmail::new("test_id", "Test Subject", "sender@example.com");
    let json = email.to_json();

    assert_eq!(json["id"].as_str().unwrap(), "test_id", "ID should match");
    assert_eq!(
        json["subject"].as_str().unwrap(),
        "Test Subject",
        "Subject should match"
    );
    assert_eq!(
        json["from"].as_str().unwrap(),
        "sender@example.com",
        "From should match"
    );
    assert!(
        json["bodyText"].as_str().is_some(),
        "Should include body text"
    );
    assert!(
        json["bodyHtml"].as_str().is_some(),
        "Should include HTML body"
    );
}
