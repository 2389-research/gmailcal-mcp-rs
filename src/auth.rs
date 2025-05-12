use crate::config::{
    get_token_expiry_buffer_seconds, get_token_expiry_seconds, get_token_refresh_threshold_seconds,
    Config, OAUTH_TOKEN_URL,
};
use crate::errors::{GmailApiError, GmailResult};
use log::{debug, error, info, warn};
use reqwest::Client;
use serde::Deserialize;
use std::cmp::min;
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

use crate::token_cache::{TokenCache, TokenCacheConfig};

// OAuth token manager
#[derive(Debug, Clone)]
pub struct TokenManager {
    access_token: String,
    expiry: SystemTime,
    refresh_token: String,
    client_id: String,
    client_secret: String,
    retry_count: u8,
    max_retries: u8,
    base_retry_delay_ms: u64,
    cache: Option<TokenCache>,
}

impl TokenManager {
    // Test-only accessors - used by the test extension trait
    #[cfg(test)]
    pub(crate) fn get_access_token(&self) -> &str {
        &self.access_token
    }

    #[cfg(test)]
    pub(crate) fn get_expiry(&self) -> SystemTime {
        self.expiry
    }

    #[cfg(test)]
    pub(crate) fn get_refresh_threshold(&self) -> u64 {
        get_token_refresh_threshold_seconds()
    }

    #[cfg(test)]
    pub(crate) fn get_expiry_buffer(&self) -> u64 {
        get_token_expiry_buffer_seconds()
    }

    #[cfg(test)]
    pub(crate) fn get_retry_count(&self) -> u8 {
        self.retry_count
    }

    #[cfg(test)]
    pub(crate) fn get_max_retries(&self) -> u8 {
        self.max_retries
    }

    #[cfg(test)]
    pub(crate) fn get_base_retry_delay_ms(&self) -> u64 {
        self.base_retry_delay_ms
    }

    #[cfg(test)]
    pub(crate) fn set_access_token(&mut self, token: String) {
        self.access_token = token;
    }

    #[cfg(test)]
    pub(crate) fn set_expiry(&mut self, expiry: SystemTime) {
        self.expiry = expiry;
    }

    #[cfg(test)]
    pub(crate) fn increment_retry_count(&mut self) {
        self.retry_count += 1;
    }

    #[cfg(test)]
    pub(crate) fn set_retry_count(&mut self, count: u8) {
        self.retry_count = count;
    }

    #[cfg(test)]
    pub(crate) fn set_max_retries(&mut self, max: u8) {
        self.max_retries = max;
    }

