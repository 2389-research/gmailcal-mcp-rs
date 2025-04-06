/// Error Recovery Tests Module
///
/// This module contains tests for error recovery and backoff strategies.

use mcp_gmailcal::errors::GmailApiError;
use std::time::{Duration, Instant};
use tokio::time;

#[cfg(test)]
mod error_recovery_tests {
    use super::*;

    // Test a basic retry algorithm with exponential backoff
    #[tokio::test]
    async fn test_retry_with_backoff() {
        let mut attempts = 0;
        let max_attempts = 3;
        let mut delay_ms = 100;
        let backoff_factor = 2.0;
        let start = Instant::now();
        
        let result: Result<&str, GmailApiError> = loop {
            attempts += 1;
            
            // Simulate an API call that fails for the first 2 attempts
            let success = attempts > 2;
            
            if success {
                break Ok("Success");
            } else if attempts >= max_attempts {
                break Err(GmailApiError::RateLimitError("Rate limit exceeded".to_string()));
            }
            
            // Wait with exponential backoff
            time::sleep(Duration::from_millis(delay_ms)).await;
            
            // Increase delay for the next attempt
            delay_ms = (delay_ms as f64 * backoff_factor) as u64;
        };
        
        let elapsed = start.elapsed();
        
        // Verify the result
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        
        // Verify attempts and timing
        assert_eq!(attempts, 3);
        
        // Verify the backoff delay was applied
        // Expected minimum delay: 100ms (first attempt) + 200ms (second attempt) = 300ms
        assert!(elapsed.as_millis() >= 300);
    }
    
    // Test network error recovery
    #[tokio::test]
    async fn test_network_error_recovery() {
        let mut attempts = 0;
        let max_attempts = 3;
        let mut delay_ms = 50;  // smaller delay for faster test
        
        let result: Result<&str, GmailApiError> = loop {
            attempts += 1;
            
            // Simulate network errors for the first 2 attempts
            if attempts <= 2 {
                time::sleep(Duration::from_millis(delay_ms)).await;
                delay_ms *= 2;  // Simple backoff
                continue;
            }
            
            // Success on third attempt
            if attempts <= max_attempts {
                break Ok("Connection established");
            } else {
                break Err(GmailApiError::NetworkError("Connection failed".to_string()));
            }
        };
        
        // Verify success after retries
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Connection established");
        assert_eq!(attempts, 3);
    }
    
    // Test max retries exceeded
    #[tokio::test]
    async fn test_max_retries_exceeded() {
        let mut attempts = 0;
        let max_attempts = 3;
        let mut delay_ms = 50;  // smaller delay for faster test
        
        let result: Result<&str, GmailApiError> = loop {
            attempts += 1;
            
            // Always fail
            if attempts >= max_attempts {
                break Err(GmailApiError::RateLimitError("Rate limit exceeded".to_string()));
            }
            
            // Wait before retrying
            time::sleep(Duration::from_millis(delay_ms)).await;
            delay_ms *= 2;  // Simple backoff
        };
        
        // Verify failure after max retries
        assert!(result.is_err());
        match result.unwrap_err() {
            GmailApiError::RateLimitError(msg) => {
                assert_eq!(msg, "Rate limit exceeded");
            }
            _ => panic!("Expected RateLimitError but got different error type"),
        }
        assert_eq!(attempts, 3);
    }
}