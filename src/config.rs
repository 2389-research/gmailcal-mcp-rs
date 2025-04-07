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

        debug!("OAuth configuration loaded successfully");

        Ok(Config {
            client_id,
            client_secret,
            refresh_token,
            access_token,
        })
    }
}

// API URL constants
pub const GMAIL_API_BASE_URL: &str = "https://gmail.googleapis.com/gmail/v1";
pub const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

// Configuration utility functions
pub fn get_token_expiry_seconds() -> u64 {
    std::env::var("TOKEN_EXPIRY_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(600) // Default 10 minutes if not configured
}