    #[cfg(test)]
    pub(crate) fn reset_retry_count(&mut self) {
        self.retry_count = 0;
    }
    pub fn new(config: &Config) -> Self {
        // Initialize token cache if enabled
        let cache = match TokenCacheConfig::from_env() {
            Ok(cache_config) => {
                if cache_config.enabled {
                    match TokenCache::new(cache_config) {
                        Ok(cache) => {
                            debug!("Token cache initialized successfully");
                            Some(cache)
                        }
                        Err(e) => {
                            error!("Failed to initialize token cache: {}", e);
                            None
                        }
                    }
                } else {
                    debug!("Token caching is disabled");
                    None
                }
            }
            Err(e) => {
                error!("Failed to load token cache configuration: {}", e);
                None
            }
        };

        // Try to load token from cache first
        let (access_token, expiry, loaded_from_cache) = if let Some(cache) = &cache {
            match cache.load_token() {
                Ok(Some(cached_token)) => {
                    if cache.is_token_valid(&cached_token) {
                        info!("Loaded valid token from cache");
                        let expiry_system_time = SystemTime::UNIX_EPOCH
                            + Duration::from_secs(cached_token.expiry_timestamp);
                        (cached_token.access_token, expiry_system_time, true)
                    } else {
                        debug!("Cached token exists but is expired or nearly expired");
                        let default_token = config.access_token.clone().unwrap_or_default();
                        let default_expiry = if config.access_token.is_some() {
                            SystemTime::now() + Duration::from_secs(get_token_expiry_seconds())
                        } else {
                            SystemTime::now() // Force refresh
                        };
                        (default_token, default_expiry, false)
                    }
                }
                Ok(None) => {
                    debug!("No cached token found");
                    let default_token = config.access_token.clone().unwrap_or_default();
                    let default_expiry = if config.access_token.is_some() {
                        SystemTime::now() + Duration::from_secs(get_token_expiry_seconds())
                    } else {
                        SystemTime::now() // Force refresh
                    };
                    (default_token, default_expiry, false)
                }
                Err(e) => {
                    warn!("Error loading token from cache: {}", e);
                    let default_token = config.access_token.clone().unwrap_or_default();
                    let default_expiry = if config.access_token.is_some() {
                        SystemTime::now() + Duration::from_secs(get_token_expiry_seconds())
                    } else {
                        SystemTime::now() // Force refresh
                    };
                    (default_token, default_expiry, false)
                }
            }
        } else {
            // No cache, use config values
            let default_token = config.access_token.clone().unwrap_or_default();
            let default_expiry = if config.access_token.is_some() {
                SystemTime::now() + Duration::from_secs(get_token_expiry_seconds())
            } else {
                SystemTime::now() // Force refresh
            };
            (default_token, default_expiry, false)
        };

        debug!(
            "Creating TokenManager with refresh threshold: {}s, expiry buffer: {}s",
            config.token_refresh_threshold, config.token_expiry_buffer
        );

        if loaded_from_cache {
            debug!("Using cached token, expires at {:?}", expiry);
        } else if !access_token.is_empty() {
            debug!("Using token from config, expires at {:?}", expiry);
        } else {
            debug!("No valid token available, will refresh on first use");
        }

        Self {
            access_token,
            expiry,
            refresh_token: config.refresh_token.clone(),
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            retry_count: 0,
            max_retries: 5,            // Default maximum retries
            base_retry_delay_ms: 1000, // Start with 1 second delay
            cache,
        }
    }

    // Calculate time until token expiry in seconds (can be negative if expired)
    fn time_until_expiry(&self) -> i64 {
        match self.expiry.duration_since(SystemTime::now()) {
            Ok(duration) => duration.as_secs() as i64,
            Err(_) => -1, // Token has expired
        }
    }

    // Check if token needs refresh based on refresh threshold
    fn needs_refresh(&self) -> bool {
        let refresh_threshold = get_token_refresh_threshold_seconds();
        let seconds_until_expiry = self.time_until_expiry();

        if seconds_until_expiry < 0 {
            debug!("Token has expired");
            return true;
        }

        if seconds_until_expiry < refresh_threshold as i64 {
            debug!(
                "Token will expire in {} seconds (refresh threshold: {} seconds), needs refresh",
                seconds_until_expiry, refresh_threshold
            );
            return true;
        }

        debug!(
            "Token valid for {} more seconds (refresh threshold: {} seconds), no refresh needed",
            seconds_until_expiry, refresh_threshold
        );
        false
    }

    // Reset retry counter after successful operation - accessible both from tests and internal code
    #[cfg(not(test))]
    fn reset_retry_count(&mut self) {
        if self.retry_count > 0 {
            debug!("Resetting retry count from {} to 0", self.retry_count);
            self.retry_count = 0;
        }
    }

    // Calculate exponential backoff delay based on retry count
    fn get_backoff_delay(&self) -> Duration {
        if self.retry_count == 0 {
            return Duration::from_millis(0);
        }

        // Calculate delay with exponential backoff: base_delay * 2^(retry_count-1)
        // Example: 1000ms base -> 1s, 2s, 4s, 8s, 16s
        let exponent = min(self.retry_count - 1, 16); // Prevent potential overflow
        let delay_ms = self.base_retry_delay_ms * (1u64 << exponent);

        // Cap maximum delay at 64 seconds
        let capped_delay_ms = min(delay_ms, 64_000);

        debug!(
            "Backoff delay for retry {}: {}ms",
            self.retry_count, capped_delay_ms
        );
        Duration::from_millis(capped_delay_ms)
    }

