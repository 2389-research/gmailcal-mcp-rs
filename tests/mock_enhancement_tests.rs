/// Mock Enhancement Tests Module
///
/// This module contains tests for enhanced mock implementations,
/// focusing on delay simulation, error injection, and validation.

use mcp_gmailcal::config::Config;
use mcp_gmailcal::errors::{GmailApiError, GmailResult};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time;

// Enhanced mock Gmail client
struct EnhancedMockGmailClient {
    // Configuration
    config: Arc<Config>,
    
    // Mock data
    messages: HashMap<String, serde_json::Value>,
    labels: Vec<serde_json::Value>,
    
    // Behavior configuration
    delay_ms: u64,
    error_rate: f64,
    error_type: Option<GmailApiError>,
    
    // State tracking
    call_history: Arc<Mutex<Vec<String>>>,
    call_count: Arc<Mutex<usize>>,
}

impl EnhancedMockGmailClient {
    fn new() -> Self {
        let config = Config {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            access_token: Some("test_access_token".to_string()),
        };

        Self {
            config: Arc::new(config),
            messages: HashMap::new(),
            labels: Vec::new(),
            delay_ms: 0,
            error_rate: 0.0,
            error_type: None,
            call_history: Arc::new(Mutex::new(Vec::new())),
            call_count: Arc::new(Mutex::new(0)),
        }
    }
    
    // Configure delay simulation
    fn with_delay(&mut self, delay_ms: u64) -> &mut Self {
        self.delay_ms = delay_ms;
        self
    }
    
    // Configure error injection
    fn with_error_rate(&mut self, error_rate: f64, error_type: GmailApiError) -> &mut Self {
        self.error_rate = error_rate.clamp(0.0, 1.0);
        self.error_type = Some(error_type);
        self
    }
    
    // Add a test message
    fn add_message(&mut self, id: &str, message: serde_json::Value) -> &mut Self {
        self.messages.insert(id.to_string(), message);
        self
    }
    
    // Add test labels
    fn add_labels(&mut self, labels: Vec<serde_json::Value>) -> &mut Self {
        self.labels = labels;
        self
    }
    
    // Get call history
    fn get_call_history(&self) -> Vec<String> {
        self.call_history.lock().unwrap().clone()
    }
    
    // Get call count
    fn get_call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
    
    // Clear call history
    fn clear_history(&self) {
        let mut history = self.call_history.lock().unwrap();
        history.clear();
        
        let mut count = self.call_count.lock().unwrap();
        *count = 0;
    }
    
    // Record a call
    fn record_call(&self, method_name: &str) {
        let mut history = self.call_history.lock().unwrap();
        history.push(method_name.to_string());
        
        let mut count = self.call_count.lock().unwrap();
        *count += 1;
    }
    
    // Simulate potential error based on configured error rate
    fn should_error(&self) -> bool {
        if self.error_rate > 0.0 && self.error_type.is_some() {
            let random_val: f64 = rand::random();
            return random_val < self.error_rate;
        }
        false
    }
    
    // Mock get_message method
    async fn get_message(&self, message_id: &str) -> GmailResult<serde_json::Value> {
        self.record_call(&format!("get_message:{}", message_id));
        
        // Simulate network delay
        if self.delay_ms > 0 {
            time::sleep(Duration::from_millis(self.delay_ms)).await;
        }
        
        // Simulate error if configured
        if self.should_error() {
            return Err(self.error_type.clone().unwrap());
        }
        
        // Return the message or an error if not found
        match self.messages.get(message_id) {
            Some(message) => Ok(message.clone()),
            None => Err(GmailApiError::MessageRetrievalError(format!(
                "Message not found: {}", message_id
            ))),
        }
    }
    
    // Mock list_messages method
    async fn list_messages(&self, query: Option<&str>) -> GmailResult<Vec<serde_json::Value>> {
        self.record_call(&format!("list_messages:{:?}", query));
        
        // Simulate network delay
        if self.delay_ms > 0 {
            time::sleep(Duration::from_millis(self.delay_ms)).await;
        }
        
        // Simulate error if configured
        if self.should_error() {
            return Err(self.error_type.clone().unwrap());
        }
        
        // Return all messages or filter by query
        let messages: Vec<serde_json::Value> = self.messages.values().cloned().collect();
        
        // If query is provided, we would filter messages here
        // This is a simplification - in a real implementation, we would apply the query
        Ok(messages)
    }
    
