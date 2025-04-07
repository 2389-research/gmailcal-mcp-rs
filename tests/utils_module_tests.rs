/// Utils Module Tests
///
/// This module tests the utility functions in the utils.rs file,
/// including error mapping, base64 encoding/decoding, and parsing.
use mcp_gmailcal::errors::GmailApiError;
use mcp_gmailcal::utils::{
    decode_base64, encode_base64_url_safe, map_gmail_error, parse_max_results, to_mcp_error,
    error_codes::{get_error_description, get_troubleshooting_steps},
    error_codes::{AUTH_ERROR, API_ERROR, CONFIG_ERROR, MESSAGE_FORMAT_ERROR, GENERAL_ERROR}
};
use serde_json::json;

#[cfg(test)]
mod utils_tests {
    use super::*;

    #[test]
    fn test_parse_max_results() {
        // Test cases for parse_max_results function using the TestCase struct
        let test_cases = [
            // Test with number values
            ("valid_number", Some(json!(10)), 20, 10),
            ("zero", Some(json!(0)), 20, 0),
            ("default_if_none", None, 20, 20),
            ("large_number", Some(json!(999_999_999)), 20, 999_999_999),
            // Ensure too large numbers are handled correctly
            ("very_large_number", Some(json!(4_294_967_296_i64)), 20, 20), // u32::MAX + 1
            ("negative_number", Some(json!(-5)), 20, 20),
            
            // Test with string values
            ("string_number", Some(json!("30")), 20, 30),
            ("string_zero", Some(json!("0")), 20, 0),
            ("invalid_string", Some(json!("not_a_number")), 20, 20),
            ("empty_string", Some(json!("")), 20, 20),
            
            // Test with other JSON types
            ("boolean_true", Some(json!(true)), 20, 20),
            ("boolean_false", Some(json!(false)), 20, 20),
            ("null_value", Some(json!(null)), 20, 20),
            ("object_value", Some(json!({"key": "value"})), 20, 20),
            ("array_value", Some(json!([1, 2, 3])), 20, 20),
        ];
        
        for (name, input, default, expected) in test_cases {
            let result = parse_max_results(input, default);
            assert_eq!(
                result, expected,
                "Test case '{}' failed: expected {}, got {}",
                name, expected, result
            );
        }
    }

    #[test]
    fn test_decode_base64() {
        // Basic cases
        assert_eq!(decode_base64("SGVsbG8gV29ybGQ=").unwrap(), "Hello World");
        assert_eq!(decode_base64("").unwrap(), "");
        
        // URL-safe base64
        assert_eq!(
            decode_base64("SGVsbG8gV29ybGQ").unwrap(), 
            "Hello World",
            "URL-safe base64 without padding should decode correctly"
        );
        
        // Special characters
        let special_chars = "!@#$%^&*()_+-=[]{}|;:,.<>?";
        let encoded = encode_base64_url_safe(special_chars.as_bytes());
        assert_eq!(
            decode_base64(&encoded).unwrap(), 
            special_chars,
            "Special characters should round-trip correctly"
        );
        
        // Unicode characters
        let unicode = "こんにちは世界";
        let encoded = encode_base64_url_safe(unicode.as_bytes());
        assert_eq!(
            decode_base64(&encoded).unwrap(), 
            unicode,
            "Unicode characters should round-trip correctly"
        );
        
        // Error cases
        assert!(
            decode_base64("This is not valid base64!").is_err(),
            "Invalid base64 should return an error"
        );
        
        // Malformed base64 (incorrect length)
        assert!(
            decode_base64("SGVsbG").is_err(),
            "Malformed base64 should return an error"
        );
    }

    #[test]
    fn test_encode_base64_url_safe() {
        // Basic encoding - URL-safe encoding often doesn't include padding (=)
        let encoded = encode_base64_url_safe(b"Hello World");
        assert_eq!(decode_base64(&encoded).unwrap(), "Hello World");
        assert_eq!(encode_base64_url_safe(b""), "");
        
        // Test with URL-unsafe characters
        let encoded = encode_base64_url_safe(b"Hello+World/");
        let decoded = decode_base64(&encoded).unwrap();
        assert_eq!(decoded, "Hello+World/", "URL-unsafe characters should encode/decode correctly");
        
        // Large data
        let large_data = "A".repeat(1000);
        let encoded = encode_base64_url_safe(large_data.as_bytes());
        assert!(
            encoded.len() > 1000,
            "Encoded data should be longer than original"
        );
        assert_eq!(
            decode_base64(&encoded).unwrap(),
            large_data,
            "Large data should round-trip correctly"
        );
        
        // Binary data
        let binary_data = [0u8, 1u8, 255u8, 254u8];
        let encoded = encode_base64_url_safe(&binary_data);
        assert!(!encoded.contains('+'), "Should not contain '+' character");
        assert!(!encoded.contains('/'), "Should not contain '/' character");
        
        // Make sure URL-safe encoding uses -_ instead of +/
        let unsafe_chars = "+/";
        let encoded = encode_base64_url_safe(unsafe_chars.as_bytes());
        assert!(!encoded.contains('+'), "Should not contain '+' character");
        assert!(!encoded.contains('/'), "Should not contain '/' character");
        assert_eq!(decode_base64(&encoded).unwrap(), unsafe_chars);
    }

