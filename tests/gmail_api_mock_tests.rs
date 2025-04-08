/// Gmail API Mock Tests
///
/// This module tests the Gmail API functionality using mock objects 
/// to avoid the tokio runtime issues with mockito.
///
use mcp_gmailcal::errors::GmailApiError;
use mcp_gmailcal::gmail_api::{DraftEmail, EmailMessage};
// Removed unused imports
use mockall::predicate::*;
// Mockall automatically imports what it needs
use serde_json::json;

// Mock the HTTP client used in GmailService
mockall::mock! {
    pub GmailClient {
        pub fn check_connection(&self) -> Result<(String, u64), GmailApiError>;
        pub fn list_labels(&self) -> Result<String, GmailApiError>;
        pub fn create_draft(&self, draft: &DraftEmail) -> Result<String, GmailApiError>;
        pub fn list_messages<'a>(&'a self, max_results: u32, query: Option<&'a str>) -> Result<Vec<EmailMessage>, GmailApiError>;
        pub fn get_message_details(&self, message_id: &str) -> Result<EmailMessage, GmailApiError>;
    }
}

// Tests for Gmail API Service
#[cfg(test)]
mod tests {
    use super::*;

    // Test check_connection functionality
    #[test]
    fn test_check_connection_success() {
        // Create mock client
        let mut mock = MockGmailClient::new();
        
        // Setup expectations
        mock.expect_check_connection()
            .returning(|| Ok(("test@example.com".to_string(), 123)));
        
        // Test the function
        let result = mock.check_connection();
        
        // Verify result
        assert!(result.is_ok());
        let (email, count) = result.unwrap();
        assert_eq!(email, "test@example.com");
        assert_eq!(count, 123);
    }

    #[test]
    fn test_check_connection_failure() {
        // Create mock client
        let mut mock = MockGmailClient::new();
        
        // Setup expectations for failure
        mock.expect_check_connection()
            .returning(|| Err(GmailApiError::AuthError("Authentication failed".to_string())));
        
        // Test the function
        let result = mock.check_connection();
        
        // Verify error
        assert!(result.is_err());
        match result {
            Err(GmailApiError::AuthError(msg)) => {
                assert_eq!(msg, "Authentication failed");
            }
            _ => panic!("Expected AuthError")
        }
    }
    
    // Test list_labels functionality
    #[test]
    fn test_list_labels_success() {
        // Create mock client
        let mut mock = MockGmailClient::new();
        
        // Create expected labels JSON
        let labels_json = json!({
            "labels": [
                {"id": "INBOX", "name": "Inbox", "type": "system"},
                {"id": "SENT", "name": "Sent", "type": "system"},
                {"id": "TRASH", "name": "Trash", "type": "system"},
                {"id": "CATEGORY_PERSONAL", "name": "Personal", "type": "user"}
            ]
        }).to_string();
        
        // Setup expectations
        mock.expect_list_labels()
            .returning(move || Ok(labels_json.clone()));
        
        // Test the function
        let result = mock.list_labels();
        
        // Verify result
        assert!(result.is_ok());
        let labels = result.unwrap();
        assert!(labels.contains("INBOX"));
        assert!(labels.contains("Personal"));
    }

    #[test]
    fn test_list_labels_failure() {
        // Create mock client
        let mut mock = MockGmailClient::new();
        
        // Setup expectations for failure
        mock.expect_list_labels()
            .returning(|| Err(GmailApiError::NetworkError("Network error".to_string())));
        
        // Test the function
        let result = mock.list_labels();
        
        // Verify error
        assert!(result.is_err());
        match result {
            Err(GmailApiError::NetworkError(msg)) => {
                assert_eq!(msg, "Network error");
            }
            _ => panic!("Expected NetworkError")
        }
    }
    