    // Mock list_labels method
    async fn list_labels(&self) -> GmailResult<Vec<serde_json::Value>> {
        self.record_call("list_labels");
        
        // Simulate network delay
        if self.delay_ms > 0 {
            time::sleep(Duration::from_millis(self.delay_ms)).await;
        }
        
        // Simulate error if configured
        if self.should_error() {
            return Err(self.error_type.clone().unwrap());
        }
        
        Ok(self.labels.clone())
    }
}

// Helper to create a test message
fn create_test_message(id: &str, subject: &str, body: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "threadId": format!("thread_{}", id),
        "labelIds": ["INBOX"],
        "snippet": body.chars().take(50).collect::<String>(),
        "payload": {
            "mimeType": "text/plain",
            "headers": [
                { "name": "From", "value": "sender@example.com" },
                { "name": "To", "value": "recipient@example.com" },
                { "name": "Subject", "value": subject },
                { "name": "Date", "value": "Mon, 15 Apr 2025 10:00:00 +0000" }
            ],
            "body": {
                "data": base64::engine::general_purpose::STANDARD.encode(body.as_bytes()),
                "size": body.len()
            }
        }
    })
}

#[cfg(test)]
mod mock_enhancement_tests {
    use super::*;
    use rand::Rng;

    #[tokio::test]
    async fn test_configurable_delay() {
        // Create a client with a significant delay
        let mut client = EnhancedMockGmailClient::new();
        client.with_delay(200); // 200ms delay
        
        // Add a test message
        let message = create_test_message("msg1", "Test Subject", "Test Body");
        client.add_message("msg1", message);
        
        // Measure the time it takes to get the message
        let start = Instant::now();
        let result = client.get_message("msg1").await;
        let elapsed = start.elapsed();
        
        // Verify the result and delay
        assert!(result.is_ok());
        assert!(elapsed.as_millis() >= 200, "Expected delay of at least 200ms, but got {}ms", elapsed.as_millis());
    }

    #[tokio::test]
    async fn test_error_injection() {
        // Create a client with a 100% error rate
        let mut client = EnhancedMockGmailClient::new();
        client.with_error_rate(
            1.0, 
            GmailApiError::NetworkError("Injected network error".to_string())
        );
        
        // Add a test message
        let message = create_test_message("msg1", "Test Subject", "Test Body");
        client.add_message("msg1", message);
        
        // Try to get the message, expect an error
        let result = client.get_message("msg1").await;
        
        // Verify the error
        assert!(result.is_err());
        match result.unwrap_err() {
            GmailApiError::NetworkError(msg) => {
                assert_eq!(msg, "Injected network error");
            }
            _ => panic!("Expected NetworkError but got different error type"),
        }
    }

    #[tokio::test]
    async fn test_partial_error_rate() {
        // Create a client with a 50% error rate
        let mut client = EnhancedMockGmailClient::new();
        client.with_error_rate(
            0.5, 
            GmailApiError::RateLimitError("Injected rate limit error".to_string())
        );
        
        // Add a test message
        let message = create_test_message("msg1", "Test Subject", "Test Body");
        client.add_message("msg1", message);
        
        // Try to get the message multiple times
        let mut success_count = 0;
        let mut error_count = 0;
        let iterations = 100;
        
        for _ in 0..iterations {
            let result = client.get_message("msg1").await;
            if result.is_ok() {
                success_count += 1;
            } else {
                error_count += 1;
            }
        }
        
        // Verify the error rate is approximately 50%
        // Allow for some statistical variation
        assert!(error_count > iterations * 3 / 10, "Error count too low: {}", error_count);
        assert!(error_count < iterations * 7 / 10, "Error count too high: {}", error_count);
    }

