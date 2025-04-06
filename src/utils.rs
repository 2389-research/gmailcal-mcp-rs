use crate::errors::GmailApiError;
use base64;
use log::{debug, error};
use mcp_attr::{Error as McpError, jsoncall::ErrorCode};
use serde_json;

// Error code constants for MCP errors
pub mod error_codes {
    /// Configuration related errors (environment variables, etc.)
    pub const CONFIG_ERROR: u32 = 1001;

    /// Authentication errors (tokens, OAuth, etc.)
    pub const AUTH_ERROR: u32 = 1002;

    /// API errors from Gmail
    pub const API_ERROR: u32 = 1003;

    /// Message format/missing field errors
    pub const MESSAGE_FORMAT_ERROR: u32 = 1005;

    /// General application errors for unspecified issues
    #[allow(dead_code)]
    pub const GENERAL_ERROR: u32 = 1000;

    // Map error codes to human-readable descriptions
    pub fn get_error_description(code: u32) -> &'static str {
        match code {
            CONFIG_ERROR => "Configuration Error: Missing or invalid environment variables required for Gmail authentication",
            AUTH_ERROR => "Authentication Error: Failed to authenticate with Gmail API using the provided credentials",
            API_ERROR => "Gmail API Error: The request to the Gmail API failed",
            MESSAGE_FORMAT_ERROR => "Message Format Error: The response from Gmail API has missing or invalid fields",
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
                            debug!("String \"{}\" not convertible to u32, using default {}", s, default);
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
    let bytes = base64::decode(data)
        .map_err(|e| format!("Error decoding base64: {}", e))?;
    
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
    let detailed_error = format!(
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
    match err {
        GmailApiError::ApiError(e) => {
            // Analyze the error message to provide more context
            let (code, detailed_msg) = if e.contains("quota")
                || e.contains("rate")
                || e.contains("limit")
            {
                (
                    error_codes::API_ERROR,
                    format!(
                        "Gmail API rate limit exceeded: {}. The server has made too many requests to the Gmail API. \
                        This typically happens when many requests are made in quick succession. \
                        Please try again in a few minutes.", 
                        e
                    )
                )
            } else if e.contains("network")
                || e.contains("connection")
                || e.contains("timeout")
            {
                (
                    error_codes::API_ERROR,
                    format!(
                        "Network error while connecting to Gmail API: {}. The server couldn't establish a \
                        connection to the Gmail API. This may be due to network issues or the Gmail API \
                        might be experiencing downtime.", 
                        e
                    )
                )
            } else if e.contains("authentication")
                || e.contains("auth")
                || e.contains("token")
            {
                (
                    error_codes::AUTH_ERROR,
                    format!(
                        "Gmail API authentication failed: {}. The OAuth token used to authenticate with \
                        Gmail may have expired or been revoked. Please check your credentials and try \
                        regenerating your refresh token.", 
                        e
                    )
                )
            } else if e.contains("format")
                || e.contains("missing field")
                || e.contains("parse")
            {
                (
                    error_codes::MESSAGE_FORMAT_ERROR,
                    format!(
                        "Gmail API response format error: {}. The API returned data in an unexpected format. \
                        This might be due to changes in the Gmail API or issues with specific messages.", 
                        e
                    )
                )
            } else if e.contains("not found") || e.contains("404") {
                (
                    error_codes::API_ERROR,
                    format!(
                        "Gmail API resource not found: {}. The requested message or resource doesn't exist \
                        or you don't have permission to access it. Please check the message ID and ensure \
                        it exists in your Gmail account.", 
                        e
                    )
                )
            } else {
                (
                    error_codes::API_ERROR,
                    format!(
                        "Unspecified Gmail API error: {}. An unexpected error occurred when communicating \
                        with the Gmail API. Please check the server logs for more details.", 
                        e
                    )
                )
            };

            to_mcp_error(&detailed_msg, code)
        }
        GmailApiError::AuthError(e) => {
            let detailed_msg = format!(
                "Gmail authentication error: {}. Failed to authenticate with the Gmail API using the provided \
                credentials. Please verify your client ID, client secret, and refresh token.", 
                e
            );
            to_mcp_error(&detailed_msg, error_codes::AUTH_ERROR)
        }
        GmailApiError::MessageRetrievalError(e) => {
            let detailed_msg = format!(
                "Message retrieval error: {}. Failed to retrieve the requested message from Gmail. \
                This may be due to the message being deleted, access permissions, or temporary Gmail API issues.", 
                e
            );
            to_mcp_error(&detailed_msg, error_codes::API_ERROR)
        }
        GmailApiError::MessageFormatError(e) => {
            let detailed_msg = format!(
                "Message format error: {}. The Gmail API returned a malformed message or one with missing required fields.", 
                e
            );
            to_mcp_error(&detailed_msg, error_codes::MESSAGE_FORMAT_ERROR)
        }
        GmailApiError::NetworkError(e) => {
            let detailed_msg = format!(
                "Network error: {}. The server couldn't establish a connection to the Gmail API. \
                This might be due to network configuration issues, outages, or firewall restrictions. \
                Please check your internet connection and server network configuration.", 
                e
            );
            to_mcp_error(&detailed_msg, error_codes::API_ERROR)
        }
        GmailApiError::RateLimitError(e) => {
            let detailed_msg = format!(
                "Rate limit error: {}. The Gmail API has rate-limited the server's requests. \
                This happens when too many requests are made in a short period of time. \
                The server will automatically retry after a cooldown period, but you may need to wait \
                or reduce the frequency of requests.", 
                e
            );
            to_mcp_error(&detailed_msg, error_codes::API_ERROR)
        }
    }
}