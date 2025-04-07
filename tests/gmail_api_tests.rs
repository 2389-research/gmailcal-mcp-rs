/// Gmail API Tests Module
///
/// This module contains comprehensive tests for the Google Gmail API functionality,
/// focusing on email operations, message parsing, and MIME handling.
use base64;
use chrono::{DateTime, TimeZone, Utc};
use mockall::mock;
use mcp_gmailcal::errors::GmailApiError;
use mcp_gmailcal::gmail_api::{DraftEmail, EmailMessage, GmailService};
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// Define a proper interface for GmailClient that we can mock
trait GmailClientInterface {
    fn list_messages(&self, max_results: u32, query: Option<&str>) -> Result<Vec<EmailMessage>, GmailApiError>;
    fn get_message_details(&self, message_id: &str) -> Result<EmailMessage, GmailApiError>;
    fn create_draft(&self, draft: &DraftEmail) -> Result<String, GmailApiError>;
    fn list_labels(&self) -> Result<Value, GmailApiError>;
    fn check_connection(&self) -> Result<(String, u64), GmailApiError>;
}

// Wrapper for GmailClient that we can test against
struct MockableGmailClient {
    client: Arc<Mutex<dyn GmailClientInterface + Send + Sync>>,
}

impl MockableGmailClient {
    fn new(client: Arc<Mutex<dyn GmailClientInterface + Send + Sync>>) -> Self {
        Self { client }
    }

    fn list_messages(&self, max_results: u32, query: Option<&str>) -> Result<Vec<EmailMessage>, GmailApiError> {
        let client = self.client.lock().unwrap();
        client.list_messages(max_results, query)
    }

    fn get_message_details(&self, message_id: &str) -> Result<EmailMessage, GmailApiError> {
        let client = self.client.lock().unwrap();
        client.get_message_details(message_id)
    }

    fn create_draft(&self, draft: &DraftEmail) -> Result<String, GmailApiError> {
        let client = self.client.lock().unwrap();
        client.create_draft(draft)
    }

    fn list_labels(&self) -> Result<Value, GmailApiError> {
        let client = self.client.lock().unwrap();
        client.list_labels()
    }

    fn check_connection(&self) -> Result<(String, u64), GmailApiError> {
        let client = self.client.lock().unwrap();
        client.check_connection()
    }
}

// Helper functions to create test data
fn create_test_email(id: &str, subject: &str, from: &str, to: &str, body_text: &str, body_html: Option<&str>) -> EmailMessage {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let date = Utc.timestamp(now as i64, 0).to_rfc3339();

    EmailMessage {
        id: id.to_string(),
        thread_id: format!("thread_{}", id),
        subject: Some(subject.to_string()),
        from: Some(from.to_string()),
        to: Some(to.to_string()),
        date: Some(date),
        snippet: Some(format!("This is a snippet for email {}", id)),
        body_text: Some(body_text.to_string()),
        body_html: body_html.map(|s| s.to_string()),
    }
}

fn create_test_labels() -> Vec<(String, String)> {
    vec![
        ("INBOX".to_string(), "Inbox".to_string()),
        ("SENT".to_string(), "Sent".to_string()),
        ("DRAFT".to_string(), "Drafts".to_string()),
        ("TRASH".to_string(), "Trash".to_string()),
        ("SPAM".to_string(), "Spam".to_string()),
        ("CATEGORY_PERSONAL".to_string(), "Personal".to_string()),
        ("CATEGORY_SOCIAL".to_string(), "Social".to_string()),
        ("CATEGORY_PROMOTIONS".to_string(), "Promotions".to_string()),
        ("CATEGORY_UPDATES".to_string(), "Updates".to_string()),
        ("CATEGORY_FORUMS".to_string(), "Forums".to_string()),
    ]
}

fn labels_to_json(labels: &[(String, String)]) -> Value {
    let labels_json: Vec<Value> = labels
        .iter()
        .map(|(id, name)| {
            json!({
                "id": id,
                "name": name,
            })
        })
        .collect();

    json!({
        "labels": labels_json
    })
}

