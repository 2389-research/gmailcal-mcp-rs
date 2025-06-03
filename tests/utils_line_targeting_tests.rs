/// Line-Targeting Tests for Utils Module
///
/// These tests specifically target the exact lines that are uncovered in utils.rs.
/// We need to run with RUST_LOG=debug to ensure the log statements are executed.
use env_logger;
use log::{debug, error, LevelFilter};
use mcp_attr::{jsoncall::ErrorCode, Error as McpError};
use mcp_gmailcal::errors::GmailApiError;
use mcp_gmailcal::utils::{
    decode_base64, encode_base64_url_safe, error_codes::API_ERROR, map_gmail_error,
    parse_max_results, to_mcp_error,
};
use serde_json::{json, Number, Value};
use std::env;
use std::sync::Once;

// Initialize logger once for all tests
static INIT: Once = Once::new();
fn init_logger() {
    INIT.call_once(|| {
        // Set RUST_LOG to debug for this module to capture log statements
        env::set_var("RUST_LOG", "debug");
        env_logger::builder()
            .filter_level(LevelFilter::Debug)
            .init();
    });
}

#[cfg(test)]
mod utils_line_targeting_tests {
    use super::*;

    // Directly tests line 59 by ensuring we have exactly u32::MAX
    #[test]
    fn test_exact_u32_max_boundary() {
        init_logger();

        // Test exactly at u32::MAX - this is line 59
        let exact_u32_max = u32::MAX as u64;
        let number = Number::from(exact_u32_max);
        let value = Value::Number(number);

        // This should execute line 59 exactly
        let result = parse_max_results(Some(value), 42);
        assert_eq!(result, u32::MAX);

        println!("Successfully tested u32::MAX boundary condition at line 59");
    }

    // Tests line 76 directly with parse failure
    #[test]
    fn test_string_parse_failure() {
        init_logger();

        // This will hit line 76 (debug log message for string parsing failure)
        let invalid_string = json!("not_a_number");
        let result = parse_max_results(Some(invalid_string), 100);
        assert_eq!(result, 100);

        // Try with more variations to ensure debug log is hit
        let test_cases = ["xyz", "", "-123", "3.14", "9999999999999999999999"];

        for invalid_str in test_cases {
            let invalid_json = json!(invalid_str);
            let result = parse_max_results(Some(invalid_json), 200);
            assert_eq!(result, 200);
        }

        println!("Successfully tested string parse failure at line 76");
    }

    // Tests line 125 directly by forcing error logging
    #[test]
    fn test_error_logging() {
        init_logger();

        // This will hit line 125 (error log in to_mcp_error function)
        let test_error_message = "Test error message for line 125";
        let mcp_error = to_mcp_error(test_error_message, API_ERROR);

        // Verify the error was created properly
        let error_string = format!("{:?}", mcp_error);
        assert!(error_string.contains(test_error_message));
        assert!(error_string.contains(&API_ERROR.to_string()));

        println!("Successfully tested error logging at line 125");
    }

    // Tests line 138 with a specifically crafted quota message
    #[test]
    fn test_quota_error_condition() {
        init_logger();

        // These are intended to hit line 138 directly (the if condition for API rate limits)
        let error_messages = [
            "quota exceeded for this request",
            "rate limit reached",
            "user has exceeded their limit",
        ];

        for msg in error_messages {
            let result = map_gmail_error(GmailApiError::ApiError(msg.to_string()));
            let error_string = format!("{:?}", result);

            // Validate we're hitting the rate limit branch
            assert!(error_string.contains("Gmail API rate limit exceeded"));
            assert!(error_string.contains(&API_ERROR.to_string()));
        }

        // Check we get expected formatted error message
        let quota_error = map_gmail_error(GmailApiError::ApiError("quota".to_string()));
        let debug_str = format!("{:?}", quota_error);
        assert!(debug_str.contains("Gmail API rate limit exceeded"));

        println!("Successfully tested quota error condition at line 138");
    }

    // Tests line 201 by creating a completely generic API error
    #[test]
    fn test_default_api_error_case() {
        init_logger();

        // This is intended to hit line 201 (the default case in map_gmail_error)
        // Need to ensure message doesn't match any of the specific conditions
        let generic_message = "xyz"; // A message that won't match any patterns

        // Verify this message doesn't match any of the condition patterns
        assert!(!generic_message.contains("quota"));
        assert!(!generic_message.contains("rate"));
        assert!(!generic_message.contains("limit"));
        assert!(!generic_message.contains("network"));
        assert!(!generic_message.contains("connection"));
        assert!(!generic_message.contains("timeout"));
        assert!(!generic_message.contains("authentication"));
        assert!(!generic_message.contains("auth"));
        assert!(!generic_message.contains("token"));
        assert!(!generic_message.contains("format"));
        assert!(!generic_message.contains("missing field"));
        assert!(!generic_message.contains("parse"));
        assert!(!generic_message.contains("not found"));
        assert!(!generic_message.contains("404"));

        // This should hit the default case at line 201
        let api_error = map_gmail_error(GmailApiError::ApiError(generic_message.to_string()));

        // Verify we get the expected unspecified error message
        let error_string = format!("{:?}", api_error);
        assert!(error_string.contains("Unspecified Gmail API error"));
        assert!(error_string.contains(generic_message));
        assert!(error_string.contains(&API_ERROR.to_string()));

        println!("Successfully tested default API error case at line 201");
    }

    // Additional comprehensive test with multiple lines covered
    #[test]
    fn test_combined_coverage() {
        init_logger();

        // This test tries to hit all three lines in one test

        // Line 59: u32::MAX boundary in parse_max_results
        let max_u32_val = u32::MAX as u64;
        let num = Number::from(max_u32_val);
        let val = Value::Number(num);
        let result = parse_max_results(Some(val), 999);
        assert_eq!(result, u32::MAX);

        // Line 76: String parse error in parse_max_results
        let bad_string = json!("bad_number");
        let result = parse_max_results(Some(bad_string), 888);
        assert_eq!(result, 888);

        // Line 125: Error log in to_mcp_error
        let err_msg = "Combined test error message";
        let _ = to_mcp_error(err_msg, API_ERROR);

        // Line 138: Rate limit condition in map_gmail_error
        let quota_msg = map_gmail_error(GmailApiError::ApiError("quota exceeded".to_string()));
        assert!(format!("{:?}", quota_msg).contains("Gmail API rate limit exceeded"));

        // Line 201: Default case in map_gmail_error
        let default_msg = map_gmail_error(GmailApiError::ApiError("abc".to_string()));
        assert!(format!("{:?}", default_msg).contains("Unspecified Gmail API error"));

        println!("Successfully tested all target lines in combined test");
    }
}
