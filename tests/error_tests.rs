use mcp_gmailcal::errors::{
    CalendarApiError, ConfigError, GmailApiError, PeopleApiError,
    error_codes::{
        AUTHENTICATION_ERROR, CONFIG_ERROR, CONTACT_NOT_FOUND, DRAFT_NOT_FOUND,
        EVENT_FORMAT_ERROR, EVENT_NOT_FOUND, INTERNAL_ERROR, INVALID_REQUEST,
        MESSAGE_FORMAT_ERROR, MESSAGE_NOT_FOUND, NOT_FOUND_ERROR, RATE_LIMIT_ERROR,
        CALENDAR_NOT_FOUND
    },
};
use reqwest;
use std::env;

/// Error Handling Tests Module
///
/// This module contains tests for the error handling functionality,
/// focusing on error mapping and formatting.

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
        
        // Test EnvError variant with NotUnicode
        #[cfg(unix)] // NotUnicode can only be tested on Unix
        {
            use std::ffi::OsString;
            use std::os::unix::ffi::OsStringExt;
            
            // Create an invalid UTF-8 OsString
            let invalid_utf8 = OsString::from_vec(vec![0x80, 0x80, 0x80]);
            let env_error = ConfigError::EnvError(env::VarError::NotUnicode(invalid_utf8));
            assert!(env_error.to_string().contains("Environment error"));
        }
        
        // Ensure Debug trait is implemented
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("MissingEnvVar"));
        assert!(debug_str.contains("CLIENT_ID"));
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
        
        // Ensure Debug trait is implemented
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("RateLimitError"));
        assert!(debug_str.contains("Too many requests"));
    }
    
    // Test PeopleApiError
    #[test]
    fn test_people_api_error() {
        // Create People API errors
        let error = PeopleApiError::NetworkError("Connection failed".to_string());
        assert!(error.to_string().contains("Connection failed"));
        assert!(error.to_string().contains("Network error"));
        
        let error = PeopleApiError::AuthError("Invalid token".to_string());
        assert!(error.to_string().contains("Invalid token"));
        assert!(error.to_string().contains("Authentication error"));
        
        let error = PeopleApiError::ApiError("API request failed".to_string());
        assert!(error.to_string().contains("API request failed"));
        assert!(error.to_string().contains("People API error"));
        
        let error = PeopleApiError::InvalidInput("Missing name field".to_string());
        assert!(error.to_string().contains("Missing name field"));
        assert!(error.to_string().contains("Invalid input"));
        
        let error = PeopleApiError::ParseError("Failed to parse response".to_string());
        assert!(error.to_string().contains("Failed to parse response"));
        assert!(error.to_string().contains("Parse error"));
        
        // Ensure Debug trait is implemented
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ParseError"));
        assert!(debug_str.contains("Failed to parse response"));
    }
    
    // Test CalendarApiError
    #[test]
    fn test_calendar_api_error() {
        // Create Calendar API errors
        let error = CalendarApiError::ApiError("API request failed".to_string());
        assert!(error.to_string().contains("API request failed"));
        assert!(error.to_string().contains("Calendar API error"));
        
        let error = CalendarApiError::AuthError("Invalid credentials".to_string());
        assert!(error.to_string().contains("Invalid credentials"));
        assert!(error.to_string().contains("Authentication error"));
        
        let error = CalendarApiError::EventRetrievalError("Event not found".to_string());
        assert!(error.to_string().contains("Event not found"));
        assert!(error.to_string().contains("Event retrieval error"));
        
        let error = CalendarApiError::EventFormatError("Malformed event".to_string());
        assert!(error.to_string().contains("Malformed event"));
        assert!(error.to_string().contains("Event format error"));
        
        let error = CalendarApiError::NetworkError("Connection failed".to_string());
        assert!(error.to_string().contains("Connection failed"));
        assert!(error.to_string().contains("Network error"));
        
        let error = CalendarApiError::RateLimitError("Too many requests".to_string());
        assert!(error.to_string().contains("Too many requests"));
        assert!(error.to_string().contains("Rate limit error"));
        
        let error = CalendarApiError::ParseError("Failed to parse date".to_string());
        assert!(error.to_string().contains("Failed to parse date"));
        assert!(error.to_string().contains("Parse error"));
        
        // Ensure Debug trait is implemented
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ParseError"));
        assert!(debug_str.contains("Failed to parse date"));
    }
    
    // Test From<reqwest::Error> implementations
    #[test]
    fn test_error_from_impls() {
        // Since we can't easily create a reqwest::Error directly in tests,
        // we'll just test that the conversion logic exists by verifying that
        // our error types have From implementations for reqwest::Error
        
        // These compile-time checks verify the From implementation exists
        let _: fn(reqwest::Error) -> GmailApiError = Into::into;
        let _: fn(reqwest::Error) -> PeopleApiError = Into::into;
        let _: fn(reqwest::Error) -> CalendarApiError = Into::into;
        
        // If we get here, the From implementation exists
        assert!(true, "From<reqwest::Error> implementations exist");
    }
    
    // Test for error codes constants
    #[test]
    fn test_error_code_constants() {
        // Check that all error codes are defined and accessible
        assert_eq!(INTERNAL_ERROR, "internal_error");
        assert_eq!(AUTHENTICATION_ERROR, "authentication_error");
        assert_eq!(NOT_FOUND_ERROR, "not_found_error");
        assert_eq!(INVALID_REQUEST, "invalid_request");
        assert_eq!(RATE_LIMIT_ERROR, "rate_limit_error");
        assert_eq!(CONFIG_ERROR, "config_error");
        
        // Gmail specific errors
        assert_eq!(MESSAGE_NOT_FOUND, "message_not_found");
        assert_eq!(DRAFT_NOT_FOUND, "draft_not_found");
        assert_eq!(MESSAGE_FORMAT_ERROR, "message_format_error");
        
        // Calendar specific errors
        assert_eq!(CALENDAR_NOT_FOUND, "calendar_not_found");
        assert_eq!(EVENT_NOT_FOUND, "event_not_found");
        assert_eq!(EVENT_FORMAT_ERROR, "event_format_error");
        
        // People API specific errors
        assert_eq!(CONTACT_NOT_FOUND, "contact_not_found");
    }
}