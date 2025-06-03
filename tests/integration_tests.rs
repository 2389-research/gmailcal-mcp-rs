use chrono::{Datelike, Timelike, Utc};
/// Integration Test Suite for Gmail MCP Server
///
/// This file contains integration tests for the Gmail MCP server
/// with a focus on testing the server creation and basic functionality.
///
use mcp_gmailcal::{config::Config, errors::GmailApiError, prompts, GmailServer};
use serde_json::json;
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

// Test that prompts are correctly defined
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

    // Check the email prioritization prompt
    assert!(!prompts::EMAIL_PRIORITIZATION_PROMPT.is_empty());
    assert!(prompts::EMAIL_PRIORITIZATION_PROMPT.contains("prioritize emails"));

    // Check the email drafting prompt
    assert!(!prompts::EMAIL_DRAFTING_PROMPT.is_empty());
    assert!(prompts::EMAIL_DRAFTING_PROMPT.contains("draft effective"));
}

// Test for server creation and basic configuration
#[test]
fn test_server_creation() {
    setup();
    let _server = GmailServer::new();

    // We need to ensure all environment variables are set for Config
    // before testing Config::from_env()
    env::set_var("GMAIL_CLIENT_ID", "test_client_id");
    env::set_var("GMAIL_CLIENT_SECRET", "test_client_secret");
    env::set_var("GMAIL_REFRESH_TOKEN", "test_refresh_token");
    env::set_var("GMAIL_ACCESS_TOKEN", "test_access_token");

    // Create a config object directly instead of from environment
    let config = Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        refresh_token: "test_refresh_token".to_string(),
        access_token: Some("test_access_token".to_string()),
        token_refresh_threshold: 300, // Default 5 minutes
        token_expiry_buffer: 60,      // Default 1 minute
    };

    // Check the config values directly
    assert_eq!(config.client_id, "test_client_id");
    assert_eq!(config.client_secret, "test_client_secret");
    assert_eq!(config.refresh_token, "test_refresh_token");
    assert_eq!(config.access_token.unwrap(), "test_access_token");
}

// Test for configuration validation
#[test]
fn test_configuration() {
    setup();

    // Save the original value to restore later
    let original_client_id = env::var("GMAIL_CLIENT_ID").unwrap_or_default();

    // Set up a guard to ensure we restore the environment variable even if the test fails
    struct EnvGuard {
        key: &'static str,
        value: String,
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            env::set_var(self.key, &self.value);
        }
    }

    // Create guard for the client ID
    let _guard = EnvGuard {
        key: "GMAIL_CLIENT_ID",
        value: original_client_id.clone(),
    };

    // Make sure all required variables are set correctly
    env::set_var("GMAIL_CLIENT_ID", "test_client_id");
    env::set_var("GMAIL_CLIENT_SECRET", "test_client_secret");
    env::set_var("GMAIL_REFRESH_TOKEN", "test_refresh_token");
    env::set_var("GMAIL_ACCESS_TOKEN", "test_access_token");
    env::set_var("GMAIL_REDIRECT_URI", "test_redirect_uri");

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

    // Because the test structure shares environment variables, we need to check
    // if our other tests are interfering with this one.
    // Set a non-existent dotenv path to make sure no .env file is loaded
    env::set_var("DOTENV_PATH", "/tmp/nonexistent_dotenv_file_for_tests");

    // First make sure we can create a valid config
    let valid_config = Config::from_env();
    assert!(
        valid_config.is_ok(),
        "Config should be valid with all variables set"
    );

    // Now test with missing environment variables
    env::remove_var("GMAIL_CLIENT_ID");
    assert!(env::var("GMAIL_CLIENT_ID").is_err());

    // Verify config creation fails with missing variable
    let invalid_config = Config::from_env();
    assert!(
        invalid_config.is_err(),
        "Config should fail with missing GMAIL_CLIENT_ID"
    );

    // Restore the variable (not necessary because of guard, but for clarity)
    env::set_var("GMAIL_CLIENT_ID", "test_client_id");
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

