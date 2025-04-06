/// Mock Enhancement Tests Module
///
/// This module contains tests for enhanced mock implementations,
/// focusing on delay simulation, error injection, and validation.
use mcp_gmailcal::errors::{GmailApiError, GmailResult};
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
struct EnhancedMock {
    // Configuration
    delay_ms: u64,
    error_rate: f64,

    // State tracking
    call_count: Arc<Mutex<usize>>,
    call_history: Arc<Mutex<Vec<String>>>,
}

impl EnhancedMock {
    fn new() -> Self {
        Self {
            delay_ms: 0,
            error_rate: 0.0,
            call_count: Arc::new(Mutex::new(0)),
            call_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    // Configure delay
    fn with_delay(&mut self, delay_ms: u64) -> &mut Self {
        self.delay_ms = delay_ms;
        self
    }

    // Configure error rate
    fn with_error_rate(&mut self, error_rate: f64) -> &mut Self {
        self.error_rate = error_rate.clamp(0.0, 1.0);
        self
    }

    // Record a call
    fn record_call(&self, method_name: &str) {
        let mut history = self.call_history.lock().unwrap();
        history.push(method_name.to_string());

        let mut count = self.call_count.lock().unwrap();
        *count += 1;
    }

    // Get call count
    fn get_call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }

    // Get call history
    fn get_call_history(&self) -> Vec<String> {
        self.call_history.lock().unwrap().clone()
    }

    // Clear history
    fn clear_history(&self) {
        let mut history = self.call_history.lock().unwrap();
        history.clear();

        let mut count = self.call_count.lock().unwrap();
        *count = 0;
    }

    // Simulated API call with potential delay and error
    async fn call(&self, method_name: &str) -> GmailResult<String> {
        self.record_call(method_name);

        // Simulate delay if configured
        if self.delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
        }

        // Simulate error based on error rate
        if self.error_rate > 0.0 {
            let random_val: f64 = rand::thread_rng().gen();
            if random_val < self.error_rate {
                return Err(GmailApiError::NetworkError(
                    "Simulated network error".to_string(),
                ));
            }
        }

        // Return success response
        Ok(format!("Response from {}", method_name))
    }
}

#[cfg(test)]
mod mock_enhancement_tests {
    use super::*;

    #[tokio::test]
    async fn test_configurable_delay() {
        // Create a mock with a delay
        let mut mock = EnhancedMock::new();
        mock.with_delay(200); // 200ms delay

        // Measure the call time
        let start = Instant::now();
        let result = mock.call("test_delay").await;
        let elapsed = start.elapsed();

        // Verify the delay was applied
        assert!(result.is_ok());
        assert!(
            elapsed.as_millis() >= 200,
            "Expected delay of at least 200ms, but got {}ms",
            elapsed.as_millis()
        );
    }

    #[tokio::test]
    async fn test_error_injection() {
        // Create a mock with a 100% error rate
        let mut mock = EnhancedMock::new();
        mock.with_error_rate(1.0);

        // Make a call
        let result = mock.call("test_error").await;

        // Verify the error
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                GmailApiError::NetworkError(msg) => {
                    assert_eq!(msg, "Simulated network error");
                }
                _ => panic!("Expected NetworkError but got a different error type"),
            }
        }
    }

    #[tokio::test]
    async fn test_partial_error_rate() {
        // Create a mock with a 50% error rate
        let mut mock = EnhancedMock::new();
        mock.with_error_rate(0.5);

        // Make multiple calls
        let mut _success_count = 0;
        let mut error_count = 0;
        let iterations = 100;

        for i in 0..iterations {
            let result = mock.call(&format!("test_partial_error_{}", i)).await;
            if result.is_ok() {
                _success_count += 1;
            } else {
                error_count += 1;
            }
        }

        // Verify the error rate is approximately 50%
        // Allow for some statistical variation (40-60%)
        assert!(
            error_count >= iterations * 4 / 10,
            "Error count too low: {}",
            error_count
        );
        assert!(
            error_count <= iterations * 6 / 10,
            "Error count too high: {}",
            error_count
        );
    }

    #[tokio::test]
    async fn test_call_validation() {
        // Create a mock
        let mock = EnhancedMock::new();

        // Clear any initialization history
        mock.clear_history();

        // Make some calls
        let _ = mock.call("method1").await;
        let _ = mock.call("method2").await;
        let _ = mock.call("method3").await;

        // Verify the call history
        let history = mock.get_call_history();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0], "method1");
        assert_eq!(history[1], "method2");
        assert_eq!(history[2], "method3");

        // Verify call count
        assert_eq!(mock.get_call_count(), 3);
    }

    #[tokio::test]
    async fn test_mock_response_customization() {
        // Create a mock
        let mock = EnhancedMock::new();

        // Make calls with different method names
        let response1 = mock.call("get_messages").await.unwrap();
        let response2 = mock.call("get_labels").await.unwrap();

        // Verify the responses are customized based on the method name
        assert_eq!(response1, "Response from get_messages");
        assert_eq!(response2, "Response from get_labels");
    }

    #[tokio::test]
    async fn test_mock_state_tracking() {
        // Create a mock
        let mock = EnhancedMock::new();

        // Define expected call sequence
        let expected_sequence = vec!["list_messages", "get_message", "list_labels"];

        // Make calls in the expected sequence
        for method in &expected_sequence {
            let _ = mock.call(method).await;
        }

        // Verify the call history matches the expected sequence
        let history = mock.get_call_history();
        assert_eq!(history, expected_sequence);

        // Clear history for the next test
        mock.clear_history();
        assert_eq!(mock.get_call_history().len(), 0);

        // Test a different sequence
        let _ = mock.call("get_message").await;
        let _ = mock.call("list_messages").await;

        // Verify the new call history
        let history = mock.get_call_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0], "get_message");
        assert_eq!(history[1], "list_messages");
    }
}
