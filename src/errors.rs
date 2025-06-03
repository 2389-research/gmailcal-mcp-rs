use reqwest;
use std::env;
use thiserror::Error;

/// Error type for configuration issues
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Environment error: {0}")]
    EnvError(#[from] env::VarError),
}

/// General application errors
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Cache is disabled")]
    CacheDisabled,

    #[error("IO Error: {0}")]
    IoError(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),
}

/// Error type for Gmail API operations
#[derive(Debug, Error)]
pub enum GmailApiError {
    #[error("Gmail API error: {0}")]
    ApiError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Message retrieval error: {0}")]
    MessageRetrievalError(String),

    #[error("Message format error: {0}")]
    MessageFormatError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Rate limit error: {0}")]
    RateLimitError(String),

    #[error("Token cache error: {0}")]
    CacheError(String),
}

/// Type alias for Gmail API results
pub type GmailResult<T> = std::result::Result<T, GmailApiError>;

/// Error type for People API operations
#[derive(Debug, Error)]
pub enum PeopleApiError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("People API error: {0}")]
    ApiError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Type alias for People API results
pub type PeopleResult<T> = std::result::Result<T, PeopleApiError>;

/// Error type for Calendar API operations
#[derive(Debug, Error)]
pub enum CalendarApiError {
    #[error("Calendar API error: {0}")]
    ApiError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Event retrieval error: {0}")]
    EventRetrievalError(String),

    #[error("Event format error: {0}")]
    EventFormatError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Rate limit error: {0}")]
    RateLimitError(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Type alias for Calendar API results
pub type CalendarResult<T> = std::result::Result<T, CalendarApiError>;

/// MCP error codes for different error scenarios
pub mod error_codes {
    // General errors
    pub const INTERNAL_ERROR: &str = "internal_error";
    pub const AUTHENTICATION_ERROR: &str = "authentication_error";
    pub const NOT_FOUND_ERROR: &str = "not_found_error";
    pub const INVALID_REQUEST: &str = "invalid_request";
    pub const RATE_LIMIT_ERROR: &str = "rate_limit_error";
    pub const CONFIG_ERROR: &str = "config_error";

    // Gmail specific errors
    pub const MESSAGE_NOT_FOUND: &str = "message_not_found";
    pub const DRAFT_NOT_FOUND: &str = "draft_not_found";
    pub const MESSAGE_FORMAT_ERROR: &str = "message_format_error";

    // Calendar specific errors
    pub const CALENDAR_NOT_FOUND: &str = "calendar_not_found";
    pub const EVENT_NOT_FOUND: &str = "event_not_found";
    pub const EVENT_FORMAT_ERROR: &str = "event_format_error";

    // People API specific errors
    pub const CONTACT_NOT_FOUND: &str = "contact_not_found";
}

// From implementations for error conversion
impl From<reqwest::Error> for GmailApiError {
    fn from(err: reqwest::Error) -> Self {
        GmailApiError::NetworkError(format!("Network error: {}", err))
    }
}

impl From<reqwest::Error> for PeopleApiError {
    fn from(err: reqwest::Error) -> Self {
        PeopleApiError::NetworkError(format!("Network error: {}", err))
    }
}

impl From<reqwest::Error> for CalendarApiError {
    fn from(err: reqwest::Error) -> Self {
        CalendarApiError::NetworkError(format!("Network error: {}", err))
    }
}