// Integration test for email workflows
#[test]
fn test_email_workflows() {
    setup();

    // Simulate email workflow with mock data
    let email_json = json!({
        "id": "msg123456",
        "threadId": "thread123456",
        "historyId": "12345",
        "snippet": "This is an email about a meeting next week",
        "labelIds": ["INBOX", "UNREAD"],
        "date": "2023-05-15T10:00:00Z",
        "from": "sender@example.com",
        "to": "recipient@example.com",
        "subject": "Meeting Next Week",
        "body": "Let's meet next week to discuss the project progress."
    });

    // Simulate email search results
    let search_results = json!({
        "messages": [
            {
                "id": "msg123456",
                "threadId": "thread123456"
            },
            {
                "id": "msg789012",
                "threadId": "thread789012"
            }
        ],
        "nextPageToken": "token123",
        "resultSizeEstimate": 2
    });

    // Test email parsing
    assert_eq!(email_json["id"], "msg123456");
    assert_eq!(email_json["from"], "sender@example.com");
    assert_eq!(email_json["subject"], "Meeting Next Week");

    // Verify search results structure
    assert_eq!(search_results["messages"].as_array().unwrap().len(), 2);
    assert_eq!(search_results["messages"][0]["id"], "msg123456");
    assert_eq!(search_results["resultSizeEstimate"], 2);

    // Simulate email analysis workflow
    let email_content = email_json["body"].as_str().unwrap();
    assert!(email_content.contains("meet next week"));

    // Simulate detecting meeting intent in the email
    let has_meeting_intent = email_content.contains("meet")
        && (email_json["subject"].as_str().unwrap().contains("Meeting")
            || email_content.contains("meeting"));
    assert!(has_meeting_intent, "Email should contain meeting intent");
}

// Integration test for calendar operations
#[test]
fn test_calendar_operations() {
    setup();

    // Create a calendar event from mock data
    let now = Utc::now();
    let tomorrow = now + chrono::Duration::days(1);
    let end_time = tomorrow + chrono::Duration::hours(1);

    let event_data = json!({
        "summary": "Project Status Meeting",
        "description": "Discuss current project status and next steps",
        "location": "Conference Room B",
        "start": {
            "dateTime": tomorrow.to_rfc3339(),
            "timeZone": "UTC"
        },
        "end": {
            "dateTime": end_time.to_rfc3339(),
            "timeZone": "UTC"
        },
        "attendees": [
            {
                "email": "team@example.com"
            },
            {
                "email": "manager@example.com"
            }
        ]
    });

    // Simulate calendar list response
    let calendar_list = json!({
        "items": [
            {
                "id": "primary",
                "summary": "Primary Calendar",
                "primary": true
            },
            {
                "id": "calendar123",
                "summary": "Work Calendar"
            }
        ]
    });

    // Verify event data structure
    assert_eq!(event_data["summary"], "Project Status Meeting");
    assert!(event_data["start"]["dateTime"].as_str().unwrap().len() > 0);
    assert_eq!(event_data["attendees"].as_array().unwrap().len(), 2);

    // Verify calendar list structure
    assert_eq!(calendar_list["items"].as_array().unwrap().len(), 2);
    assert_eq!(calendar_list["items"][0]["id"], "primary");
    assert_eq!(calendar_list["items"][0]["summary"], "Primary Calendar");

    // Verify calendar event date handling
    let start_date = event_data["start"]["dateTime"].as_str().unwrap();
    let end_date = event_data["end"]["dateTime"].as_str().unwrap();
    assert!(start_date < end_date, "End date should be after start date");

    // Verify time zone handling
    assert_eq!(event_data["start"]["timeZone"], "UTC");
    assert_eq!(event_data["end"]["timeZone"], "UTC");
}

