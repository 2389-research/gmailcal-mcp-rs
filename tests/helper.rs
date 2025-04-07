/// Testing Utilities Module
///
/// This module provides shared utilities and helpers for testing the Gmail MCP
/// server application. It includes parameterized test helpers, test macros,
/// environment variable management, and other common testing functionality.
///
use mockall::predicate::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
#[allow(unused_imports)]
use lazy_static::lazy_static;

/// Environment variable management for tests
/// This struct provides a convenient way to set environment variables
/// for tests and automatically restore them when the test is done.
pub struct EnvVarGuard {
    vars: HashMap<String, Option<String>>,
}

impl EnvVarGuard {
    /// Create a new environment variable guard
    pub fn new() -> Self {
        EnvVarGuard {
            vars: HashMap::new(),
        }
    }

    /// Set an environment variable for the duration of the test
    /// 
    /// # Arguments
    /// * `key` - The name of the environment variable
    /// * `value` - The value to set the environment variable to
    pub fn set(&mut self, key: &str, value: &str) {
        let prev = env::var(key).ok();
        self.vars.insert(key.to_string(), prev);
        env::set_var(key, value);
    }

    /// Remove an environment variable for the duration of the test
    /// 
    /// # Arguments
    /// * `key` - The name of the environment variable to remove
    pub fn remove(&mut self, key: &str) {
        let prev = env::var(key).ok();
        self.vars.insert(key.to_string(), prev);
        env::remove_var(key);
    }
}

/// When the EnvVarGuard is dropped (goes out of scope),
/// it restores the original environment variables
impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        for (key, value) in &self.vars {
            match value {
                Some(val) => env::set_var(key, val),
                None => env::remove_var(key),
            }
        }
    }
}

/// Generate a random email address for testing
pub fn random_email() -> String {
    use rand::{thread_rng, Rng};
    
    let chars: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let random_part: String = thread_rng()
        .sample_iter(rand::distributions::Uniform::new(0, chars.len()))
        .take(10)
        .map(|i| chars[i] as char)
        .collect();
    
    format!("test-{}@example.com", random_part)
}

/// Generate a random string for testing
pub fn random_string(length: usize) -> String {
    use rand::{thread_rng, Rng};
    
    let chars: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    thread_rng()
        .sample_iter(rand::distributions::Uniform::new(0, chars.len()))
        .take(length)
        .map(|i| chars[i] as char)
        .collect()
}

/// Test data structures for parameterized tests

/// Holds a test case with input and expected output
pub struct TestCase<I, E> {
    pub name: String,
    pub input: I,
    pub expected: E,
}

impl<I, E> TestCase<I, E> {
    pub fn new(name: &str, input: I, expected: E) -> Self {
        TestCase {
            name: name.to_string(),
            input,
            expected,
        }
    }
}

/// Runs a series of test cases through a test function
pub fn run_test_cases<I: Clone, E: PartialEq + std::fmt::Debug>(
    test_cases: &[TestCase<I, E>],
    test_fn: impl Fn(I) -> E,
) {
    for case in test_cases {
        let actual = test_fn(case.input.clone());
        assert_eq!(
            actual, case.expected,
            "Test case '{}' failed: expected {:?}, got {:?}",
            case.name, case.expected, actual
        );
    }
}

/// Fixture for common test API responses

/// Standard API error response
pub fn api_error_response(code: u16, message: &str) -> Value {
    json!({
        "error": {
            "code": code,
            "message": message,
            "status": "FAILED_PRECONDITION" 
        }
    })
}

/// Create a mock time provider for testing time-based functions
pub struct MockTimeProvider {
    pub current_time: std::time::SystemTime,
}

impl MockTimeProvider {
    pub fn new(seconds_from_epoch: u64) -> Self {
        use std::time::{Duration, UNIX_EPOCH};
        
        let current_time = UNIX_EPOCH + Duration::from_secs(seconds_from_epoch);
        MockTimeProvider { current_time }
    }
    
    pub fn now(&self) -> std::time::SystemTime {
        self.current_time
    }
    
    pub fn advance(&mut self, seconds: u64) {
        use std::time::Duration;
        
        self.current_time += Duration::from_secs(seconds);
    }
}

/// Base64 helpers
pub mod base64_helpers {
    use base64;
    
    /// Encode a string as base64
    pub fn encode(s: &str) -> String {
        base64::encode(s)
    }
    
    /// Decode a base64 string
    pub fn decode(s: &str) -> Result<String, String> {
        base64::decode(s)
            .map_err(|e| format!("Base64 decode error: {}", e))
            .and_then(|bytes| {
                String::from_utf8(bytes)
                    .map_err(|e| format!("UTF-8 decode error: {}", e))
            })
    }
}