    #[tokio::test]
    async fn test_call_validation() {
        // Create a client
        let mut client = EnhancedMockGmailClient::new();
        
        // Add test messages and labels
        client.add_message("msg1", create_test_message("msg1", "First Message", "Message 1 Body"));
        client.add_message("msg2", create_test_message("msg2", "Second Message", "Message 2 Body"));
        client.add_labels(vec![
            serde_json::json!({"id": "INBOX", "name": "INBOX", "type": "system"}),
            serde_json::json!({"id": "STARRED", "name": "STARRED", "type": "system"}),
        ]);
        
        // Clear any initialization history
        client.clear_history();
        
        // Make some API calls
        let _ = client.list_messages(None).await;
        let _ = client.get_message("msg1").await;
        let _ = client.list_labels().await;
        let _ = client.get_message("msg2").await;
        
        // Verify the call history
        let history = client.get_call_history();
        assert_eq!(history.len(), 4);
        assert_eq!(history[0], "list_messages:None");
        assert_eq!(history[1], "get_message:msg1");
        assert_eq!(history[2], "list_labels");
        assert_eq!(history[3], "get_message:msg2");
        
        // Verify call count
        assert_eq!(client.get_call_count(), 4);
    }

    #[tokio::test]
    async fn test_mock_state_tracking() {
        // Create a client
        let mut client = EnhancedMockGmailClient::new();
        
        // Add a test message
        let message = create_test_message("msg1", "Test Subject", "Test Body");
        client.add_message("msg1", message);
        
        // Track state for complex testing scenario
        let mut expected_call_sequence = vec![
            "list_messages:None",
            "get_message:msg1",
            "list_labels",
        ];
        
        // Make API calls in the expected sequence
        let _ = client.list_messages(None).await;
        let _ = client.get_message("msg1").await;
        let _ = client.list_labels().await;
        
        // Verify the call history matches the expected sequence
        let history = client.get_call_history();
        assert_eq!(history, expected_call_sequence);
        
        // Clear history for next test
        client.clear_history();
        assert_eq!(client.get_call_history().len(), 0);
        
        // Test a different sequence
        let _ = client.get_message("msg1").await;
        let _ = client.list_messages(Some("is:unread")).await;
        
        expected_call_sequence = vec![
            "get_message:msg1",
            "list_messages:Some(\"is:unread\")",
        ];
        
        // Verify the new call history
        let history = client.get_call_history();
        assert_eq!(history, expected_call_sequence);
    }

    #[tokio::test]
    async fn test_mock_response_customization() {
        // Create a client
        let mut client = EnhancedMockGmailClient::new();
        
        // Add customized test messages
        let message1 = create_test_message("msg1", "Important Message", "This is important!");
        let message2 = create_test_message("msg2", "Unread Message", "This is unread");
        let message3 = serde_json::json!({
            "id": "msg3",
            "threadId": "thread_msg3",
            "labelIds": ["INBOX", "IMPORTANT", "STARRED"],
            "snippet": "Custom message with multiple labels",
            "payload": {
                "mimeType": "multipart/alternative",
                "headers": [
                    { "name": "From", "value": "vip@example.com" },
                    { "name": "To", "value": "recipient@example.com" },
                    { "name": "Subject", "value": "Custom Message" },
                    { "name": "Date", "value": "Mon, 15 Apr 2025 11:00:00 +0000" }
                ],
                "parts": [
                    {
                        "mimeType": "text/plain",
                        "body": {
                            "data": base64::engine::general_purpose::STANDARD.encode("Plain text version".as_bytes()),
                            "size": 18
                        }
                    },
                    {
                        "mimeType": "text/html",
                        "body": {
                            "data": base64::engine::general_purpose::STANDARD.encode("<html><body>HTML version</body></html>".as_bytes()),
                            "size": 36
                        }
                    }
                ]
            }
        });
        
        client.add_message("msg1", message1);
        client.add_message("msg2", message2);
        client.add_message("msg3", message3);
        
        // Test customized responses
        let result1 = client.get_message("msg1").await.unwrap();
        let result2 = client.get_message("msg2").await.unwrap();
        let result3 = client.get_message("msg3").await.unwrap();
        
        // Verify the customized messages
        assert_eq!(result1["payload"]["headers"][2]["value"], "Important Message");
        assert_eq!(result2["payload"]["headers"][2]["value"], "Unread Message");
        assert_eq!(result3["payload"]["headers"][2]["value"], "Custom Message");
        
        // Verify the complex message structure
        assert_eq!(result3["labelIds"].as_array().unwrap().len(), 3);
        assert!(result3["payload"]["parts"].as_array().unwrap().len() > 1);
    }
}