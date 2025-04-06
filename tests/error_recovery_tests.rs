/// Error Recovery Tests Module
///
/// This module contains tests for error recovery, retry logic, and backoff strategies.

use mcp_gmailcal::errors::{GmailApiError, GmailResult};
use reqwest::StatusCode;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time;

// Mock HTTP client for testing
#[derive(Clone)]
struct MockHttpClient {
    status_codes: Vec<StatusCode>,
    call_count: Arc<Mutex<usize>>,
    delay_ms: u64,
}

impl MockHttpClient {
    fn new(status_codes: Vec<StatusCode>, delay_ms: u64) -> Self {
        Self {
            status_codes,
            call_count: Arc::new(Mutex::new(0)),
            delay_ms,
        }
    }

    async fn call(&self) -> Result<String, reqwest::Error> {
        // Simulate network delay
        if self.delay_ms > 0 {
            time::sleep(Duration::from_millis(self.delay_ms)).await;
        }

        // Get the current call count and increment it
        let mut count = self.call_count.lock().unwrap();
        let current_count = *count;
        *count += 1;

        // Determine the status code for this call
        let status_code = if current_count < self.status_codes.len() {
            self.status_codes[current_count]
        } else {
            // Default to success if we've gone beyond the defined sequence
            StatusCode::OK
        };

        // If status is in the 4xx or 5xx range, create an error
        if status_code.is_client_error() || status_code.is_server_error() {
            // Create a simple error with status code (limited by reqwest's API)
            let url = reqwest::Url::parse("https://example.com").unwrap();
            let req = http::Request::new(());
            let mut res = http::Response::builder()
                .status(status_code)
                .body(())
                .unwrap();
            
            return Err(reqwest::Error::decode(url, res));
        }

        // Return success response
        Ok("Success".to_string())
    }

    fn get_call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
}

// Implement retry logic
async fn retry_with_backoff<F, T, E>(
    mut operation: F,
    initial_delay_ms: u64,
    max_retries: usize,
    backoff_factor: f64,
) -> Result<T, E>
where
    F: FnMut() -> futures::future::BoxFuture<'static, Result<T, E>>,
{
    let mut delay_ms = initial_delay_ms;
    let mut attempt = 0;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(err);
                }

                // Wait with exponential backoff
                time::sleep(Duration::from_millis(delay_ms)).await;

                // Increase the delay for the next attempt
                delay_ms = (delay_ms as f64 * backoff_factor) as u64;
            }
        }
    }
}

// Test function that converts reqwest errors to GmailApiError
fn convert_error(err: reqwest::Error) -> GmailApiError {
    if let Some(status) = err.status() {
        if status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::SERVICE_UNAVAILABLE {
            return GmailApiError::RateLimitError(format!("Rate limit exceeded: {}", status));
        } else if status.is_client_error() {
            return GmailApiError::ApiError(format!("API error: {}", status));
        } else if status.is_server_error() {
            return GmailApiError::NetworkError(format!("Server error: {}", status));
        }
    }
    GmailApiError::NetworkError(format!("Network error: {}", err))
}

// Function to perform a request with retry logic
async fn perform_request_with_retry(client: &MockHttpClient) -> GmailResult<String> {
    let client_ref = client.clone();
    
    retry_with_backoff(
        move || {
            let client_clone = client_ref.clone();
            let fut = async move {
                match client_clone.call().await {
                    Ok(response) => Ok(response),
                    Err(err) => Err(convert_error(err)),
                }
            };
            Box::pin(fut)
        },
        100, // Initial delay of 100ms
        3,   // Max 3 retries
        2.0, // Double the delay each time
    )
    .await
}

#[cfg(test)]
mod error_recovery_tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_handling() {
        // Create a client that returns rate limit errors twice, then succeeds
        let client = MockHttpClient::new(
            vec![
                StatusCode::TOO_MANY_REQUESTS,
                StatusCode::TOO_MANY_REQUESTS,
                StatusCode::OK,
            ],
            10, // Small delay for testing
        );

        // Perform request with retry
        let start = Instant::now();
        let result = perform_request_with_retry(&client).await;
        let elapsed = start.elapsed();