/// Create a Gmail message with the given parameters
pub fn create_gmail_message(
    id: &str,
    thread_id: &str,
    subject: &str,
    from: &str,
    to: &str,
    body_text: &str,
    body_html: Option<&str>,
) -> Value {
    let mut payload = json!({
        "headers": [
            { "name": "Subject", "value": subject },
            { "name": "From", "value": from },
            { "name": "To", "value": to },
            { "name": "Date", "value": "Tue, 01 Apr 2025 12:34:56 +0000" }
        ]
    });
    
    if let Some(html) = body_html {
        // Multipart message with both text and HTML
        payload["mimeType"] = json!("multipart/alternative");
        payload["parts"] = json!([
            {
                "mimeType": "text/plain",
                "body": {
                    "data": base64_helpers::encode(body_text),
                    "size": body_text.len()
                }
            },
            {
                "mimeType": "text/html",
                "body": {
                    "data": base64_helpers::encode(html),
                    "size": html.len()
                }
            }
        ]);
    } else {
        // Plain text only message
        payload["mimeType"] = json!("text/plain");
        payload["body"] = json!({
            "data": base64_helpers::encode(body_text),
            "size": body_text.len()
        });
    }
    
    json!({
        "id": id,
        "threadId": thread_id,
        "snippet": &body_text[..std::cmp::min(body_text.len(), 100)],
        "payload": payload
    })
}

/// Create a calendar event with the given parameters
pub fn create_calendar_event(
    id: &str,
    summary: &str,
    description: Option<&str>,
    start_time: &str,
    end_time: &str,
    attendees: Vec<&str>,
) -> Value {
    let mut attendee_list = Vec::new();
    for email in attendees {
        attendee_list.push(json!({
            "email": email,
            "responseStatus": "needsAction"
        }));
    }
    
    json!({
        "id": id,
        "summary": summary,
        "description": description,
        "start": {
            "dateTime": start_time,
            "timeZone": "UTC"
        },
        "end": {
            "dateTime": end_time,
            "timeZone": "UTC" 
        },
        "attendees": attendee_list,
        "created": "2025-04-01T10:00:00Z",
        "updated": "2025-04-01T10:00:00Z"
    })
}

/// Create a contact with the given parameters
pub fn create_contact(
    resource_name: &str,
    display_name: &str,
    email_addresses: Vec<&str>,
    phone_numbers: Vec<&str>,
) -> Value {
    let mut emails = Vec::new();
    for (i, email) in email_addresses.iter().enumerate() {
        emails.push(json!({
            "metadata": {
                "primary": i == 0
            },
            "value": email
        }));
    }
    
    let mut phones = Vec::new();
    for (i, phone) in phone_numbers.iter().enumerate() {
        phones.push(json!({
            "metadata": {
                "primary": i == 0
            },
            "value": phone
        }));
    }
    
    json!({
        "resourceName": resource_name,
        "etag": "test-etag",
        "names": [
            {
                "metadata": {
                    "primary": true
                },
                "displayName": display_name
            }
        ],
        "emailAddresses": emails,
        "phoneNumbers": phones
    })
}

/// Provides assertions for comparing API responses
#[macro_export]
macro_rules! assert_json_contains {
    ($actual:expr, $expected:expr) => {
        let actual_value = &$actual;
        let expected_value = &$expected;
        
        fn check_json_subset(actual: &serde_json::Value, expected: &serde_json::Value) -> bool {
            match (actual, expected) {
                (serde_json::Value::Object(actual_obj), serde_json::Value::Object(expected_obj)) => {
                    for (key, expected_val) in expected_obj {
                        match actual_obj.get(key) {
                            Some(actual_val) => {
                                if !check_json_subset(actual_val, expected_val) {
                                    return false;
                                }
                            },
                            None => return false,
                        }
                    }
                    true
                },
                (serde_json::Value::Array(actual_arr), serde_json::Value::Array(expected_arr)) => {
                    // Check if all expected elements are in the actual array
                    // This is an approximation as order might be different
                    if expected_arr.len() > actual_arr.len() {
                        return false;
                    }
                    
                    for expected_item in expected_arr {
                        if !actual_arr.iter().any(|actual_item| check_json_subset(actual_item, expected_item)) {
                            return false;
                        }
                    }
                    true
                },
                (actual, expected) => actual == expected,
            }
        }
        
        assert!(
            check_json_subset(actual_value, expected_value),
            "JSON assertion failed: expected {:?} to be contained in {:?}",
            expected_value, actual_value
        );
    };
}

/// Helper macro to parameterize tests with different inputs
#[macro_export]
macro_rules! parameterized_test {
    ($name:ident, $($param:ident),+ $(,)?) => {
        mod $name {
            $(
                #[test]
                fn $param() {
                    super::$name(stringify!($param));
                }
            )+
        }
    };
}