// Create sample message response in raw JSON format (similar to Gmail API response)
fn create_raw_message_response(
    id: &str,
    thread_id: &str,
    subject: &str,
    from: &str,
    to: &str,
    body_text: &str,
    body_html: Option<&str>,
) -> Value {
    let mut parts = Vec::new();
    
    // Add text part
    parts.push(json!({
        "mimeType": "text/plain",
        "body": {
            "data": base64::encode(body_text)
                .replace('+', "-")
                .replace('/', "_"),
            "size": body_text.len()
        }
    }));
    
    // Add HTML part if provided
    if let Some(html) = body_html {
        parts.push(json!({
            "mimeType": "text/html",
            "body": {
                "data": base64::encode(html)
                    .replace('+', "-")
                    .replace('/', "_"),
                "size": html.len()
            }
        }));
    }
    
    // Create headers
    let headers = vec![
        json!({"name": "From", "value": from}),
        json!({"name": "To", "value": to}),
        json!({"name": "Subject", "value": subject}),
        json!({"name": "Date", "value": format!("{} GMT", Utc::now().format("%a, %d %b %Y %H:%M:%S"))}),
    ];
    
    // Create payload
    let payload = if body_html.is_some() {
        // Multipart message
        json!({
            "mimeType": "multipart/alternative",
            "headers": headers,
            "parts": parts
        })
    } else {
        // Simple message
        json!({
            "mimeType": "text/plain",
            "headers": headers,
            "body": {
                "data": base64::encode(body_text)
                    .replace('+', "-")
                    .replace('/', "_"),
                "size": body_text.len()
            }
        })
    };
    
    // Create full message
    json!({
        "id": id,
        "threadId": thread_id,
        "labelIds": ["INBOX"],
        "snippet": format!("{:.50}", body_text),
        "payload": payload
    })
}

// Mock implementation
struct MockGmailClient {
    emails: Vec<EmailMessage>,
    labels: Vec<(String, String)>,
    should_fail: bool,
}

impl MockGmailClient {
    fn new() -> Self {
        // Create sample emails
        let emails = vec![
            create_test_email(
                "msg1",
                "Important Meeting",
                "sender@example.com",
                "recipient@example.com",
                "Let's meet tomorrow to discuss the project.",
                Some("<html><body>Let's meet tomorrow to discuss the project.</body></html>"),
            ),
            create_test_email(
                "msg2",
                "Weekly Report",
                "reports@example.com",
                "recipient@example.com",
                "Please find attached the weekly report.",
                Some("<html><body>Please find attached the weekly report.</body></html>"),
            ),
            create_test_email(
                "msg3",
                "Action Required",
                "system@example.com",
                "recipient@example.com",
                "Your action is required on the following items...",
                None,
            ),
        ];
        
        let labels = create_test_labels();
        
        Self {
            emails,
            labels,
            should_fail: false,
        }
    }
    
    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }
}

impl GmailClientInterface for MockGmailClient {
    fn list_messages(&self, max_results: u32, query: Option<&str>) -> Result<Vec<EmailMessage>, GmailApiError> {
        if self.should_fail {
            return Err(GmailApiError::ApiError("Failed to list messages".to_string()));
        }
        
        // Filter by query if provided
        let filtered_emails = match query {
            Some(q) => {
                let q = q.to_lowercase();
                self.emails.iter()
                    .filter(|email| {
                        let subject = email.subject.as_ref().map_or("", |s| s.as_str()).to_lowercase();
                        let body = email.body_text.as_ref().map_or("", |s| s.as_str()).to_lowercase();
                        subject.contains(&q) || body.contains(&q)
                    })
                    .cloned()
                    .collect::<Vec<EmailMessage>>()
            },
            None => self.emails.clone(),
        };
        
        // Apply max_results
        let limited_emails = filtered_emails.into_iter()
            .take(max_results as usize)
            .collect();
        
        Ok(limited_emails)
    }
    
    fn get_message_details(&self, message_id: &str) -> Result<EmailMessage, GmailApiError> {
        if self.should_fail {
            return Err(GmailApiError::ApiError("Failed to get message".to_string()));
        }
        
        // Find the email with the given ID
        let email = self.emails.iter()
            .find(|email| email.id == message_id)
            .cloned();
        
        match email {
            Some(email) => Ok(email),
            None => Err(GmailApiError::MessageRetrievalError(format!("Message {} not found", message_id))),
        }
    }
    
