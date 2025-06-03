use crate::errors::GmailApiError;
use base64;
use log::{debug, error};
use mcp_attr::{jsoncall::ErrorCode, Error as McpError};
use serde_json;

// Error code constants for MCP errors
pub mod error_codes {
    /// General internal errors
    pub const GENERAL_ERROR: u32 = 1000;

    /// Configuration related errors (environment variables, etc.)
    pub const CONFIG_ERROR: u32 = 1001;

    /// Authentication errors (tokens, OAuth, etc.)
    pub const AUTH_ERROR: u32 = 1002;

    /// API errors from Gmail
    pub const API_ERROR: u32 = 1003;

    /// Message format/missing field errors
    pub const MESSAGE_FORMAT_ERROR: u32 = 1005;

    /// Network/HTTP request errors
    pub const NETWORK_ERROR: u32 = 1006;

    // Map error codes to human-readable descriptions
    pub fn get_error_description(code: u32) -> &'static str {
        match code {
            CONFIG_ERROR => "Configuration Error: Missing or invalid environment variables required for Gmail authentication",
            AUTH_ERROR => "Authentication Error: Failed to authenticate with Gmail API using the provided credentials",
            API_ERROR => "Gmail API Error: The request to the Gmail API failed",
            MESSAGE_FORMAT_ERROR => "Message Format Error: The response from Gmail API has missing or invalid fields",
            NETWORK_ERROR => "Network Error: Failed to make HTTP request or network connection issue",
            GENERAL_ERROR => "General Error: An unspecified error occurred in the Gmail MCP server",
            _ => "Unknown Error: An unclassified error occurred",
        }
    }

    // Get detailed troubleshooting steps for each error code
    pub fn get_troubleshooting_steps(code: u32) -> &'static str {
        match code {
            CONFIG_ERROR => "Check that you have correctly set the following environment variables: GMAIL_CLIENT_ID, GMAIL_CLIENT_SECRET, and GMAIL_REFRESH_TOKEN. These should be in your .env file or exported in your shell.",
            AUTH_ERROR => "Verify your OAuth credentials. Your refresh token may have expired or been revoked. Try generating new OAuth credentials and updating your environment variables.",
            API_ERROR => "The Gmail API request failed. This could be due to API rate limits, network issues, or an invalid request. Check your internet connection and review the specific error details.",
            MESSAGE_FORMAT_ERROR => "The Gmail API returned data in an unexpected format. This may be due to changes in the API or issues with specific messages. Try with a different message ID or update the server code.",
            NETWORK_ERROR => "Check your internet connection and ensure the target servers are accessible. This may be due to network issues, DNS problems, or server timeouts.",
            GENERAL_ERROR => "Review server logs for more details about what went wrong. Check for any recent changes to your code or environment.",
            _ => "Check the server logs for more specific error information. Ensure all dependencies are up to date.",
        }
    }
}

/// Parse maximum results parameter from JSON value
pub fn parse_max_results(value: Option<serde_json::Value>, default: u32) -> u32 {
    match value {
        Some(val) => {
            match val {
                serde_json::Value::Number(num) => {
                    // Handle number input
                    if let Some(n) = num.as_u64() {
                        // Ensure it fits in u32
                        if n <= u32::MAX as u64 {
                            n as u32
                        } else {
                            debug!("Number too large for u32, using default {}", default);
                            default
                        }
                    } else {
                        debug!("Number not convertible to u32, using default {}", default);
                        default
                    }
                }
                serde_json::Value::String(s) => {
                    // Handle string input
                    match s.parse::<u32>() {
                        Ok(n) => n,
                        Err(_) => {
                            debug!(
                                "String \"{}\" not convertible to u32, using default {}",
                                s, default
                            );
                            default
                        }
                    }
                }
                _ => {
                    debug!("Invalid type for max_results, using default {}", default);
                    default
                }
            }
        }
        None => default,
    }
}

/// Decode a base64 encoded string
pub fn decode_base64(data: &str) -> Result<String, String> {
    let bytes = base64::decode(data).map_err(|e| format!("Error decoding base64: {}", e))?;

    String::from_utf8(bytes).map_err(|e| format!("Error converting base64 to string: {}", e))
}

/// Encode data to base64 URL safe string
pub fn encode_base64_url_safe(data: &[u8]) -> String {
    base64::encode_config(data, base64::URL_SAFE)
}

/// Convert an error message and code to an MCP error
pub fn to_mcp_error(message: &str, code: u32) -> McpError {
    use error_codes::{get_error_description, get_troubleshooting_steps};

    // Get the generic description for this error code
    let description = get_error_description(code);

    // Get troubleshooting steps
    let steps = get_troubleshooting_steps(code);

    // Create a detailed error message with multiple parts
    let detailed_error =
        format!(
        "ERROR CODE {}: {}\n\nDETAILS: {}\n\nTROUBLESHOOTING: {}\n\nSERVER MESSAGE: {}",
        code, description, message, steps,
        "If the problem persists, contact the server administrator and reference this error code."
    );

    // Log the full error details
    error!(
        "Creating MCP error: {} (code: {})\n{}",
        message, code, detailed_error
    );

    // Create the MCP error with the detailed message
    McpError::new(ErrorCode(code as i64)).with_message(detailed_error, true)
}

/// Map Gmail API errors to MCP errors
pub fn map_gmail_error(err: GmailApiError) -> McpError {
    to_mcp_error(&err.to_string(), error_codes::API_ERROR)
}
