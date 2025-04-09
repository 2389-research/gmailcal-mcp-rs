/// Final Coverage Tests for Utils Module
///
/// This module specifically targets the remaining uncovered lines in utils.rs
/// to achieve 100% coverage as required by the test plan.
use mcp_gmailcal::errors::GmailApiError;
use mcp_gmailcal::utils::{
    map_gmail_error, parse_max_results, to_mcp_error,
    error_codes::API_ERROR
};
use serde_json::{json, Number, Value};
use std::env;
use std::sync::Once;

// Initialize logger once for all tests
static INIT: Once = Once::new();
fn init_logger() {
    INIT.call_once(|| {
        env::set_var("RUST_LOG", "debug");
        env_logger::builder().init();
    });
}

#[cfg(test)]
mod utils_final_coverage_tests {
    use super::*;
    
    /// Test to target line 59 in utils.rs - u32::MAX boundary case
    #[test]
    fn test_max_u32_boundary() {
        init_logger();
        
        // Test exactly at u32::MAX 
        let exact_u32_max = u32::MAX as u64;
        let number = Number::from(exact_u32_max);
        let value = Value::Number(number);
        
        let result = parse_max_results(Some(value), 100);
        assert_eq!(result, u32::MAX);
    }
    
    /// Test to target line 138 in utils.rs - rate limit error condition
    #[test]
    fn test_rate_limit_condition() {
        init_logger();
        
        // Test specifically to trigger line 138 in map_gmail_error (rate limit check)
        let quota_error = map_gmail_error(GmailApiError::ApiError("quota limit".to_string()));
        let debug_str = format!("{:?}", quota_error);
        
        // Verify we hit the rate limit branch
        assert!(debug_str.contains("Gmail API rate limit exceeded"));
        assert!(debug_str.contains(&API_ERROR.to_string()));
        
        // Test with rate limit keyword
        let rate_error = map_gmail_error(GmailApiError::ApiError("rate limit".to_string()));
        let debug_str = format!("{:?}", rate_error);
        assert!(debug_str.contains("Gmail API rate limit exceeded"));
    }
    
    /// Test to target line 201 in utils.rs - unspecified API error
    #[test]
    fn test_unspecified_api_error() {
        init_logger();
        
        // Create a message that doesn't match any specific error condition patterns
        // to trigger the default case at line 201
        let generic_message = "xyz";
        let api_error = map_gmail_error(GmailApiError::ApiError(generic_message.to_string()));
        
        // Verify we hit the default branch
        let error_string = format!("{:?}", api_error);
        assert!(error_string.contains("Unspecified Gmail API error"));
        assert!(error_string.contains(generic_message));
    }
}