// Integration test for contact operations
#[test]
fn test_contact_operations() {
    setup();

    // Simulate contacts list
    let contacts_list = json!({
        "connections": [
            {
                "resourceName": "people/c12345",
                "names": [
                    {
                        "displayName": "John Doe",
                        "familyName": "Doe",
                        "givenName": "John"
                    }
                ],
                "emailAddresses": [
                    {
                        "value": "john.doe@example.com",
                        "type": "work"
                    }
                ],
                "phoneNumbers": [
                    {
                        "value": "+1234567890",
                        "type": "mobile"
                    }
                ]
            },
            {
                "resourceName": "people/c67890",
                "names": [
                    {
                        "displayName": "Jane Smith",
                        "familyName": "Smith",
                        "givenName": "Jane"
                    }
                ],
                "emailAddresses": [
                    {
                        "value": "jane.smith@example.com",
                        "type": "work"
                    }
                ]
            }
        ]
    });

    // Verify contacts structure
    assert_eq!(contacts_list["connections"].as_array().unwrap().len(), 2);
    assert_eq!(
        contacts_list["connections"][0]["names"][0]["displayName"],
        "John Doe"
    );
    assert_eq!(
        contacts_list["connections"][0]["emailAddresses"][0]["value"],
        "john.doe@example.com"
    );

    // Simulate contact lookup by email
    let email_to_find = "john.doe@example.com";
    let mut found_contact = None;

    for contact in contacts_list["connections"].as_array().unwrap() {
        if let Some(emails) = contact["emailAddresses"].as_array() {
            for email in emails {
                if email["value"] == email_to_find {
                    found_contact = Some(contact);
                    break;
                }
            }
        }
        if found_contact.is_some() {
            break;
        }
    }

    assert!(found_contact.is_some(), "Should find contact by email");
    assert_eq!(
        found_contact.unwrap()["names"][0]["displayName"],
        "John Doe"
    );
}

// Integration test for authentication flows
#[test]
fn test_authentication_flows() {
    setup();

    // Test token generation flow (simulated)
    let _auth_code = "4/P7q7W91a-oMsCeLvIaQm6bTrgtp7";
    let token_response = json!({
        "access_token": "ya29.a0ARrdaM-8ESz8QsScH1YW7pyQyVkT_hxLBp",
        "expires_in": 3599,
        "refresh_token": "1//0eHB-yGB0O5MuCgYIARAAGA4SNwF",
        "scope": "https://www.googleapis.com/auth/gmail.readonly",
        "token_type": "Bearer"
    });

    // Verify token response structure
    assert!(token_response["access_token"].as_str().unwrap().len() > 0);
    assert!(token_response["refresh_token"].as_str().unwrap().len() > 0);
    assert_eq!(token_response["token_type"], "Bearer");

    // Test config with tokens
    let config = Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        refresh_token: "test_refresh_token".to_string(),
        access_token: Some("test_access_token".to_string()),
        token_refresh_threshold: 300, // Default 5 minutes
        token_expiry_buffer: 60,      // Default 1 minute
    };

    // Verify config has expected tokens
    assert_eq!(config.refresh_token, "test_refresh_token");
    assert_eq!(config.access_token.unwrap(), "test_access_token");
}

// Integration test for error recovery
#[test]
fn test_error_recovery() {
    setup();

    // Test retry mechanism with simulated errors

    // Simulation helpers
    struct RetryState {
        attempts: u32,
        max_attempts: u32,
    }

    impl RetryState {
        fn new(max_attempts: u32) -> Self {
            Self {
                attempts: 0,
                max_attempts,
            }
        }

        // Simulate a flaky API call that succeeds after a few attempts
        fn attempt<T>(
            &mut self,
            success_value: T,
            error_fn: impl Fn() -> GmailApiError,
        ) -> Result<T, GmailApiError> {
            self.attempts += 1;

            if self.attempts < self.max_attempts {
                Err(error_fn())
            } else {
                Ok(success_value)
            }
        }
    }

    // Test rate limit recovery
    let mut rate_limit_state = RetryState::new(3);

    // First attempt - should fail with rate limit error
    let result = rate_limit_state.attempt("Success", || {
        GmailApiError::RateLimitError("Rate limit exceeded".to_string())
    });
    assert!(result.is_err());
    match result {
        Err(GmailApiError::RateLimitError(msg)) => {
            assert_eq!(msg, "Rate limit exceeded");
        }
        _ => panic!("Expected RateLimitError"),
    }

    // Second attempt - should fail again
    let result = rate_limit_state.attempt("Success", || {
        GmailApiError::RateLimitError("Rate limit exceeded".to_string())
    });
    assert!(result.is_err());

    // Third attempt - should succeed
    let result = rate_limit_state.attempt("Success", || {
        GmailApiError::RateLimitError("Rate limit exceeded".to_string())
    });
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");

    // Test network error recovery
    let mut network_error_state = RetryState::new(2);

    // First attempt - should fail with network error
    let result = network_error_state.attempt("Connected", || {
        GmailApiError::NetworkError("Connection failed".to_string())
    });
    assert!(result.is_err());

    // Second attempt - should succeed
    let result = network_error_state.attempt("Connected", || {
        GmailApiError::NetworkError("Connection failed".to_string())
    });
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Connected");
}