    pub async fn get_token(&mut self, client: &Client) -> Result<String> {
        // Log token expiration details
        let seconds_until_expiry = self.time_until_expiry();
        let has_token = !self.access_token.is_empty();

        debug!(
            "Token status check - have token: {}, expires in: {} seconds",
            has_token, seconds_until_expiry
        );

        // Check if current token is still valid and not near expiration
        if has_token && !self.needs_refresh() {
            debug!("Using existing token, not near expiration");
            return Ok(self.access_token.clone());
        }

        // Token is missing, expired, or near expiration
        if has_token {
            debug!("OAuth token expiring soon, proactively refreshing");
        } else {
            debug!("OAuth token not set, obtaining new token");
        }

        // Apply exponential backoff if retrying
        if self.retry_count > 0 {
            let backoff_delay = self.get_backoff_delay();
            warn!(
                "Applying exponential backoff delay of {}ms for retry attempt {}",
                backoff_delay.as_millis(),
                self.retry_count
            );
            tokio::time::sleep(backoff_delay).await;
        }

        // Check if we've exceeded max retries
        if self.retry_count >= self.max_retries && self.retry_count > 0 {
            error!(
                "Maximum retry attempts ({}) exceeded for token refresh",
                self.max_retries
            );
            return Err(GmailApiError::AuthError(
                "Maximum retry attempts exceeded for token refresh".to_string(),
            ));
        }

        // Increment retry counter
        self.retry_count += 1;
        if self.retry_count > 1 {
            debug!("Retry attempt {} of {}", self.retry_count, self.max_retries);
        }

        // Prepare token refresh request
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

        // Send token refresh request
        let response = match client.post(OAUTH_TOKEN_URL).form(&params).send().await {
            Ok(resp) => resp,
            Err(e) => {
                warn!("Network error during token refresh: {}", e);
                return Err(GmailApiError::NetworkError(e.to_string()));
            }
        };

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

            // Some errors shouldn't be retried (e.g., invalid_grant)
            if error_text.contains("invalid_grant") {
                error!("Invalid grant error detected, not retrying");
                self.retry_count = self.max_retries; // Prevent further retries
            }

            return Err(GmailApiError::AuthError(format!(
                "Failed to refresh token. Status: {}, Error: {}",
                status, error_text
            )));
        }

        let response_text = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                warn!("Failed to get token response text: {}", e);
                return Err(GmailApiError::ApiError(format!(
                    "Failed to get token response: {}",
                    e
                )));
            }
        };

        debug!("Token response received, parsing JSON");

        let token_data: TokenResponse = match serde_json::from_str(&response_text) {
            Ok(data) => data,
            Err(e) => {
                error!(
                    "Failed to parse token response: {}. Response: {}",
                    e, response_text
                );
                return Err(GmailApiError::ApiError(format!(
                    "Failed to parse token response: {}",
                    e
                )));
            }
        };

        // Update token and expiry
        self.access_token = token_data.access_token.clone();

        // Apply buffer to expiry time from config
        let buffer = get_token_expiry_buffer_seconds();
        let expires_in = token_data.expires_in.saturating_sub(buffer);
        self.expiry = SystemTime::now() + Duration::from_secs(expires_in);

        // Save token to cache if enabled
        if let Some(cache) = &self.cache {
            match cache.save_token(&self.access_token, &self.refresh_token, self.expiry) {
                Ok(_) => debug!("Token successfully saved to cache"),
                Err(e) => warn!("Failed to save token to cache: {}", e),
            }
        }

        // Reset retry counter after success
        self.reset_retry_count();

        // Calculate when we'll need to refresh this token
        let refresh_threshold = get_token_refresh_threshold_seconds();
        let effective_lifetime = expires_in.saturating_sub(refresh_threshold);

        info!(
            "Token refreshed successfully, valid for {} seconds (with {}s buffer)",
            expires_in, buffer
        );
        debug!(
            "Token will be refreshed after {} seconds (threshold: {}s)",
            effective_lifetime, refresh_threshold
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
