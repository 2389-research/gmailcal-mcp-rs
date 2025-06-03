/// Utils Module Tests
///
/// This module tests the utility functions in the utils.rs file,
/// including error mapping, base64 encoding/decoding, and parsing.
use mcp_gmailcal::errors::GmailApiError;
use mcp_gmailcal::utils::{
    decode_base64, encode_base64_url_safe,
    error_codes::{get_error_description, get_troubleshooting_steps},
    error_codes::{API_ERROR, AUTH_ERROR, CONFIG_ERROR, GENERAL_ERROR, MESSAGE_FORMAT_ERROR},
    map_gmail_error, parse_max_results, to_mcp_error,
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
            // Ensure too large numbers are handled correctly - these test the number bounds checking
            ("very_large_number", Some(json!(4_294_967_296_i64)), 20, 20), // u32::MAX + 1
            ("u32_max", Some(json!(4_294_967_295_u64)), 20, 4_294_967_295), // u32::MAX exactly
            ("negative_number", Some(json!(-5)), 20, 20),
            // Test with string values - testing string conversion branches
            ("string_number", Some(json!("30")), 20, 30),
            ("string_zero", Some(json!("0")), 20, 0),
            (
                "string_large_number",
                Some(json!("4294967295")),
                20,
                4294967295,
            ), // u32::MAX as string
            ("invalid_string", Some(json!("not_a_number")), 20, 20),
            ("empty_string", Some(json!("")), 20, 20),
            // Test with other JSON types - testing the type branches in match statement
            ("boolean_true", Some(json!(true)), 20, 20),
            ("boolean_false", Some(json!(false)), 20, 20),
            ("null_value", Some(json!(null)), 20, 20),
            ("object_value", Some(json!({"key": "value"})), 20, 20),
            ("array_value", Some(json!([1, 2, 3])), 20, 20),
            // Floating-point values are treated as invalid in parse_max_results
            ("float_whole_number", Some(json!(42.0)), 20, 20),
            ("float_fractional", Some(json!(42.5)), 20, 20),
            // Edge cases
            ("zero_default", Some(json!(10)), 0, 10), // Using zero as default
            ("min_u32", Some(json!(1)), 20, 1),       // Min positive value
            // These test the debug log branches
            (
                "impossible_negative_i64",
                Some(json!(-9223372036854775808_i64)),
                20,
                20,
            ), // i64::MIN
            ("non_u32_float", Some(json!(42.7)), 20, 20), // Definitely not convertible to u32
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

        // Error cases - testing both error paths in decode_base64

        // Base64 decoding error branch
        assert!(
            decode_base64("This is not valid base64!").is_err(),
            "Invalid base64 should return an error"
        );
        let invalid_base64_result = decode_base64("This is not valid base64!");
        assert!(
            invalid_base64_result.is_err(),
            "Invalid base64 should return an error"
        );
        let err_msg = invalid_base64_result.unwrap_err();
        assert!(
            err_msg.contains("Error decoding base64"),
            "Error message should indicate base64 decoding error"
        );

        // Malformed base64 (incorrect length)
        assert!(
            decode_base64("SGVsbG").is_err(),
            "Malformed base64 should return an error"
        );

        // UTF-8 conversion error branch
        let invalid_utf8 = encode_base64_url_safe(&[0xFF, 0xFE, 0xFD]);
        let result = decode_base64(&invalid_utf8);
        assert!(
            result.is_err(),
            "Invalid UTF-8 sequence should return an error"
        );
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("Error converting base64 to string")
                || err_msg.contains("Error decoding base64"),
            "Error message should indicate conversion error"
        );

        // More complex test cases
        // Testing standard base64 with padding
        assert_eq!(
            decode_base64("YQ==").unwrap(),
            "a",
            "Standard base64 with padding should decode"
        );

        // Testing long base64 strings
        let long_text = "A".repeat(1000);
        let encoded = encode_base64_url_safe(long_text.as_bytes());
        assert_eq!(
            decode_base64(&encoded).unwrap(),
            long_text,
            "Long text should decode correctly"
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
        assert_eq!(
            decoded, "Hello+World/",
            "URL-unsafe characters should encode/decode correctly"
        );

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

        // Test with data that would require padding in standard base64
        let data_with_padding = "a"; // This would require padding in standard base64
        let encoded = encode_base64_url_safe(data_with_padding.as_bytes());
        // Note: The base64::URL_SAFE config might still include padding, so we don't assert its absence
        assert_eq!(decode_base64(&encoded).unwrap(), data_with_padding);
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

        // Test all descriptions have required components
        for code in [
            CONFIG_ERROR,
            AUTH_ERROR,
            API_ERROR,
            MESSAGE_FORMAT_ERROR,
            GENERAL_ERROR,
        ] {
            let description = get_error_description(code);
            let troubleshooting = get_troubleshooting_steps(code);

            // All descriptions should end with "Error"
            assert!(
                description.ends_with("Error") || description.contains("Error:"),
                "Error description for code {} should end with 'Error'",
                code
            );

            // All troubleshooting steps should be non-empty
            assert!(
                !troubleshooting.is_empty(),
                "Troubleshooting steps for code {} should not be empty",
                code
            );
        }
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

        // Test with empty error message
        let empty_error = to_mcp_error("", CONFIG_ERROR);
        let debug_str = format!("{:?}", empty_error);
        assert!(debug_str.contains(&format!("{}", CONFIG_ERROR)));

        // Test the error message content (use Debug representation since Display isn't available)
        let long_message =
            "This is a very detailed error message that explains what went wrong in great detail.";
        let error = to_mcp_error(long_message, CONFIG_ERROR);
        let debug_str = format!("{:?}", error);

        // The error should contain the message and error code
        assert!(
            debug_str.contains(&CONFIG_ERROR.to_string()),
            "Error should contain the error code"
        );
        // We can't check for the exact message format since we're using Debug formatting
    }

    #[test]
    fn test_map_gmail_error() {
        // Test mapping different Gmail API errors to MCP errors
        // We'll use the Debug representation to check error code mapping

        // ApiError variants
        // Test quota/rate limit section
        let rate_limit_error =
            map_gmail_error(GmailApiError::ApiError("rate limit exceeded".to_string()));
        let debug_str = format!("{:?}", rate_limit_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        let quota_error = map_gmail_error(GmailApiError::ApiError("quota exceeded".to_string()));
        let debug_str = format!("{:?}", quota_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        let limit_error = map_gmail_error(GmailApiError::ApiError("user rate limit".to_string()));
        let debug_str = format!("{:?}", limit_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        // Test network/connection section
        let network_error = map_gmail_error(GmailApiError::ApiError(
            "network error occurred".to_string(),
        ));
        let debug_str = format!("{:?}", network_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        let connection_error =
            map_gmail_error(GmailApiError::ApiError("connection refused".to_string()));
        let debug_str = format!("{:?}", connection_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        let timeout_error =
            map_gmail_error(GmailApiError::ApiError("timeout occurred".to_string()));
        let debug_str = format!("{:?}", timeout_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        // Test auth/token section
        let auth_api_error =
            map_gmail_error(GmailApiError::ApiError("authentication failed".to_string()));
        let debug_str = format!("{:?}", auth_api_error);
        assert!(debug_str.contains(&format!("{}", AUTH_ERROR)));

        let token_error = map_gmail_error(GmailApiError::ApiError("token expired".to_string()));
        let debug_str = format!("{:?}", token_error);
        assert!(debug_str.contains(&format!("{}", AUTH_ERROR)));

        // Test format/parsing section
        let format_error = map_gmail_error(GmailApiError::ApiError(
            "missing field in response".to_string(),
        ));
        let debug_str = format!("{:?}", format_error);
        assert!(debug_str.contains(&format!("{}", MESSAGE_FORMAT_ERROR)));

        let parse_error = map_gmail_error(GmailApiError::ApiError("parse error".to_string()));
        let debug_str = format!("{:?}", parse_error);
        assert!(debug_str.contains(&format!("{}", MESSAGE_FORMAT_ERROR)));

        // Test not found section
        let not_found_error =
            map_gmail_error(GmailApiError::ApiError("resource not found".to_string()));
        let debug_str = format!("{:?}", not_found_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        let error_404 = map_gmail_error(GmailApiError::ApiError("404 error".to_string()));
        let debug_str = format!("{:?}", error_404);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        // Test unspecified default case
        let unspecified_error =
            map_gmail_error(GmailApiError::ApiError("some other error".to_string()));
        let debug_str = format!("{:?}", unspecified_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        // Other error types (testing all enum variants)
        let auth_error =
            map_gmail_error(GmailApiError::AuthError("invalid credentials".to_string()));
        let debug_str = format!("{:?}", auth_error);
        assert!(debug_str.contains(&format!("{}", AUTH_ERROR)));

        let message_retrieval_error = map_gmail_error(GmailApiError::MessageRetrievalError(
            "message not found".to_string(),
        ));
        let debug_str = format!("{:?}", message_retrieval_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        let message_format_error = map_gmail_error(GmailApiError::MessageFormatError(
            "invalid format".to_string(),
        ));
        let debug_str = format!("{:?}", message_format_error);
        assert!(debug_str.contains(&format!("{}", MESSAGE_FORMAT_ERROR)));

        let network_error = map_gmail_error(GmailApiError::NetworkError(
            "connection timeout".to_string(),
        ));
        let debug_str = format!("{:?}", network_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        let rate_limit_error = map_gmail_error(GmailApiError::RateLimitError(
            "too many requests".to_string(),
        ));
        let debug_str = format!("{:?}", rate_limit_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        // Test for empty error messages as well
        let empty_api_error = map_gmail_error(GmailApiError::ApiError("".to_string()));
        let debug_str = format!("{:?}", empty_api_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        let empty_auth_error = map_gmail_error(GmailApiError::AuthError("".to_string()));
        let debug_str = format!("{:?}", empty_auth_error);
        assert!(debug_str.contains(&format!("{}", AUTH_ERROR)));

        let empty_message_error =
            map_gmail_error(GmailApiError::MessageRetrievalError("".to_string()));
        let debug_str = format!("{:?}", empty_message_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        let empty_format_error = map_gmail_error(GmailApiError::MessageFormatError("".to_string()));
        let debug_str = format!("{:?}", empty_format_error);
        assert!(debug_str.contains(&format!("{}", MESSAGE_FORMAT_ERROR)));

        let empty_network_error = map_gmail_error(GmailApiError::NetworkError("".to_string()));
        let debug_str = format!("{:?}", empty_network_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        let empty_rate_limit_error = map_gmail_error(GmailApiError::RateLimitError("".to_string()));
        let debug_str = format!("{:?}", empty_rate_limit_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));

        // Using Debug format, we can only verify the error code
        assert!(
            debug_str.contains(&API_ERROR.to_string()),
            "Error should contain the API_ERROR code"
        );
    }

    #[test]
    fn test_error_code_in_mcp_error() {
        // Since we can only use Debug format, we'll just verify the error codes are set correctly

        // Test with different error codes
        let config_error = to_mcp_error("Config error message", CONFIG_ERROR);
        let debug_str = format!("{:?}", config_error);
        assert!(
            debug_str.contains(&CONFIG_ERROR.to_string()),
            "Error should contain CONFIG_ERROR code"
        );

        let auth_error = to_mcp_error("Auth error message", AUTH_ERROR);
        let debug_str = format!("{:?}", auth_error);
        assert!(
            debug_str.contains(&AUTH_ERROR.to_string()),
            "Error should contain AUTH_ERROR code"
        );

        let api_error = to_mcp_error("API error message", API_ERROR);
        let debug_str = format!("{:?}", api_error);
        assert!(
            debug_str.contains(&API_ERROR.to_string()),
            "Error should contain API_ERROR code"
        );

        let message_format_error = to_mcp_error("Format error message", MESSAGE_FORMAT_ERROR);
        let debug_str = format!("{:?}", message_format_error);
        assert!(
            debug_str.contains(&MESSAGE_FORMAT_ERROR.to_string()),
            "Error should contain MESSAGE_FORMAT_ERROR code"
        );
    }

    #[test]
    fn test_error_codes_exhaustive() {
        // Test that all defined error codes have descriptions and troubleshooting steps
        let error_codes = [
            CONFIG_ERROR,
            AUTH_ERROR,
            API_ERROR,
            MESSAGE_FORMAT_ERROR,
            GENERAL_ERROR,
        ];

        for &code in &error_codes {
            let description = get_error_description(code);
            let steps = get_troubleshooting_steps(code);

            assert!(
                !description.is_empty(),
                "Description for code {} should not be empty",
                code
            );
            assert!(
                !steps.is_empty(),
                "Troubleshooting steps for code {} should not be empty",
                code
            );

            // Description should not be the unknown error one
            assert_ne!(
                description,
                get_error_description(9999),
                "Error code {} should have a specific description",
                code
            );

            // Steps should not be the unknown error one
            assert_ne!(
                steps,
                get_troubleshooting_steps(9999),
                "Error code {} should have specific troubleshooting steps",
                code
            );
        }
    }
}
