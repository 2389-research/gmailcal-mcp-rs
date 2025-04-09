/// Extended Utils Module Tests to improve coverage
///
/// This module contains additional tests for the utils.rs file
/// to target uncovered parts that weren't reached by the existing tests.
use mcp_gmailcal::errors::GmailApiError;
use mcp_gmailcal::utils::{
    decode_base64, encode_base64_url_safe, map_gmail_error, parse_max_results, to_mcp_error,
    error_codes::{get_error_description, get_troubleshooting_steps},
    error_codes::{AUTH_ERROR, API_ERROR, CONFIG_ERROR, MESSAGE_FORMAT_ERROR, GENERAL_ERROR}
};
use serde_json::json;

#[cfg(test)]
mod utils_extended_tests {
    use super::*;

    #[test]
    fn test_parse_max_results_edge_cases() {
        // Test for line 59 - when the number is exactly the u32::MAX value
        let max_u32_value = json!(u32::MAX);
        let result = parse_max_results(Some(max_u32_value), 100);
        assert_eq!(result, u32::MAX);
        
        // Test for line 76 - string parser with debug log when conversion fails
        let invalid_string = json!("not_a_number");
        let result = parse_max_results(Some(invalid_string), 200);
        assert_eq!(result, 200);
    }

    #[test]
    fn test_map_gmail_error_uncovered_branches() {
        // Test for line 138-201 - covering the API error classification branches
        
        // Test for line 201 - Ensuring it's called at the end of the match block
        let api_error = map_gmail_error(GmailApiError::ApiError("standard api error".to_string()));
        let debug_str = format!("{:?}", api_error);
        assert!(debug_str.contains(&format!("{}", API_ERROR)));
        
        // Test for line 125 - test specific error message construction
        let auth_error_message = "auth test error";
        let auth_error = map_gmail_error(GmailApiError::AuthError(auth_error_message.to_string()));
        let debug_str = format!("{:?}", auth_error);
        assert!(debug_str.contains(&format!("{}", AUTH_ERROR)));
        assert!(debug_str.contains(auth_error_message));
    }

    #[test]
    fn test_to_mcp_error_construction() {
        // Test for detailed error message construction (line 115-121)
        let error_msg = "Test error message";
        let error = to_mcp_error(error_msg, CONFIG_ERROR);
        
        // The full formatted string can be accessed through Debug representation
        let debug_str = format!("{:?}", error);
        
        // Verify essential components are included
        assert!(debug_str.contains(&CONFIG_ERROR.to_string()));
        assert!(debug_str.contains(error_msg));
        assert!(debug_str.contains("ERROR CODE"));
        assert!(debug_str.contains("DETAILS"));
        assert!(debug_str.contains("TROUBLESHOOTING"));
        assert!(debug_str.contains("SERVER MESSAGE"));
        
        // Verify error code description was included
        let desc = get_error_description(CONFIG_ERROR);
        assert!(debug_str.contains(desc));
        
        // Verify troubleshooting steps were included
        let steps = get_troubleshooting_steps(CONFIG_ERROR);
        assert!(debug_str.contains(steps));
    }
}