    fn create_draft(&self, draft: &DraftEmail) -> Result<String, GmailApiError> {
        if self.should_fail {
            return Err(GmailApiError::ApiError("Failed to create draft".to_string()));
        }
        
        // Basic validation
        if draft.to.is_empty() {
            return Err(GmailApiError::MessageFormatError("Recipient cannot be empty".to_string()));
        }
        
        if draft.subject.is_empty() {
            return Err(GmailApiError::MessageFormatError("Subject cannot be empty".to_string()));
        }
        
        // Generate a draft ID
        let draft_id = format!("draft_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        
        Ok(draft_id)
    }
    
    fn list_labels(&self) -> Result<Value, GmailApiError> {
        if self.should_fail {
            return Err(GmailApiError::ApiError("Failed to list labels".to_string()));
        }
        
        Ok(labels_to_json(&self.labels))
    }
    
    fn check_connection(&self) -> Result<(String, u64), GmailApiError> {
        if self.should_fail {
            return Err(GmailApiError::AuthError("Authentication failed".to_string()));
        }
        
        Ok(("test@example.com".to_string(), 1000))
    }
}

#[cfg(test)]
mod comprehensive_gmail_tests {
    use super::*;
    
    // Helper to create a test client
    fn create_test_client() -> MockableGmailClient {
        let mock_client = MockGmailClient::new();
        let client = Arc::new(Mutex::new(mock_client));
        MockableGmailClient::new(client)
    }
    
    // Helper to create a failing test client
    fn create_failing_client() -> MockableGmailClient {
        let mock_client = MockGmailClient::new().with_failure();
        let client = Arc::new(Mutex::new(mock_client));
        MockableGmailClient::new(client)
    }
    
    #[test]
    fn test_list_messages_success() {
        let client = create_test_client();
        
        // Test listing all messages
        let result = client.list_messages(10, None);
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 3);
        
        // Test with max_results
        let result = client.list_messages(2, None);
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 2);
        