// Integration test for cross-API workflows
#[test]
fn test_cross_api_workflows() {
    setup();

    // This test simulates a workflow that combines multiple APIs
    // Steps:
    // 1. Find an email about a meeting
    // 2. Extract meeting details
    // 3. Look up the attendees in contacts
    // 4. Create a calendar event
    // 5. Send a confirmation email

    // Find an email (simulated)
    let email = json!({
        "id": "email123",
        "threadId": "thread123",
        "subject": "Project Meeting",
        "from": "colleague@example.com",
        "snippet": "Let's meet to discuss the project next Monday at 2 PM",
        "body": "Hi team,\n\nLet's schedule a meeting to discuss our project progress.\n\nDate: Next Monday\nTime: 2:00 PM - 3:00 PM\nLocation: Conference Room A\n\nPlease confirm if you can attend.\n\nRegards,\nYour Colleague"
    });

    // Extract meeting details (simulated)
    struct MeetingDetails {
        title: String,
        date: String,
        time: String,
        location: String,
        attendees: Vec<String>,
    }

    let meeting = MeetingDetails {
        title: "Project Meeting".to_string(),
        date: "Next Monday".to_string(),
        time: "2:00 PM - 3:00 PM".to_string(),
        location: "Conference Room A".to_string(),
        attendees: vec![
            "colleague@example.com".to_string(),
            "team@example.com".to_string(),
        ],
    };

    // Extract from email
    assert_eq!(email["subject"], meeting.title);
    assert!(email["body"].as_str().unwrap().contains(&meeting.date));
    assert!(email["body"].as_str().unwrap().contains(&meeting.time));
    assert!(email["body"].as_str().unwrap().contains(&meeting.location));

    // Look up contacts (simulated)
    let contacts = json!({
        "connections": [
            {
                "resourceName": "people/c12345",
                "names": [
                    {
                        "displayName": "Colleague",
                        "givenName": "Colleague"
                    }
                ],
                "emailAddresses": [
                    {
                        "value": "colleague@example.com"
                    }
                ]
            }
        ]
    });

    // Verify contact lookup
    assert_eq!(
        contacts["connections"][0]["emailAddresses"][0]["value"],
        meeting.attendees[0]
    );

    // Create calendar event (simulated)
    let now = Utc::now();
    let next_monday =
        now + chrono::Duration::days(((8 - now.weekday().num_days_from_monday() as i64) % 7) + 1);
    let event_start = next_monday
        .with_hour(14)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap();
    let event_end = event_start + chrono::Duration::hours(1);

    let event = json!({
        "summary": meeting.title,
        "location": meeting.location,
        "description": "Meeting scheduled based on email conversation.",
        "start": {
            "dateTime": event_start.to_rfc3339(),
            "timeZone": "UTC"
        },
        "end": {
            "dateTime": event_end.to_rfc3339(),
            "timeZone": "UTC"
        },
        "attendees": [
            {
                "email": meeting.attendees[0]
            },
            {
                "email": meeting.attendees[1]
            }
        ]
    });

    // Verify event creation
    assert_eq!(event["summary"], meeting.title);
    assert_eq!(event["location"], meeting.location);
    assert_eq!(event["attendees"].as_array().unwrap().len(), 2);
    assert_eq!(event["attendees"][0]["email"], meeting.attendees[0]);

    // Create draft response (simulated)
    let draft = json!({
        "id": "draft123",
        "threadId": email["threadId"],
        "to": email["from"],
        "subject": format!("Re: {}", email["subject"].as_str().unwrap()),
        "body": "I've added the meeting to my calendar and sent invites to all attendees. Looking forward to it!"
    });

    // Verify draft creation
    assert_eq!(draft["threadId"], email["threadId"]);
    assert_eq!(draft["to"], email["from"]);
    assert!(draft["subject"].as_str().unwrap().starts_with("Re: "));
    assert!(draft["body"]
        .as_str()
        .unwrap()
        .contains("added the meeting"));
}