        // Verify success after retries
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        
        // Verify correct number of calls were made
        assert_eq!(client.get_call_count(), 3);
        
        // Verify the backoff delay was applied (at least the minimum expected delay)
        // Initial delay: 100ms, Second attempt: 200ms, so minimum elapsed > 300ms
        assert!(elapsed.as_millis() >= 300);
    }

    #[tokio::test]
    async fn test_network_timeout_recovery() {
        // Create a client that times out twice, then succeeds
        let client = MockHttpClient::new(
            vec![StatusCode::REQUEST_TIMEOUT, StatusCode::REQUEST_TIMEOUT, StatusCode::OK],
            10, // Small delay for testing
        );

        // Perform request with retry
        let result = perform_request_with_retry(&client).await;

        // Verify success after retries
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        
        // Verify correct number of calls were made
        assert_eq!(client.get_call_count(), 3);
    }

    #[tokio::test]
    async fn test_server_error_recovery() {
        // Create a client that returns server errors, then succeeds
        let client = MockHttpClient::new(
            vec![StatusCode::INTERNAL_SERVER_ERROR, StatusCode::BAD_GATEWAY, StatusCode::OK],
            10, // Small delay for testing
        );

        // Perform request with retry
        let result = perform_request_with_retry(&client).await;

        // Verify success after retries
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        
        // Verify correct number of calls were made
        assert_eq!(client.get_call_count(), 3);
    }

    #[tokio::test]
    async fn test_max_retries_exceeded() {
        // Create a client that always returns errors
        let client = MockHttpClient::new(
            vec![
                StatusCode::INTERNAL_SERVER_ERROR,
                StatusCode::INTERNAL_SERVER_ERROR,
                StatusCode::INTERNAL_SERVER_ERROR,
                StatusCode::INTERNAL_SERVER_ERROR,
            ],
            10, // Small delay for testing
        );

        // Perform request with retry (max 3 retries)
        let result = perform_request_with_retry(&client).await;

        // Verify failure after max retries
        assert!(result.is_err());
        
        // Verify correct number of calls were made (initial + 3 retries = 4)
        assert_eq!(client.get_call_count(), 4);
    }

    #[tokio::test]
    async fn test_backoff_strategy() {
        // Create a client that requires all retries
        let client = MockHttpClient::new(
            vec![
                StatusCode::SERVICE_UNAVAILABLE,
                StatusCode::SERVICE_UNAVAILABLE,
                StatusCode::SERVICE_UNAVAILABLE,
                StatusCode::OK,
            ],
            0, // No artificial delay
        );

        // Track the time between each call
        let call_times = Arc::new(Mutex::new(Vec::<Instant>::new()));
        let call_times_clone = call_times.clone();
        
        // Create a custom operation with timing measurement
        let operation = move || {
            let client_ref = client.clone();
            let times_ref = call_times_clone.clone();
            
            let fut = async move {
                // Record call time
                {
                    let mut times = times_ref.lock().unwrap();
                    times.push(Instant::now());
                }
                
                // Make the client call
                match client_ref.call().await {
                    Ok(response) => Ok(response),
                    Err(err) => Err(convert_error(err)),
                }
            };
            Box::pin(fut)
        };
        
        // Perform retry with clear backoff parameters
        let result = retry_with_backoff(
            operation,
            100,  // Initial delay of 100ms
            5,    // Max 5 retries (more than we need)
            2.0,  // Double the delay each time
        )
        .await;
        
        // Get all call times
        let times = call_times.lock().unwrap();
        
        // Verify success after retries
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        
        // Verify correct number of calls were made
        assert_eq!(times.len(), 4);
        
        // Verify the backoff delay was applied with increasing intervals
        // Calculate the time between calls
        for i in 1..times.len() {
            let elapsed = times[i].duration_since(times[i-1]);
            
            // Expected delays: 0 (initial), ~100ms, ~200ms, ~400ms
            let expected_min_ms = if i == 1 { 80 } else if i == 2 { 180 } else { 380 };
            
            // Allow some tolerance for system scheduling
            assert!(elapsed.as_millis() >= expected_min_ms as u128);
        }
    }
}