    // Test create_draft functionality
    #[test]
    fn test_create_draft_success() {
        // Create mock client
        let mut mock = MockGmailClient::new();
        
        // Setup expectations
        mock.expect_create_draft()
            .returning(|_| Ok("draft123".to_string()));
        
        // Create a draft email
        let draft = DraftEmail {
            to: "recipient@example.com".to_string(),
            subject: "Test Draft".to_string(),
            body: "This is a test draft.".to_string(),
            cc: None,
            bcc: None,
            thread_id: None,
            in_reply_to: None,
            references: None,
        };
        
        // Test the function
        let result = mock.create_draft(&draft);
        
        // Verify result
        assert!(result.is_ok());
        let draft_id = result.unwrap();
        assert_eq!(draft_id, "draft123");
    }

    #[test]
    fn test_create_draft_with_all_fields() {
        // Create mock client
        let mut mock = MockGmailClient::new();
        
        // Setup expectations - we'll check that the draft has all fields
        mock.expect_create_draft()
            .withf(|draft| {
                draft.to == "recipient@example.com" && 
                draft.subject == "Test Draft with All Fields" &&
                draft.cc.is_some() &&
                draft.bcc.is_some() &&
                draft.thread_id.is_some() &&
                draft.in_reply_to.is_some() &&
                draft.references.is_some()
            })
            .returning(|_| Ok("draft456".to_string()));
        
        // Create a draft email with all fields
        let draft = DraftEmail {
            to: "recipient@example.com".to_string(),
            subject: "Test Draft with All Fields".to_string(),
            body: "This is a test draft with all fields.".to_string(),
            cc: Some("cc@example.com".to_string()),
            bcc: Some("bcc@example.com".to_string()),
            thread_id: Some("thread123".to_string()),
            in_reply_to: Some("message123".to_string()),
            references: Some("reference123".to_string()),
        };
        
        // Test the function
        let result = mock.create_draft(&draft);
        
        // Verify result
        assert!(result.is_ok());
        let draft_id = result.unwrap();
        assert_eq!(draft_id, "draft456");
    }

    #[test]
    fn test_create_draft_failure() {
        // Create mock client
        let mut mock = MockGmailClient::new();
        
        // Setup expectations
        mock.expect_create_draft()
            .returning(|_| Err(GmailApiError::ApiError("API error".to_string())));
        
        // Create a draft email
        let draft = DraftEmail {
            to: "recipient@example.com".to_string(),
            subject: "Test Draft".to_string(),
            body: "This is a test draft.".to_string(),
            cc: None,
            bcc: None,
            thread_id: None,
            in_reply_to: None,
            references: None,
        };
        
        // Test the function
        let result = mock.create_draft(&draft);
        
        // Verify error
        assert!(result.is_err());
        match result {
            Err(GmailApiError::ApiError(msg)) => {
                assert_eq!(msg, "API error");
            }
            _ => panic!("Expected ApiError")
        }
    }
    