    #[test]
    fn test_error_codes_descriptions() {
        // Test each error code's description and troubleshooting steps
        assert!(get_error_description(CONFIG_ERROR).contains("Configuration Error"));
        assert!(get_error_description(AUTH_ERROR).contains("Authentication Error"));
        assert!(get_error_description(API_ERROR).contains("Gmail API Error"));
        assert!(get_error_description(MESSAGE_FORMAT_ERROR).contains("Message Format Error"));
        assert!(get_error_description(GENERAL_ERROR).contains("General Error"));
        
        // Test unknown error code
        assert!(get_error_description(9999).contains("Unknown Error"));
        
        // Test troubleshooting steps
        assert!(get_troubleshooting_steps(CONFIG_ERROR).contains("environment variables"));
        assert!(get_troubleshooting_steps(AUTH_ERROR).contains("OAuth credentials"));
        assert!(get_troubleshooting_steps(API_ERROR).contains("API request failed"));
        assert!(get_troubleshooting_steps(MESSAGE_FORMAT_ERROR).contains("unexpected format"));
        assert!(get_troubleshooting_steps(GENERAL_ERROR).contains("server logs"));
        
        // Test unknown error code troubleshooting
        assert!(get_troubleshooting_steps(9999).contains("server logs"));
    }

    #[test]
    fn test_to_mcp_error() {
        // Since we can't directly access the McpError's private fields, we'll test
        // the error construction by examining the Debug output which contains the code
        let error_message = "Test error message";
        
        let config_error = to_mcp_error(error_message, CONFIG_ERROR);
        let debug_str = format!("{:?}", config_error);
        assert!(debug_str.contains(&format!("{}", CONFIG_ERROR)));
        
        let auth_error = to_mcp_error(error_message, AUTH_ERROR);
        let debug_str = format!("{:?}", auth_error);
        assert!(debug_str.contains(&format!("{}", AUTH_ERROR)));
        
        let api_error = to_mcp_error(error_message, API_ERROR);
        let debug_str = format!("{:?}", api_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));
        
        let general_error = to_mcp_error(error_message, GENERAL_ERROR);
        let debug_str = format!("{:?}", general_error);
        assert!(debug_str.contains(&format!("{}", GENERAL_ERROR)));
    }

    #[test]
    fn test_map_gmail_error() {
        // Test mapping different Gmail API errors to MCP errors
        // We'll use the Debug representation to check error code mapping
        
        // ApiError variants
        let rate_limit_error = map_gmail_error(GmailApiError::ApiError("rate limit exceeded".to_string()));
        let debug_str = format!("{:?}", rate_limit_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));
        
        let network_error = map_gmail_error(GmailApiError::ApiError("network error occurred".to_string()));
        let debug_str = format!("{:?}", network_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));
        
        let auth_api_error = map_gmail_error(GmailApiError::ApiError("authentication failed".to_string()));
        let debug_str = format!("{:?}", auth_api_error);
        assert!(debug_str.contains(&format!("{}", AUTH_ERROR)));
        
        let format_error = map_gmail_error(GmailApiError::ApiError("missing field in response".to_string()));
        let debug_str = format!("{:?}", format_error);
        assert!(debug_str.contains(&format!("{}", MESSAGE_FORMAT_ERROR)));
        
        let not_found_error = map_gmail_error(GmailApiError::ApiError("resource not found".to_string()));
        let debug_str = format!("{:?}", not_found_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));
        
        let unspecified_error = map_gmail_error(GmailApiError::ApiError("some other error".to_string()));
        let debug_str = format!("{:?}", unspecified_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));
        
        // Other error types
        let auth_error = map_gmail_error(GmailApiError::AuthError("invalid credentials".to_string()));
        let debug_str = format!("{:?}", auth_error);
        assert!(debug_str.contains(&format!("{}", AUTH_ERROR)));
        
        let message_retrieval_error = map_gmail_error(GmailApiError::MessageRetrievalError("message not found".to_string()));
        let debug_str = format!("{:?}", message_retrieval_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));
        
        let message_format_error = map_gmail_error(GmailApiError::MessageFormatError("invalid format".to_string()));
        let debug_str = format!("{:?}", message_format_error);
        assert!(debug_str.contains(&format!("{}", MESSAGE_FORMAT_ERROR)));
        
        let network_error = map_gmail_error(GmailApiError::NetworkError("connection timeout".to_string()));
        let debug_str = format!("{:?}", network_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));
        
        let rate_limit_error = map_gmail_error(GmailApiError::RateLimitError("too many requests".to_string()));
        let debug_str = format!("{:?}", rate_limit_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));
    }
}