/// Test Macros Example
///
/// This file demonstrates how to use the helper macros
/// defined in test_macros.rs.
///
mod helper;
#[macro_use]
mod test_macros;

use std::error::Error;
use std::fmt;

// Define a custom error type for demonstration
#[derive(Debug)]
enum TestError {
    NotFound(String),
    InvalidInput(String),
    NetworkError(String),
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestError::NotFound(msg) => write!(f, "Not found: {}", msg),
            TestError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            TestError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl Error for TestError {}

// Example functions to test
fn divide(a: i32, b: i32) -> Result<i32, TestError> {
    if b == 0 {
        return Err(TestError::InvalidInput("Cannot divide by zero".to_string()));
    }
    Ok(a / b)
}

fn find_user(id: &str) -> Result<String, TestError> {
    match id {
        "1" => Ok("Alice".to_string()),
        "2" => Ok("Bob".to_string()),
        _ => Err(TestError::NotFound(format!("User {} not found", id))),
    }
}

async fn async_operation(succeed: bool) -> Result<String, TestError> {
    if succeed {
        Ok("Operation succeeded".to_string())
    } else {
        Err(TestError::NetworkError("Connection failed".to_string()))
    }
}

fn is_even(n: i32) -> bool {
    n % 2 == 0
}

fn format_time(time_provider: &helper::MockTimeProvider) -> String {
    use chrono::{DateTime, Utc};
    let system_time = time_provider.now();
    let datetime: DateTime<Utc> = system_time.into();
    datetime.format("%Y-%m-%d").to_string()
}

fn get_api_key() -> String {
    std::env::var("API_KEY").unwrap_or_else(|_| "default_key".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_ok_macro() {
        // Using assert_ok to test a function that should succeed
        let result = assert_ok!(divide(10, 2));
        assert_eq!(result, 5);

        // Using assert_ok with a custom message
        let user = assert_ok!(find_user("1"), "Failed to find user");
        assert_eq!(user, "Alice");
    }

    #[test]
    fn test_assert_err_macro() {
        // Using assert_err to test a function that should fail
        let error = assert_err!(divide(10, 0));
        match error {
            TestError::InvalidInput(msg) => assert_eq!(msg, "Cannot divide by zero"),
            _ => panic!("Expected InvalidInput error"),
        }

        // Using assert_err with a custom message
        let error = assert_err!(find_user("999"), "Expected user not found error");
        match error {
            TestError::NotFound(_) => (),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_assert_err_variant_macro() {
        // Using assert_err_variant to test error type
        assert_err_variant!(divide(10, 0), TestError::InvalidInput(_));
        assert_err_variant!(find_user("999"), TestError::NotFound(_));

        // Using assert_err_variant with a custom message
        assert_err_variant!(
            find_user("999"),
            TestError::NotFound(_),
            "Expected NotFound error variant"
        );
    }

    #[test]
    fn test_with_env_vars_macro() {
        // Test with environment variables
        with_env_vars!(
            ("API_KEY", "test_value"),
            ("DEBUG", "true")
            => {
                assert_eq!(get_api_key(), "test_value");
            }
        );

        // Environment variables are reset after the block
        assert_ne!(get_api_key(), "test_value");
    }

    #[test]
    fn test_test_cases_macro() {
        // Test a function with multiple inputs and expected outputs
        test_cases!(
            is_even,
            ("zero", 0, true),
            ("positive even", 2, true),
            ("positive odd", 3, false),
            ("negative even", -4, true),
            ("negative odd", -7, false)
        );
    }

    #[test]
    fn test_with_mock_time_macro() {
        // Test with mocked time
        with_mock_time!(1617235200 => |time| {
            assert_eq!(format_time(time), "2021-04-01");
        });

        // Test with time advancement
        let (result1, result2) = with_mock_time!(1617235200, 86400 => |time| {
            format_time(time)
        });

        assert_eq!(result1, "2021-04-01");
        assert_eq!(result2, "2021-04-02"); // Advanced by 1 day (86400 seconds)
    }

    #[test]
    fn test_assert_contains_macro() {
        let numbers = vec![1, 2, 3, 4, 5];

        // Test that the collection contains an item matching a predicate
        assert_contains!(numbers, |&n| n == 3);
        assert_contains!(numbers, |&n| n > 4);

        // Test with a custom message
        assert_contains!(
            numbers,
            |&n| n % 2 == 0,
            "Expected collection to contain an even number"
        );
    }
}