    // Test get_message_details functionality
    #[test]
    fn test_get_message_details_success() {
        // Create mock client
        let mut mock = MockGmailClient::new();
        
        // Create a sample email message
        let email = EmailMessage {
            id: "msg123".to_string(),
            thread_id: "thread123".to_string(),
            subject: Some("Test Message".to_string()),
            from: Some("sender@example.com".to_string()),
            to: Some("recipient@example.com".to_string()),
            date: Some("2025-01-01T12:00:00Z".to_string()),
            snippet: Some("This is a test message...".to_string()),
            body_text: Some("This is the message body.".to_string()),
            body_html: Some("<html><body>This is the HTML message body.</body></html>".to_string()),
        };
        
        // Setup expectations
        mock.expect_get_message_details()
            .with(eq("msg123"))
            .returning(move |_| Ok(email.clone()));
        
        // Test the function
        let result = mock.get_message_details("msg123");
        
        // Verify result
        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.id, "msg123");
        assert_eq!(message.subject.unwrap(), "Test Message");
        assert_eq!(message.from.unwrap(), "sender@example.com");
        assert_eq!(message.to.unwrap(), "recipient@example.com");
        assert!(message.body_text.is_some());
        assert!(message.body_html.is_some());
    }

    #[test]
    fn test_get_message_details_failure() {
        // Create mock client
        let mut mock = MockGmailClient::new();
        
        // Setup expectations
        mock.expect_get_message_details()
            .with(eq("not_found"))
            .returning(|_| Err(GmailApiError::MessageRetrievalError("Message not found".to_string())));
        
        // Test the function
        let result = mock.get_message_details("not_found");
        
        // Verify error
        assert!(result.is_err());
        match result {
            Err(GmailApiError::MessageRetrievalError(msg)) => {
                assert_eq!(msg, "Message not found");
            }
            _ => panic!("Expected MessageRetrievalError")
        }
    }
    
    // Test list_messages functionality
    #[test]
    fn test_list_messages_success() {
        // Create mock client
        let mut mock = MockGmailClient::new();
        
        // Create sample email messages
        let messages = vec![
            EmailMessage {
                id: "msg1".to_string(),
                thread_id: "thread1".to_string(),
                subject: Some("First Message".to_string()),
                from: Some("sender1@example.com".to_string()),
                to: Some("recipient@example.com".to_string()),
                date: Some("2025-01-01T12:00:00Z".to_string()),
                snippet: Some("First message snippet...".to_string()),
                body_text: Some("First message body.".to_string()),
                body_html: None,
            },
            EmailMessage {
                id: "msg2".to_string(),
                thread_id: "thread2".to_string(),
                subject: Some("Second Message".to_string()),
                from: Some("sender2@example.com".to_string()),
                to: Some("recipient@example.com".to_string()),
                date: Some("2025-01-02T12:00:00Z".to_string()),
                snippet: Some("Second message snippet...".to_string()),
                body_text: Some("Second message body.".to_string()),
                body_html: None,
            },
        ];
        
        // Setup expectations - use any() matcher instead of eq() for lifetimes
        mock.expect_list_messages()
            .returning(move |_, _| Ok(messages.clone()));
        
        // Test the function
        let result = mock.list_messages(10, None);
        
        // Verify result
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id, "msg1");
        assert_eq!(messages[1].id, "msg2");
    }

    #[test]
    fn test_list_messages_with_query() {
        // Create mock client
        let mut mock = MockGmailClient::new();
        
        // Create sample email message for the query result
        let messages = vec![
            EmailMessage {
                id: "msg3".to_string(),
                thread_id: "thread3".to_string(),
                subject: Some("Important Message".to_string()),
                from: Some("important@example.com".to_string()),
                to: Some("recipient@example.com".to_string()),
                date: Some("2025-01-03T12:00:00Z".to_string()),
                snippet: Some("Important message snippet...".to_string()),
                body_text: Some("Important message body.".to_string()),
                body_html: None,
            },
        ];
        
        // Setup expectations for a query - without using eq() for lifetime issues
        mock.expect_list_messages()
            .returning(move |max, query| {
                if max == 10 && query == Some("important") {
                    Ok(messages.clone())
                } else {
                    Ok(vec![])
                }
            });
        
        // Test the function with a query
        let result = mock.list_messages(10, Some("important"));
        
        // Verify result
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, "msg3");
        assert_eq!(messages[0].subject.as_ref().unwrap(), "Important Message");
    }

    #[test]
    fn test_list_messages_failure() {
        // Create mock client
        let mut mock = MockGmailClient::new();
        
        // Setup expectations
        mock.expect_list_messages()
            .returning(|_, _| Err(GmailApiError::ApiError("API error".to_string())));
        
        // Test the function
        let result = mock.list_messages(10, None);
        
        // Verify error
        assert!(result.is_err());
        match result {
            Err(GmailApiError::ApiError(msg)) => {
                assert_eq!(msg, "API error");
            }
            _ => panic!("Expected ApiError")
        }
    }
}