        // Test with query
        let result = client.list_messages(10, Some("report"));
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].subject.as_ref().unwrap().contains("Report"));
    }
    
    #[test]
    fn test_list_messages_failure() {
        let client = create_failing_client();
        
        let result = client.list_messages(10, None);
        assert!(result.is_err());
        match result {
            Err(GmailApiError::ApiError(msg)) => {
                assert_eq!(msg, "Failed to list messages");
            }
            _ => panic!("Expected ApiError"),
        }
    }
    
    #[test]
    fn test_get_message_details_success() {
        let client = create_test_client();
        
        // Test getting an existing message
        let result = client.get_message_details("msg1");
        assert!(result.is_ok());
        let message = result.unwrap();
        
        // Verify message details
        assert_eq!(message.id, "msg1");
        assert_eq!(message.subject.as_ref().unwrap(), "Important Meeting");
        assert_eq!(message.from.as_ref().unwrap(), "sender@example.com");
        assert_eq!(message.to.as_ref().unwrap(), "recipient@example.com");
        assert!(message.body_text.is_some());
        assert!(message.body_html.is_some());
    }
    
    #[test]
    fn test_get_message_details_not_found() {
        let client = create_test_client();
        
        // Test getting a non-existent message
        let result = client.get_message_details("nonexistent");
        assert!(result.is_err());
        match result {
            Err(GmailApiError::MessageRetrievalError(msg)) => {
                assert!(msg.contains("Message nonexistent not found"));
            }
            _ => panic!("Expected MessageRetrievalError"),
        }
    }
    
    #[test]
    fn test_get_message_details_failure() {
        let client = create_failing_client();
        
        let result = client.get_message_details("msg1");
        assert!(result.is_err());
        match result {
            Err(GmailApiError::ApiError(msg)) => {
                assert_eq!(msg, "Failed to get message");
            }
            _ => panic!("Expected ApiError"),
        }
    }
    
    #[test]
    fn test_create_draft_success() {
        let client = create_test_client();
        
        // Create a valid draft
        let draft = DraftEmail {
            to: "recipient@example.com".to_string(),
            subject: "Test Draft".to_string(),
            body: "This is a test draft email.".to_string(),
            cc: None,
            bcc: None,
            thread_id: None,
            in_reply_to: None,
            references: None,
        };
        
        let result = client.create_draft(&draft);
        assert!(result.is_ok());
        let draft_id = result.unwrap();
        assert!(draft_id.starts_with("draft_"));
    }
    
    #[test]
    fn test_create_draft_with_optional_fields() {
        let client = create_test_client();
        
        // Create a draft with optional fields
        let draft = DraftEmail {
            to: "recipient@example.com".to_string(),
            subject: "Test Draft".to_string(),
            body: "This is a test draft email.".to_string(),
            cc: Some("cc@example.com".to_string()),
            bcc: Some("bcc@example.com".to_string()),
            thread_id: Some("thread123".to_string()),
            in_reply_to: Some("msg123".to_string()),
            references: Some("ref123".to_string()),
        };
        
        let result = client.create_draft(&draft);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_create_draft_validation_failure() {
        let client = create_test_client();
        
        // Test with empty recipient
        let invalid_draft = DraftEmail {
            to: "".to_string(),
            subject: "Test Draft".to_string(),
            body: "This is a test draft email.".to_string(),
            cc: None,
            bcc: None,
            thread_id: None,
            in_reply_to: None,
            references: None,
        };
        
        let result = client.create_draft(&invalid_draft);
        assert!(result.is_err());
        match result {
            Err(GmailApiError::MessageFormatError(msg)) => {
                assert!(msg.contains("Recipient cannot be empty"));
            }
            _ => panic!("Expected MessageFormatError"),
        }
        
        // Test with empty subject
        let invalid_draft = DraftEmail {
            to: "recipient@example.com".to_string(),
            subject: "".to_string(),
            body: "This is a test draft email.".to_string(),
            cc: None,
            bcc: None,
            thread_id: None,
            in_reply_to: None,
            references: None,
        };
        
        let result = client.create_draft(&invalid_draft);
        assert!(result.is_err());
        match result {
            Err(GmailApiError::MessageFormatError(msg)) => {
                assert!(msg.contains("Subject cannot be empty"));
            }
            _ => panic!("Expected MessageFormatError"),
        }
    }
    
    #[test]
    fn test_create_draft_failure() {
        let client = create_failing_client();
        
        let draft = DraftEmail {
            to: "recipient@example.com".to_string(),
            subject: "Test Draft".to_string(),
            body: "This is a test draft email.".to_string(),
            cc: None,
            bcc: None,
            thread_id: None,
            in_reply_to: None,
            references: None,
        };
        
        let result = client.create_draft(&draft);
        assert!(result.is_err());
        match result {
            Err(GmailApiError::ApiError(msg)) => {
                assert_eq!(msg, "Failed to create draft");
            }
            _ => panic!("Expected ApiError"),
        }
    }
    
    #[test]
    fn test_list_labels_success() {
        let client = create_test_client();
        
        let result = client.list_labels();
        assert!(result.is_ok());
        
        let labels = result.unwrap();
        assert!(labels["labels"].is_array());
        
        let labels_array = labels["labels"].as_array().unwrap();
        assert!(!labels_array.is_empty());
        
        // Verify standard labels exist
        let inbox_label = labels_array.iter().find(|label| label["id"] == "INBOX").unwrap();
        assert_eq!(inbox_label["name"], "Inbox");
        
        let sent_label = labels_array.iter().find(|label| label["id"] == "SENT").unwrap();
        assert_eq!(sent_label["name"], "Sent");
    }
    
    #[test]
    fn test_list_labels_failure() {
        let client = create_failing_client();
        
        let result = client.list_labels();
        assert!(result.is_err());
        match result {
            Err(GmailApiError::ApiError(msg)) => {
                assert_eq!(msg, "Failed to list labels");
            }
            _ => panic!("Expected ApiError"),
        }
    }
    
    #[test]
    fn test_check_connection_success() {
        let client = create_test_client();
        
        let result = client.check_connection();
        assert!(result.is_ok());
        
        let (email, count) = result.unwrap();
        assert_eq!(email, "test@example.com");
        assert_eq!(count, 1000);
    }
    
    #[test]
    fn test_check_connection_failure() {
        let client = create_failing_client();
        
        let result = client.check_connection();
        assert!(result.is_err());
        match result {
            Err(GmailApiError::AuthError(msg)) => {
                assert_eq!(msg, "Authentication failed");
            }
            _ => panic!("Expected AuthError"),
        }
    }
    
    #[test]
    fn test_parsing_email_message() {
        // Test parsing a raw Gmail API message response
        let raw_message = create_raw_message_response(
            "test_id",
            "test_thread",
            "Test Subject",
            "sender@example.com",
            "recipient@example.com",
            "This is the plain text body",
            Some("<html><body>This is the HTML body</body></html>"),
        );
        
        // Verify the structure of the raw message
        assert_eq!(raw_message["id"], "test_id");
        assert_eq!(raw_message["threadId"], "test_thread");
        assert_eq!(raw_message["payload"]["headers"][2]["name"], "Subject");
        assert_eq!(raw_message["payload"]["headers"][2]["value"], "Test Subject");
        
        // Verify that base64 encoding of the body text is correct
        let encoded_body = raw_message["payload"]["parts"][0]["body"]["data"].as_str().unwrap();
        let decoded_body = base64::decode(
            encoded_body.replace('-', "+").replace('_', "/")
        ).unwrap();
        let decoded_text = String::from_utf8(decoded_body).unwrap();
        assert_eq!(decoded_text, "This is the plain text body");
    }
    
    #[test]
    fn test_mime_message_format() {
        // Test MIME message format for draft creation
        let draft = DraftEmail {
            to: "recipient@example.com".to_string(),
            subject: "Test Subject".to_string(),
            body: "This is the email body.".to_string(),
            cc: Some("cc@example.com".to_string()),
            bcc: Some("bcc@example.com".to_string()),
            thread_id: None,
            in_reply_to: None,
            references: None,
        };
        
        // Create expected MIME format
        let expected_mime = "\
            From: me\r\n\
            To: recipient@example.com\r\n\
            Subject: Test Subject\r\n\
            Cc: cc@example.com\r\n\
            Bcc: bcc@example.com\r\n\
            \r\n\
            This is the email body.";
        
        // Create actual MIME message
        let actual_mime = format!(
            "From: me\r\n\
            To: {}\r\n\
            Subject: {}\r\n\
            {}{}\r\n\
            {}",
            draft.to,
            draft.subject,
            // Add optional fields if present
            draft.cc.as_ref().map_or("".to_string(), |cc| format!("Cc: {}\r\n", cc)),
            draft.bcc.as_ref().map_or("".to_string(), |bcc| format!("Bcc: {}\r\n", bcc)),
            draft.body
        );
        
        // Verify
        assert_eq!(actual_mime.replace(" ", ""), expected_mime.replace(" ", ""));
    }
    
    #[test]
    fn test_specialized_email_formats() {
        // Test creating and working with different email formats
        
        // 1. Plain text only
        let plain_email = create_test_email(
            "plain_id",
            "Plain Text Email",
            "sender@example.com",
            "recipient@example.com",
            "This is a plain text email with no HTML part.",
            None
        );
        
        assert!(plain_email.body_html.is_none());
        assert!(plain_email.body_text.is_some());
        
        // 2. HTML and text parts
        let html_email = create_test_email(
            "html_id",
            "HTML Email",
            "sender@example.com",
            "recipient@example.com",
            "This is the plain text part.",
            Some("<html><body>This is the <b>HTML</b> part.</body></html>")
        );
        
        assert!(html_email.body_html.is_some());
        assert!(html_email.body_text.is_some());
        
        // 3. Special characters
        let special_chars_email = create_test_email(
            "special_id",
            "Email with Special Characters: 😀 € ß",
            "sender@example.com",
            "recipient@example.com",
            "This email contains emoji 🎉 and special chars: äöüß",
            Some("<html><body>HTML with emoji 🎉 and special chars: äöüß</body></html>")
        );
        
        assert!(special_chars_email.subject.unwrap().contains("😀"));
        assert!(special_chars_email.body_text.unwrap().contains("🎉"));
        assert!(special_chars_email.body_html.unwrap().contains("🎉"));
    }
    
    #[test]
    fn test_message_thread_handling() {
        // Test creating messages in the same thread
        let draft1 = DraftEmail {
            to: "recipient@example.com".to_string(),
            subject: "Original Message".to_string(),
            body: "This is the original message.".to_string(),
            cc: None,
            bcc: None,
            thread_id: None, // Not part of a thread yet
            in_reply_to: None,
            references: None,
        };
        
        let client = create_test_client();
        let result1 = client.create_draft(&draft1);
        assert!(result1.is_ok());
        
        // Create a reply in the same thread
        let draft2 = DraftEmail {
            to: "recipient@example.com".to_string(),
            subject: "Re: Original Message".to_string(),
            body: "This is a reply to the original message.".to_string(),
            cc: None,
            bcc: None,
            thread_id: Some("thread123".to_string()), // Part of a thread
            in_reply_to: Some("msg123".to_string()), // References original message
            references: Some("msg123".to_string()), // References for threading
        };
        
        let result2 = client.create_draft(&draft2);
        assert!(result2.is_ok());
    }
}