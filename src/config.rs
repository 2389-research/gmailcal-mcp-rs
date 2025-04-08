use crate::errors::ConfigError;
use dotenv::dotenv;
use log::debug;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
    pub access_token: Option<String>,
    
    /// Number of seconds before an access token expires that it should be refreshed.
    /// Allows for proactive refresh to prevent working with nearly-expired tokens.
    /// Can be configured with TOKEN_REFRESH_THRESHOLD_SECONDS environment variable.
    pub token_refresh_threshold: u64,
    
    /// Buffer time in seconds subtracted from the token's expiration time to ensure
    /// we don't use tokens too close to their expiry time. Provides a safety margin.
    /// Can be configured with TOKEN_EXPIRY_BUFFER_SECONDS environment variable.
    pub token_expiry_buffer: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        // Attempt to load .env file if present
        // If DOTENV_PATH is set, use that path, otherwise use default
        if let Ok(path) = std::env::var("DOTENV_PATH") {
            let _ = dotenv::from_path(path);
        } else {
            let _ = dotenv();
        }

        debug!("Loading Gmail OAuth configuration from environment");

        // Get required variables
        let client_id = env::var("GMAIL_CLIENT_ID")
            .map_err(|_| ConfigError::MissingEnvVar("GMAIL_CLIENT_ID".to_string()))?;

        let client_secret = env::var("GMAIL_CLIENT_SECRET")
            .map_err(|_| ConfigError::MissingEnvVar("GMAIL_CLIENT_SECRET".to_string()))?;

        let refresh_token = env::var("GMAIL_REFRESH_TOKEN")
            .map_err(|_| ConfigError::MissingEnvVar("GMAIL_REFRESH_TOKEN".to_string()))?;

        // Get optional access token
        let access_token = env::var("GMAIL_ACCESS_TOKEN").ok();
        
        // Get token expiry configuration with defaults
        let token_refresh_threshold = get_token_refresh_threshold_seconds();
        let token_expiry_buffer = get_token_expiry_buffer_seconds();
        
        debug!("OAuth configuration loaded successfully");
        debug!("Token refresh threshold: {} seconds", token_refresh_threshold);
        debug!("Token expiry buffer: {} seconds", token_expiry_buffer);

        Ok(Config {
            client_id,
            client_secret,
            refresh_token,
            access_token,
            token_refresh_threshold,
            token_expiry_buffer,
        })
    }
}

// API URL constants
pub const GMAIL_API_BASE_URL: &str = "https://gmail.googleapis.com/gmail/v1";
pub const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

// Configuration utility functions

/// Returns the total token expiry time in seconds.
/// 
/// This value controls how long the application considers the token valid.
/// Default is 3540 seconds (59 minutes) if not configured.
/// 
/// Environment variable: TOKEN_EXPIRY_SECONDS
pub fn get_token_expiry_seconds() -> u64 {
    std::env::var("TOKEN_EXPIRY_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(3540) // Default 59 minutes if not configured
}

/// Returns the buffer time in seconds subtracted from token expiry time.
/// 
/// This buffer ensures we don't use tokens that are too close to expiry.
/// It's subtracted from the token's actual expiry time to create a safety margin.
/// Default is 60 seconds (1 minute) if not configured.
/// 
/// Environment variable: TOKEN_EXPIRY_BUFFER_SECONDS
pub fn get_token_expiry_buffer_seconds() -> u64 {
    std::env::var("TOKEN_EXPIRY_BUFFER_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60) // Default 1 minute if not configured
}

/// Returns the threshold in seconds before token expiry that a refresh should be triggered.
/// 
/// This controls how soon before a token expires that the application should proactively
/// refresh it. This prevents using nearly-expired tokens which might expire during operations.
/// Default is 300 seconds (5 minutes) if not configured.
/// 
/// Environment variable: TOKEN_REFRESH_THRESHOLD_SECONDS
pub fn get_token_refresh_threshold_seconds() -> u64 {
    std::env::var("TOKEN_REFRESH_THRESHOLD_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(300) // Default 5 minutes if not configured
}
