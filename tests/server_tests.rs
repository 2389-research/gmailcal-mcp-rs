/// Server and MCP Command Tests Module
///
/// This module contains tests for the GmailServer and MCP command handling functionality,
/// focusing on command parsing, validation, and response formatting.
///
use mcp_gmailcal::{
    GmailServer,
    errors::{GmailApiError, ConfigError},
    utils::parse_max_results,
};
use serde_json::json;
use std::env;
use std::sync::Once;

// Used to ensure environment setup happens only once
static INIT: Once = Once::new();

// Setup function to initialize environment variables for testing
fn setup() {
    INIT.call_once(|| {
        // Set mock environment variables for testing
        env::set_var("GMAIL_CLIENT_ID", "test_client_id");
        env::set_var("GMAIL_CLIENT_SECRET", "test_client_secret");
        env::set_var("GMAIL_REFRESH_TOKEN", "test_refresh_token");
        env::set_var("GMAIL_ACCESS_TOKEN", "test_access_token");
        env::set_var("GMAIL_REDIRECT_URI", "test_redirect_uri");
    });
}

#[cfg(test)]
mod server_tests {
    use super::*;

    // Basic test that verifies the server can be created
    #[test]
    fn test_server_creation() {
        setup();
        let _server = GmailServer::new();
        
        // Test Default implementation
        let _default_server = GmailServer::default();
        
        // The server should exist, which is already verified by successful creation
        // GmailServer might be a zero-sized type, so we won't check its size
    }

    // Test parameter parsing for MCP commands
    #[test]
    fn test_command_parsing() {
        // Test the parse_max_results helper function with different input types
        
        // With valid numeric input
        let params = json!({
            "max_results": 5
        });
        
        assert_eq!(
            parse_max_results(Some(params["max_results"].clone()), 10),
            5
        );
        
        // With valid string input
        let params = json!({
            "max_results": "5"
        });
        
        assert_eq!(
            parse_max_results(Some(params["max_results"].clone()), 10),
            5
        );
        
        // With invalid string input
        let params = json!({
            "max_results": "not a number"
        });
        
        assert_eq!(
            parse_max_results(Some(params["max_results"].clone()), 10),
            10
        );
        
        // With null input
        let params = json!({
            "max_results": null
        });
        
        assert_eq!(
            parse_max_results(Some(params["max_results"].clone()), 10),
            10
        );
        
        // With boolean input
        let params = json!({
            "max_results": true
        });
        
        assert_eq!(
            parse_max_results(Some(params["max_results"].clone()), 10),
            10
        );
        
        // With array input
        let params = json!({
            "max_results": [1, 2, 3]
        });
        
        assert_eq!(
            parse_max_results(Some(params["max_results"].clone()), 10),
            10
        );
        
        // With object input
        let params = json!({
            "max_results": {"value": 5}
        });
        
        assert_eq!(
            parse_max_results(Some(params["max_results"].clone()), 10),
            10
        );
        
        // With no input
        assert_eq!(
            parse_max_results(None, 10),
            10
        );
    }
    
    // Test error formatting for error types in the codebase
    #[test]
    fn test_error_formatting() {
        // Test ConfigError formatting
        let missing_var_error = ConfigError::MissingEnvVar("TEST_VAR".to_string());
        let error_string = format!("{}", missing_var_error);
        
        assert!(error_string.contains("Missing environment variable"));
        assert!(error_string.contains("TEST_VAR"));
        
        // Test VarError formatting
        let env_error = ConfigError::EnvError(env::VarError::NotPresent);
        let error_string = format!("{}", env_error);
        
        assert!(error_string.contains("Environment error"));
        
        // Test GmailApiError formatting
        let gmail_errors = [
            (GmailApiError::ApiError("API error".to_string()), "Gmail API error"),
            (GmailApiError::AuthError("Auth error".to_string()), "Authentication error"),
            (GmailApiError::MessageRetrievalError("Not found".to_string()), "Message retrieval error"),
            (GmailApiError::MessageFormatError("Invalid format".to_string()), "Message format error"),
            (GmailApiError::NetworkError("Connection error".to_string()), "Network error"),
            (GmailApiError::RateLimitError("Too many requests".to_string()), "Rate limit error"),
        ];
        
        for (error, expected_text) in gmail_errors {
            let error_string = format!("{}", error);
            assert!(
                error_string.contains(expected_text),
                "Error '{}' should contain '{}'",
                error_string, expected_text
            );
        }
    }
    
    // Test debug formatting for error types
    #[test]
    fn test_error_debug() {
        // Test ConfigError debug formatting
        let missing_var_error = ConfigError::MissingEnvVar("TEST_VAR".to_string());
        let debug_string = format!("{:?}", missing_var_error);
        
        assert!(debug_string.contains("MissingEnvVar"));
        assert!(debug_string.contains("TEST_VAR"));
        
        // Test VarError debug formatting
        let env_error = ConfigError::EnvError(env::VarError::NotPresent);
        let debug_string = format!("{:?}", env_error);
        
        assert!(debug_string.contains("EnvError"));
        assert!(debug_string.contains("NotPresent"));
        
        // Test GmailApiError debug formatting
        let gmail_errors = [
            (GmailApiError::ApiError("API error".to_string()), "ApiError"),
            (GmailApiError::AuthError("Auth error".to_string()), "AuthError"),
            (GmailApiError::MessageRetrievalError("Not found".to_string()), "MessageRetrievalError"),
            (GmailApiError::MessageFormatError("Invalid format".to_string()), "MessageFormatError"),
            (GmailApiError::NetworkError("Connection error".to_string()), "NetworkError"),
            (GmailApiError::RateLimitError("Too many requests".to_string()), "RateLimitError"),
        ];
        
        for (error, expected_text) in gmail_errors {
            let debug_string = format!("{:?}", error);
            assert!(
                debug_string.contains(expected_text),
                "Debug of '{}' should contain '{}'",
                debug_string, expected_text
            );
        }
    }
    
    // Test parsing edge cases in parse_max_results
    #[test]
    fn test_parse_max_results_edge_cases() {
        // Test with zero
        assert_eq!(parse_max_results(Some(json!(0)), 10), 0);
        
        // Test with negative number
        assert_eq!(parse_max_results(Some(json!(-1)), 10), 10);
        
        // Test with large number
        assert_eq!(parse_max_results(Some(json!(u32::MAX)), 10), u32::MAX);
        
        // Test with number larger than u32::MAX
        let large_number = (u32::MAX as u64) + 1;
        assert_eq!(parse_max_results(Some(json!(large_number)), 10), 10);
        
        // Test with very large string number
        assert_eq!(parse_max_results(Some(json!("4294967295")), 10), u32::MAX);
        
        // Test with empty string
        assert_eq!(parse_max_results(Some(json!("")), 10), 10);
    }
}