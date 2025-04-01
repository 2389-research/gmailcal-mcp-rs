use mockall::mock;
/// Mock Gmail API Client for Unit Testing
///
/// This module provides mock implementations of the Gmail API client
/// for use in unit testing.
///
use mockall::predicate::*;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// Create a dummy email message structure for testing
#[derive(Debug, Clone)]
pub struct TestEmail {
    pub id: String,
    pub thread_id: String,
    pub subject: String,
    pub from: String,
    pub to: String,
    pub date: String,
    pub snippet: String,
    pub body_text: String,
    pub body_html: String,
}

impl TestEmail {
    pub fn new(id: &str, subject: &str, sender: &str) -> Self {
        TestEmail {
            id: id.to_string(),
            thread_id: format!("thread_{}", id),
            subject: subject.to_string(),
            from: sender.to_string(),
            to: "recipient@example.com".to_string(),
            date: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string(),
            snippet: format!("This is a snippet of email {}", id),
            body_text: format!("This is the plain text body of email {}. It contains some content that can be analyzed.", id),
            body_html: format!("<html><body><p>This is the HTML body of email {}.</p></body></html>", id),
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "threadId": self.thread_id,
            "subject": self.subject,
            "from": self.from,
            "to": self.to,
            "date": self.date,
            "snippet": self.snippet,
            "bodyText": self.body_text,
            "bodyHtml": self.body_html,
        })
    }
}

// Create a sample of test emails
pub fn create_test_emails() -> Vec<TestEmail> {
    vec![
        TestEmail::new("123", "Important meeting tomorrow", "boss@example.com"),
        TestEmail::new("456", "Project update", "colleague@example.com"),
        TestEmail::new("789", "Invoice #12345", "accounting@example.com"),
        TestEmail::new("101", "Weekly newsletter", "newsletter@example.com"),
        TestEmail::new(
            "112",
            "Action required: Task deadline",
            "project@example.com",
        ),
    ]
}

// Create sample labels
pub fn create_test_labels() -> Vec<(String, String)> {
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

// Convert labels to JSON format
pub fn labels_to_json(labels: &[(String, String)]) -> Value {
    let labels_json: Vec<Value> = labels
        .iter()
        .map(|(id, name)| {
            json!({
                "id": id,
                "name": name,
            })
        })
        .collect();

    json!(labels_json)
}

// Create mock Gmail API service
mock! {
    pub GmailClient {
        pub fn list_messages(&self, max_results: Option<u32>, query: Option<String>, label_ids: Option<Vec<String>>) -> Result<Value, String>;
        pub fn get_message(&self, id: &str) -> Result<Value, String>;
        pub fn list_labels(&self) -> Result<Value, String>;
        pub fn get_profile(&self) -> Result<Value, String>;
        pub fn create_draft(&self, to: String, subject: String, body: String, cc: Option<String>, bcc: Option<String>) -> Result<Value, String>;
    }
}

// Helper to create a pre-configured mock client
pub fn create_mock_client() -> MockGmailClient {
    let mut mock = MockGmailClient::new();

    // Setup for list_messages
    let test_emails = create_test_emails();
    let emails_json: Vec<Value> = test_emails.iter().map(|e| e.to_json()).collect();

    mock.expect_list_messages().returning(move |_, query, _| {
        // If query contains a filter, apply it
        let filtered_emails = if let Some(q) = query {
            let q = q.to_lowercase();
            emails_json
                .iter()
                .filter(|e| {
                    let subject = e["subject"].as_str().unwrap_or("").to_lowercase();
                    let snippet = e["snippet"].as_str().unwrap_or("").to_lowercase();
                    subject.contains(&q) || snippet.contains(&q)
                })
                .cloned()
                .collect::<Vec<Value>>()
        } else {
            emails_json.clone()
        };

        Ok(json!(filtered_emails))
    });

    // Setup for get_message
    mock.expect_get_message().returning(move |id| {
        // Find the email with the matching ID
        let email = test_emails.iter().find(|e| e.id == id).map(|e| e.to_json());

        match email {
            Some(email) => Ok(email),
            None => Err(format!("Email with ID {} not found", id)),
        }
    });

    // Setup for list_labels
    let test_labels = create_test_labels();
    mock.expect_list_labels()
        .returning(move || Ok(labels_to_json(&test_labels)));

    // Setup for get_profile
    mock.expect_get_profile().returning(|| {
        Ok(json!({
            "email": "test@example.com",
            "messagesTotal": 1253,
            "threadsTotal": 893,
            "historyId": "12345"
        }))
    });

    // Setup for create_draft
    mock.expect_create_draft()
        .returning(|to, subject, body, cc, bcc| {
            Ok(json!({
                "id": "draft_123456",
                "message": {
                    "id": "msg_123456",
                    "threadId": "thread_123456",
                    "to": to,
                    "subject": subject,
                    "body": body,
                    "cc": cc,
                    "bcc": bcc
                }
            }))
        });

    mock
}

// Create a type alias for a thread-safe mock client
pub type SharedMockClient = Arc<Mutex<MockGmailClient>>;
