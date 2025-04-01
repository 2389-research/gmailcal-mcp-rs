use mcp_gmailcal::config::ConfigError;
/// Error Handling Tests Module
///
/// This module contains tests for the error handling functionality,
/// focusing on error mapping and formatting.
///
use mcp_gmailcal::gmail_api::GmailApiError;
use std::env;

#[cfg(test)]
mod error_tests {
    use super::*;

    // Test ConfigError
    #[test]
    fn test_config_error() {
        // Create a configuration error
        let error = ConfigError::MissingEnvVar("CLIENT_ID".to_string());

        // Verify the error message
        assert!(error.to_string().contains("CLIENT_ID"));
        assert!(error.to_string().contains("Missing environment variable"));

        // Test EnvError variant
        let env_error = ConfigError::EnvError(env::VarError::NotPresent);
        assert!(env_error.to_string().contains("Environment error"));
    }

    // Test GmailApiError
    #[test]
    fn test_gmail_api_error() {
        // Create Gmail API errors
        let error = GmailApiError::NetworkError("Failed to connect".to_string());

        // Verify the error message
        assert!(error.to_string().contains("Failed to connect"));
        assert!(error.to_string().contains("Network error"));

        let error = GmailApiError::ApiError("Invalid request".to_string());
        assert!(error.to_string().contains("Invalid request"));
        assert!(error.to_string().contains("Gmail API error"));

        let error = GmailApiError::AuthError("Invalid credentials".to_string());
        assert!(error.to_string().contains("Invalid credentials"));
        assert!(error.to_string().contains("Authentication error"));

        let error = GmailApiError::MessageFormatError("Malformed message".to_string());
        assert!(error.to_string().contains("Malformed message"));
        assert!(error.to_string().contains("Message format error"));

        let error = GmailApiError::MessageRetrievalError("Message not found".to_string());
        assert!(error.to_string().contains("Message not found"));
        assert!(error.to_string().contains("Message retrieval error"));

        let error = GmailApiError::RateLimitError("Too many requests".to_string());
        assert!(error.to_string().contains("Too many requests"));
        assert!(error.to_string().contains("Rate limit error"));
    }
}
