/// Error Recovery Tests Module
///
/// This module contains tests for error recovery and backoff strategies.
use mcp_gmailcal::errors::{CalendarApiError, GmailApiError, PeopleApiError};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::time;

#[cfg(test)]
mod error_recovery_tests {
    use super::*;

    // Simple retry counter
    struct RetryCounter {
        count: AtomicU32,
        max_attempts: u32,
    }

    impl RetryCounter {
        fn new(max_attempts: u32) -> Self {
            Self {
                count: AtomicU32::new(0),
                max_attempts,
            }
        }

        fn next_attempt(&self) -> u32 {
            self.count.fetch_add(1, Ordering::SeqCst) + 1
        }

        fn attempts(&self) -> u32 {
            self.count.load(Ordering::SeqCst)
        }

        async fn with_backoff<T, E, F>(
            &self,
            mut operation: F,
            initial_delay_ms: u64,
            backoff_factor: f64,
        ) -> Result<T, E>
        where
            F: FnMut(u32) -> Result<T, E>,
        {
            let mut delay_ms = initial_delay_ms;

            loop {
                let attempt = self.next_attempt();

                match operation(attempt) {
                    Ok(result) => return Ok(result),
                    Err(error) => {
                        if attempt >= self.max_attempts {
                            return Err(error);
                        }

                        // Wait before retrying
                        time::sleep(Duration::from_millis(delay_ms)).await;

                        // Increase delay for the next attempt with exponential backoff
                        delay_ms = (delay_ms as f64 * backoff_factor) as u64;
                    }
                }
            }
        }
    }

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
                break Err(GmailApiError::RateLimitError(
                    "Rate limit exceeded".to_string(),
                ));
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
        let mut delay_ms = 50; // smaller delay for faster test

        let result: Result<&str, GmailApiError> = loop {
            attempts += 1;

            // Simulate network errors for the first 2 attempts
            if attempts <= 2 {
                time::sleep(Duration::from_millis(delay_ms)).await;
                delay_ms *= 2; // Simple backoff
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
        let mut delay_ms = 50; // smaller delay for faster test

        let result: Result<&str, GmailApiError> = loop {
            attempts += 1;

            // Always fail
            if attempts >= max_attempts {
                break Err(GmailApiError::RateLimitError(
                    "Rate limit exceeded".to_string(),
                ));
            }

            // Wait before retrying
            time::sleep(Duration::from_millis(delay_ms)).await;
            delay_ms *= 2; // Simple backoff
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

    // Test the RetryCounter helper
    #[tokio::test]
    async fn test_retry_helper_function() {
        // Create a retry counter with max attempts of 5
        let counter = RetryCounter::new(5);

        // Function that will succeed on the 3rd attempt
        let result = counter
            .with_backoff(
                |attempt| {
                    if attempt >= 3 {
                        Ok("Operation succeeded")
                    } else {
                        Err(GmailApiError::NetworkError(format!(
                            "Failed attempt {}",
                            attempt
                        )))
                    }
                },
                50,  // initial delay (ms)
                1.5, // backoff factor
            )
            .await;

        // Verify success
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Operation succeeded");

        // Verify the operation was attempted exactly 3 times
        assert_eq!(counter.attempts(), 3);
    }

    // Test different error types
    #[tokio::test]
    async fn test_different_error_types() {
        // Create a retry counter with max attempts of 6
        let counter = RetryCounter::new(6);

        // Function that alternates between different error types
        let result = counter
            .with_backoff(
                |attempt| match attempt {
                    1 => Err(GmailApiError::NetworkError("Network timeout".to_string())),
                    2 => Err(GmailApiError::RateLimitError("Rate limited".to_string())),
                    3 => Err(GmailApiError::AuthError("Auth error".to_string())),
                    4 => Err(GmailApiError::ApiError("API error".to_string())),
                    _ => Ok("Operation succeeded"),
                },
                10,  // initial delay (ms)
                1.2, // backoff factor
            )
            .await;

        // Verify success
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Operation succeeded");

        // Verify the operation was attempted exactly 5 times
        assert_eq!(counter.attempts(), 5);
    }

    // Test retry for calendar API errors
    #[tokio::test]
    async fn test_calendar_api_retry() {
        // Create a retry counter with max attempts of 4
        let counter = RetryCounter::new(4);

        // Function that simulates a Calendar API operation
        let result = counter
            .with_backoff(
                |attempt| {
                    if attempt >= 3 {
                        Ok("Calendar operation succeeded")
                    } else {
                        Err(CalendarApiError::RateLimitError(format!(
                            "Calendar API rate limited, attempt {}",
                            attempt
                        )))
                    }
                },
                20,  // initial delay (ms)
                2.0, // backoff factor
            )
            .await;

        // Verify success
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Calendar operation succeeded");

        // Verify number of attempts
        assert_eq!(counter.attempts(), 3);
    }

    // Test retry for people API errors
    #[tokio::test]
    async fn test_people_api_retry() {
        // Create a retry counter with max attempts of 3
        let counter = RetryCounter::new(3);

        // Function that simulates a People API operation
        let result = counter
            .with_backoff(
                |attempt| {
                    if attempt >= 2 {
                        Ok("People API operation succeeded")
                    } else {
                        Err(PeopleApiError::NetworkError(format!(
                            "People API network error, attempt {}",
                            attempt
                        )))
                    }
                },
                30,  // initial delay (ms)
                1.5, // backoff factor
            )
            .await;

        // Verify success
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "People API operation succeeded");

        // Verify number of attempts
        assert_eq!(counter.attempts(), 2);
    }

    // Test hitting the maximum number of retries
    #[tokio::test]
    async fn test_max_retries_with_helper() {
        // Create a retry counter with max attempts of 3
        let counter = RetryCounter::new(3);

        // Function that always fails
        let result: Result<&str, GmailApiError> = counter
            .with_backoff(
                |attempt| {
                    Err(GmailApiError::ApiError(format!(
                        "Always failing, attempt {}",
                        attempt
                    )))
                },
                10,  // initial delay (ms)
                1.0, // backoff factor (no increase)
            )
            .await;

        // Verify the operation failed after max retries
        assert!(result.is_err());

        // Verify the operation was attempted exactly 3 times
        assert_eq!(counter.attempts(), 3);

        // Verify the final error was returned
        match result {
            Err(GmailApiError::ApiError(msg)) => {
                assert_eq!(msg, "Always failing, attempt 3");
            }
            _ => panic!("Expected ApiError"),
        }
    }
}
