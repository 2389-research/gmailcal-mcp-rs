use crate::config::{get_token_expiry_seconds, Config, OAUTH_TOKEN_URL};
use crate::errors::{GmailApiError, GmailResult};
use log::{debug, error};
use reqwest::Client;
use serde::Deserialize;
use std::time::{Duration, SystemTime};

// Alias for backward compatibility within this module
type Result<T> = GmailResult<T>;

// Token response for OAuth2
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    #[serde(default)]
    #[allow(dead_code)]
    token_type: String,
}

// OAuth token manager
#[derive(Debug, Clone)]
pub struct TokenManager {
    access_token: String,
    expiry: SystemTime,
    refresh_token: String,
    client_id: String,
    client_secret: String,
}

impl TokenManager {
    pub fn new(config: &Config) -> Self {
        let expiry = if config.access_token.is_some() {
            // If we have an initial access token, use the configurable default
            SystemTime::now() + Duration::from_secs(get_token_expiry_seconds())
        } else {
            // Otherwise set expiry to now to force a refresh
            SystemTime::now()
        };

        Self {
            access_token: config.access_token.clone().unwrap_or_default(),
            expiry,
            refresh_token: config.refresh_token.clone(),
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
        }
    }

    pub async fn get_token(&mut self, client: &Client) -> Result<String> {
        // Debug log the initial state
        debug!(
            "Token status check - have token: {}, valid: {}",
            !self.access_token.is_empty(),
            SystemTime::now() < self.expiry
        );

        // Check if current token is still valid
        if !self.access_token.is_empty() && SystemTime::now() < self.expiry {
            debug!("Using existing token");
            return Ok(self.access_token.clone());
        }

        debug!("OAuth token expired or not set, refreshing");

        // Refresh the token
        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("refresh_token", self.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ];

        // Log request details for troubleshooting (but hide credentials)
        debug!("Requesting token from {}", OAUTH_TOKEN_URL);
        // Securely log truncated credential information - never log full credentials
        if log::log_enabled!(log::Level::Debug) {
            let client_id_trunc = if self.client_id.len() > 8 {
                format!(
                    "{}...{}",
                    &self.client_id[..4],
                    &self.client_id[self.client_id.len().saturating_sub(4)..]
                )
            } else {
                "<short-id>".to_string()
            };

            let refresh_token_trunc = if self.refresh_token.len() > 8 {
                format!("{}...", &self.refresh_token[..4])
            } else {
                "<short-token>".to_string()
            };

            debug!("Using client_id: {} (truncated)", client_id_trunc);
            debug!(
                "Using refresh_token starting with: {} (truncated)",
                refresh_token_trunc
            );
        }

        let response = client
            .post(OAUTH_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| GmailApiError::NetworkError(e.to_string()))?;

        let status = response.status();
        debug!("Token response status: {}", status);

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<no response body>".to_string());

            error!(
                "Token refresh failed. Status: {}, Error: {}",
                status, error_text
            );
            return Err(GmailApiError::AuthError(format!(
                "Failed to refresh token. Status: {}, Error: {}",
                status, error_text
            )));
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| GmailApiError::ApiError(format!("Failed to get token response: {}", e)))?;

        debug!("Token response received, parsing JSON");

        let token_data: TokenResponse = serde_json::from_str(&response_text).map_err(|e| {
            error!(
                "Failed to parse token response: {}. Response: {}",
                e, response_text
            );
            GmailApiError::ApiError(format!("Failed to parse token response: {}", e))
        })?;

        // Update token and expiry
        self.access_token = token_data.access_token.clone();
        // Set expiry to slightly less than the actual expiry to be safe
        let expires_in = token_data.expires_in.saturating_sub(60); // 1 minute buffer
        self.expiry = SystemTime::now() + Duration::from_secs(expires_in);

        debug!(
            "Token refreshed successfully, valid for {} seconds",
            expires_in
        );
        // Securely log truncated token - never log the full token
        if log::log_enabled!(log::Level::Debug) {
            let token_trunc = if self.access_token.len() > 10 {
                format!(
                    "{}...{}",
                    &self.access_token[..4],
                    &self.access_token[self.access_token.len().saturating_sub(4)..]
                )
            } else {
                "<short-token>".to_string()
            };
            debug!("Token (truncated): {}", token_trunc);
        };

        Ok(self.access_token.clone())
    }
}
