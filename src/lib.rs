pub use crate::config::Config;
pub use crate::gmail_api::EmailMessage;
pub use crate::logging::setup_logging;
pub use crate::prompts::*;
/// Gmail MCP Server Implementation
///
/// This crate provides an MCP (Model Completion Protocol) server for Gmail,
/// allowing Claude to read emails from a Gmail account.
///
/// # Features
///
/// - List emails from inbox
/// - Search emails using Gmail search queries
/// - Get details for a specific email
/// - List email labels
/// - Check connection status
///
/// # Testing
///
/// The crate includes unit tests for internal functions and integration tests
/// for testing the MCP commands. Future improvements could include more
/// sophisticated mocking of the Gmail API and more comprehensive tests.
///
// Re-export key types for use in tests
pub use crate::server::GmailServer;

// Module for centralized configuration
pub mod config {
    use dotenv::dotenv;
    use log::debug;
    use std::env;
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum ConfigError {
        #[error("Missing environment variable: {0}")]
        MissingEnvVar(String),

        #[error("Environment error: {0}")]
        EnvError(#[from] env::VarError),
    }

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
            let _ = dotenv();

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
}

// Direct Gmail API implementation
pub mod gmail_api {
    use crate::config::Config;
    use log::{debug, error, info};
    use reqwest::Client;
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use std::time::{Duration, SystemTime};
    use thiserror::Error;

    const GMAIL_API_BASE_URL: &str = "https://gmail.googleapis.com/gmail/v1";
    const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

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
    struct TokenManager {
        access_token: String,
        expiry: SystemTime,
        refresh_token: String,
        client_id: String,
        client_secret: String,
    }

    impl TokenManager {
        fn new(config: &Config) -> Self {
            let expiry = if config.access_token.is_some() {
                // If we have an initial access token, set expiry to 10 minutes from now
                // This is conservative but ensures we'll refresh soon if needed
                SystemTime::now() + Duration::from_secs(600)
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

        async fn get_token(&mut self, client: &Client) -> Result<String> {
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
            debug!(
                "Using client_id: {}...{} (truncated)",
                &self.client_id[..4],
                &self.client_id[self.client_id.len().saturating_sub(4)..]
            );
            debug!(
                "Using refresh_token starting with: {}... (truncated)",
                if self.refresh_token.len() > 8 {
                    &self.refresh_token[..8]
                } else {
                    "(token too short)"
                }
            );

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

            let response_text = response.text().await.map_err(|e| {
                GmailApiError::ApiError(format!("Failed to get token response: {}", e))
            })?;

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
            debug!(
                "Token starts with: {}... (truncated)",
                if self.access_token.len() > 10 {
                    &self.access_token[..10]
                } else {
                    "(token too short)"
                }
            );

            Ok(self.access_token.clone())
        }
    }

    // Email message model
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct EmailMessage {
        pub id: String,
        pub thread_id: String,
        pub subject: Option<String>,
        pub from: Option<String>,
        pub to: Option<String>,
        pub date: Option<String>,
        pub snippet: Option<String>,
        pub body_text: Option<String>,
        pub body_html: Option<String>,
    }

    // Gmail API error types
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
    }

    pub type Result<T> = std::result::Result<T, GmailApiError>;

    pub struct GmailService {
        client: Client,
        token_manager: TokenManager,
    }

    impl GmailService {
        pub fn new(config: &Config) -> Result<Self> {
            debug!("Creating new GmailService with config");

            // Create HTTP client with reasonable timeouts
            debug!("Creating HTTP client with timeouts");
            let client = Client::builder()
                .timeout(Duration::from_secs(60)) // Longer timeout for Gmail API
                .connect_timeout(Duration::from_secs(30))
                .pool_idle_timeout(Duration::from_secs(90))
                .pool_max_idle_per_host(5)
                .user_agent("mcp-gmailcal/0.1.0")
                .build()
                .map_err(|e| {
                    error!("Failed to create HTTP client: {}", e);
                    GmailApiError::NetworkError(format!("Failed to create HTTP client: {}", e))
                })?;

            debug!("HTTP client created successfully");

            let token_manager = TokenManager::new(config);

            Ok(Self {
                client,
                token_manager,
            })
        }

        // Helper function to make authenticated requests to Gmail API
        async fn request<T: for<'de> Deserialize<'de>>(
            &mut self,
            method: reqwest::Method,
            endpoint: &str,
            query: Option<&[(&str, &str)]>,
        ) -> Result<T> {
            // Get valid access token
            let token = self.token_manager.get_token(&self.client).await?;

            let url = format!("{}{}", GMAIL_API_BASE_URL, endpoint);
            debug!("Making request to: {}", url);

            // Build request with authorization header
            debug!("Making authenticated request to {}", url);
            let mut req_builder = self
                .client
                .request(method, &url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Accept", "application/json")
                .header("User-Agent", "mcp-gmailcal/0.1.0");

            // Add query parameters if provided
            if let Some(q) = query {
                req_builder = req_builder.query(q);
            }

            // Send request
            debug!("Sending request to Gmail API");
            let response = req_builder.send().await.map_err(|e| {
                error!("Network error sending request: {}", e);
                GmailApiError::NetworkError(e.to_string())
            })?;

            debug!("Response received with status: {}", response.status());

            // Handle response status
            let status = response.status();
            if !status.is_success() {
                let status_code = status.as_u16();
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<no response body>".to_string());

                // Map common error codes to appropriate error types
                return match status_code {
                    401 | 403 => Err(GmailApiError::AuthError(format!(
                        "Authentication failed. Status: {}, Error: {}",
                        status, error_text
                    ))),
                    404 => Err(GmailApiError::MessageRetrievalError(format!(
                        "Resource not found. Status: {}, Error: {}",
                        status, error_text
                    ))),
                    429 => Err(GmailApiError::RateLimitError(format!(
                        "Rate limit exceeded. Status: {}, Error: {}",
                        status, error_text
                    ))),
                    _ => Err(GmailApiError::ApiError(format!(
                        "API request failed. Status: {}, Error: {}",
                        status, error_text
                    ))),
                };
            }

            // Parse JSON response
            response.json::<T>().await.map_err(|e| {
                GmailApiError::MessageFormatError(format!("Failed to parse response: {}", e))
            })
        }

        // Helper function to make a request and return the raw JSON response
        async fn request_raw(
            &mut self,
            method: reqwest::Method,
            endpoint: &str,
            query: Option<&[(&str, &str)]>,
        ) -> Result<String> {
            // Get valid access token
            let token = self.token_manager.get_token(&self.client).await?;

            let url = format!("{}{}", GMAIL_API_BASE_URL, endpoint);
            debug!("Making raw request to: {}", url);

            // Build request with authorization header
            debug!("Making raw authenticated request to {}", url);
            let mut req_builder = self
                .client
                .request(method, &url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Accept", "application/json")
                .header("User-Agent", "mcp-gmailcal/0.1.0");

            // Add query parameters if provided
            if let Some(q) = query {
                req_builder = req_builder.query(q);
            }

            // Send request
            debug!("Sending raw request to Gmail API");
            let response = req_builder.send().await.map_err(|e| {
                error!("Network error sending raw request: {}", e);
                GmailApiError::NetworkError(e.to_string())
            })?;

            debug!("Raw response received with status: {}", response.status());

            // Handle response status
            let status = response.status();
            if !status.is_success() {
                let status_code = status.as_u16();
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<no response body>".to_string());

                // Map common error codes to appropriate error types
                return match status_code {
                    401 | 403 => Err(GmailApiError::AuthError(format!(
                        "Authentication failed. Status: {}, Error: {}",
                        status, error_text
                    ))),
                    404 => Err(GmailApiError::MessageRetrievalError(format!(
                        "Resource not found. Status: {}, Error: {}",
                        status, error_text
                    ))),
                    429 => Err(GmailApiError::RateLimitError(format!(
                        "Rate limit exceeded. Status: {}, Error: {}",
                        status, error_text
                    ))),
                    _ => Err(GmailApiError::ApiError(format!(
                        "API request failed. Status: {}, Error: {}",
                        status, error_text
                    ))),
                };
            }

            // Get raw JSON as string
            debug!("Reading response body");
            let json_text = response.text().await.map_err(|e| {
                error!("Failed to get response body: {}", e);
                GmailApiError::NetworkError(format!("Failed to get response body: {}", e))
            })?;

            // Log a preview of the response
            let preview = if json_text.len() > 200 {
                format!(
                    "{}... (truncated, total size: {} bytes)",
                    &json_text[..200],
                    json_text.len()
                )
            } else {
                json_text.clone()
            };
            debug!("Raw response body: {}", preview);

            // Format JSON for pretty printing
            match serde_json::from_str::<Value>(&json_text) {
                Ok(value) => {
                    debug!("Successfully parsed response as JSON");
                    serde_json::to_string_pretty(&value).map_err(|e| {
                        error!("Failed to format JSON: {}", e);
                        GmailApiError::MessageFormatError(format!("Failed to format JSON: {}", e))
                    })
                }
                Err(e) => {
                    error!("Failed to parse response as JSON: {}", e);
                    debug!("Returning raw response text");
                    Ok(json_text) // Return as-is if not valid JSON
                }
            }
        }

        /// Get a message by ID and return as raw JSON
        pub async fn get_message_raw(&mut self, message_id: &str) -> Result<String> {
            debug!("Getting raw message with ID: {}", message_id);

            // Log request details
            let request_details = format!(
                "Request details: User ID: 'me', Message ID: '{}', Format: 'full'",
                message_id
            );
            info!("{}", request_details);

            // Build query params for full message format
            let query = [("format", "full")];

            // Execute request
            let endpoint = format!("/users/me/messages/{}", message_id);
            self.request_raw(reqwest::Method::GET, &endpoint, Some(&query))
                .await
        }

        /// List messages and return raw JSON response
        pub async fn list_messages_raw(
            &mut self,
            max_results: u32,
            query: Option<&str>,
        ) -> Result<String> {
            debug!(
                "Listing raw messages with max_results={}, query={:?}",
                max_results, query
            );

            // Create string representation of max_results
            let max_results_str = max_results.to_string();

            // Execute request
            let endpoint = "/users/me/messages";

            // Handle query parameter differently to avoid lifetime issues
            if let Some(q) = query {
                // Use separate array for each case
                let params = [("maxResults", max_results_str.as_str()), ("q", q)];
                self.request_raw(reqwest::Method::GET, endpoint, Some(&params))
                    .await
            } else {
                let params = [("maxResults", max_results_str.as_str())];
                self.request_raw(reqwest::Method::GET, endpoint, Some(&params))
                    .await
            }
        }

        /// Get message details with all metadata and content
        pub async fn get_message_details(&mut self, message_id: &str) -> Result<EmailMessage> {
            use base64;

            // First get the full message
            let message_json = self.get_message_raw(message_id).await?;

            // Parse the JSON
            let parsed: serde_json::Value = serde_json::from_str(&message_json).map_err(|e| {
                GmailApiError::MessageFormatError(format!("Failed to parse message JSON: {}", e))
            })?;

            // Extract the basic message data
            let id = parsed["id"]
                .as_str()
                .ok_or_else(|| {
                    GmailApiError::MessageFormatError("Message missing 'id' field".to_string())
                })?
                .to_string();

            let thread_id = parsed["threadId"]
                .as_str()
                .ok_or_else(|| {
                    GmailApiError::MessageFormatError(
                        "Message missing 'threadId' field".to_string(),
                    )
                })?
                .to_string();

            // Extract metadata
            let mut subject = None;
            let mut from = None;
            let mut to = None;
            let mut date = None;
            let mut snippet = None;
            let mut body_text = None;
            let mut body_html = None;

            // Extract snippet if available
            if let Some(s) = parsed.get("snippet").and_then(|s| s.as_str()) {
                snippet = Some(s.to_string());
            }

            // Process payload to extract headers and body parts
            if let Some(payload) = parsed.get("payload") {
                // Extract headers
                if let Some(headers) = payload.get("headers").and_then(|h| h.as_array()) {
                    for header in headers {
                        if let (Some(name), Some(value)) = (
                            header.get("name").and_then(|n| n.as_str()),
                            header.get("value").and_then(|v| v.as_str()),
                        ) {
                            match name {
                                "Subject" => subject = Some(value.to_string()),
                                "From" => from = Some(value.to_string()),
                                "To" => to = Some(value.to_string()),
                                "Date" => date = Some(value.to_string()),
                                _ => {}
                            }
                        }
                    }
                }

                // Extract message body parts
                if let Some(parts) = payload.get("parts").and_then(|p| p.as_array()) {
                    // Process each part
                    for part in parts {
                        if let Some(mime_type) = part.get("mimeType").and_then(|m| m.as_str()) {
                            // Handle text parts
                            if mime_type == "text/plain" || mime_type == "text/html" {
                                if let Some(body) = part.get("body") {
                                    if let Some(data) = body.get("data").and_then(|d| d.as_str()) {
                                        // Decode base64
                                        if let Ok(decoded) =
                                            base64::decode(data.replace('-', "+").replace('_', "/"))
                                        {
                                            if let Ok(text) = String::from_utf8(decoded) {
                                                match mime_type {
                                                    "text/plain" => body_text = Some(text),
                                                    "text/html" => body_html = Some(text),
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Check for body directly in payload (for simple messages)
                if body_text.is_none() && body_html.is_none() {
                    if let Some(body) = payload.get("body") {
                        if let Some(data) = body.get("data").and_then(|d| d.as_str()) {
                            // Decode base64
                            if let Ok(decoded) =
                                base64::decode(data.replace('-', "+").replace('_', "/"))
                            {
                                if let Ok(text) = String::from_utf8(decoded) {
                                    if let Some(mime_type) =
                                        payload.get("mimeType").and_then(|m| m.as_str())
                                    {
                                        match mime_type {
                                            "text/plain" => body_text = Some(text),
                                            "text/html" => body_html = Some(text),
                                            // Default to text if we can't determine
                                            _ => body_text = Some(text),
                                        }
                                    } else {
                                        body_text = Some(text);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Create the EmailMessage
            Ok(EmailMessage {
                id,
                thread_id,
                subject,
                from,
                to,
                date,
                snippet,
                body_text,
                body_html,
            })
        }

        /// List messages and parse metadata into structured EmailMessage objects
        pub async fn list_messages(
            &mut self,
            max_results: u32,
            query: Option<&str>,
        ) -> Result<Vec<EmailMessage>> {
            // First get the list of message IDs
            let raw_json = self.list_messages_raw(max_results, query).await?;

            // Parse the raw JSON
            let parsed: serde_json::Value = serde_json::from_str(&raw_json).map_err(|e| {
                GmailApiError::MessageFormatError(format!("Failed to parse message list: {}", e))
            })?;

            // Extract messages array
            let messages = parsed["messages"].as_array().ok_or_else(|| {
                GmailApiError::MessageFormatError(
                    "Missing 'messages' array in response".to_string(),
                )
            })?;

            // Create EmailMessage structs by fetching details for each message ID
            let mut result = Vec::new();

            for message in messages {
                let id = message["id"].as_str().ok_or_else(|| {
                    GmailApiError::MessageFormatError("Message missing 'id' field".to_string())
                })?;

                // Get full message details
                match self.get_message_details(id).await {
                    Ok(email) => {
                        result.push(email);
                    }
                    Err(e) => {
                        // Log error but continue with other messages
                        error!("Failed to get details for message {}: {}", id, e);
                    }
                }

                // Limit to 3 messages to avoid timeout during development
                if result.len() >= 3 {
                    debug!("Reached limit of 3 messages, stopping fetch to avoid timeout");
                    break;
                }
            }

            Ok(result)
        }

        /// List labels and return raw JSON response
        pub async fn list_labels(&mut self) -> Result<String> {
            debug!("Listing labels");

            let endpoint = "/users/me/labels";
            self.request_raw(reqwest::Method::GET, endpoint, None).await
        }

        /// Check connection by getting profile and return raw JSON response
        pub async fn check_connection_raw(&mut self) -> Result<String> {
            debug!("Checking connection raw");

            let endpoint = "/users/me/profile";
            self.request_raw(reqwest::Method::GET, endpoint, None).await
        }

        /// Check connection by getting profile and return email and message count
        pub async fn check_connection(&mut self) -> Result<(String, u64)> {
            debug!("Checking connection");

            let endpoint = "/users/me/profile";

            #[derive(Deserialize)]
            struct Profile {
                #[serde(rename = "emailAddress")]
                email_address: String,
                #[serde(rename = "messagesTotal")]
                messages_total: Option<u64>,
            }

            let profile: Profile = self.request(reqwest::Method::GET, endpoint, None).await?;

            let email = profile.email_address;
            let messages_total = profile.messages_total.unwrap_or(0);

            Ok((email, messages_total))
        }
    }
}

// Module for logging configuration
pub mod logging {
    use chrono::Local;
    use log::LevelFilter;
    use simplelog::{self, CombinedLogger, WriteLogger};
    use std::fs::OpenOptions;
    use std::io::Write;

    /// Sets up logging to file and optionally console
    ///
    /// # Arguments
    ///
    /// * `log_level` - The level of log messages to capture
    /// * `log_file` - Optional path to log file. If None, creates a timestamped file
    ///
    /// # Returns
    ///
    /// The path to the created log file
    pub fn setup_logging(
        log_level: LevelFilter,
        log_file: Option<&str>,
    ) -> std::io::Result<String> {
        // Create a timestamp for the log file
        let timestamp = Local::now().format("%Y%m%d_%H").to_string();

        // Determine log file path
        let log_path = match log_file {
            Some(path) => path.to_string(),
            None => format!("gmail_mcp_{}.log", timestamp),
        };

        // Create the log file with append mode and write header in one operation
        let mut log_file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&log_path)?;

        writeln!(
            log_file,
            "====== GMAIL MCP SERVER LOG - Started at {} ======",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        )?;

        // Use the default config for simplicity - explicitly use simplelog::Config to avoid ambiguity
        let log_config = simplelog::Config::default();

        // During development, consider uncommenting the second logger to see logs on console too
        CombinedLogger::init(vec![
            // File logger
            WriteLogger::new(log_level, log_config, log_file),
            // Uncomment for console logging during development
            // TermLogger::new(
            //     log_level,
            //     simplelog::Config::default(),
            //     simplelog::TerminalMode::Mixed,
            //     simplelog::ColorChoice::Auto
            // ),
        ])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        log::info!("Logging initialized to file: {}", log_path);
        log::debug!("Debug logging enabled");

        Ok(log_path)
    }
}

// Module with the server implementation
pub mod server {
    use log::{debug, error, info};
    use mcp_attr::jsoncall::ErrorCode;
    use mcp_attr::server::{mcp_server, McpServer};
    use mcp_attr::{Error as McpError, Result as McpResult};
    use serde_json::json;

    use crate::config::{Config, ConfigError};
    use crate::gmail_api::{GmailApiError, GmailService};

    // Helper functions
    mod helpers {
        use log::debug;

        /// Converts a serde_json::Value (string or number) to u32 with a default value
        ///
        /// # Arguments
        ///
        /// * `value` - Optional JSON value containing either a number or string
        /// * `default` - Default value to use if conversion fails or value is None
        ///
        /// # Returns
        ///
        /// A u32 value, either converted from input or the provided default
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
                                        "Could not parse string '{}' as u32, using default {}",
                                        s, default
                                    );
                                    default
                                }
                            }
                        }
                        _ => {
                            debug!(
                                "Unexpected value type for max_results: {:?}, using default {}",
                                val, default
                            );
                            default
                        }
                    }
                }
                None => default,
            }
        }
    }

    // Enum of error codes used by the Gmail MCP server with detailed descriptions
    mod error_codes {
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

    // MCP server for accessing Gmail API
    #[derive(Clone)]
    pub struct GmailServer;

    impl GmailServer {
        pub fn new() -> Self {
            GmailServer {}
        }

        // Helper function to create detailed McpError with appropriate error code and context
        fn to_mcp_error(&self, message: &str, code: u32) -> McpError {
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
            // Use with_message instead of set_message, setting is_public to true to show the message to the client
            McpError::new(ErrorCode(code as i64)).with_message(detailed_error, true)
        }

        // Helper function to map GmailApiError to detailed McpError with specific codes
        fn map_gmail_error(&self, err: GmailApiError) -> McpError {
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

                    self.to_mcp_error(&detailed_msg, code)
                }
                GmailApiError::AuthError(e) => {
                    let detailed_msg = format!(
                        "Gmail authentication error: {}. Failed to authenticate with the Gmail API using the provided \
                        credentials. Please verify your client ID, client secret, and refresh token.", 
                        e
                    );
                    self.to_mcp_error(&detailed_msg, error_codes::AUTH_ERROR)
                }
                GmailApiError::MessageRetrievalError(e) => {
                    let detailed_msg = format!(
                        "Message retrieval error: {}. Failed to retrieve the requested message from Gmail. \
                        This may be due to the message being deleted, access permissions, or temporary Gmail API issues.", 
                        e
                    );
                    self.to_mcp_error(&detailed_msg, error_codes::API_ERROR)
                }
                GmailApiError::MessageFormatError(e) => {
                    let detailed_msg = format!(
                        "Message format error: {}. The Gmail API returned a malformed message or one with missing required fields.", 
                        e
                    );
                    self.to_mcp_error(&detailed_msg, error_codes::MESSAGE_FORMAT_ERROR)
                }
                GmailApiError::NetworkError(e) => {
                    let detailed_msg = format!(
                        "Network error: {}. The server couldn't establish a connection to the Gmail API. \
                        This might be due to network configuration issues, outages, or firewall restrictions. \
                        Please check your internet connection and server network configuration.", 
                        e
                    );
                    self.to_mcp_error(&detailed_msg, error_codes::API_ERROR)
                }
                GmailApiError::RateLimitError(e) => {
                    let detailed_msg = format!(
                        "Rate limit error: {}. The Gmail API has rate-limited the server's requests. \
                        This happens when too many requests are made in a short period of time. \
                        The server will automatically retry after a cooldown period, but you may need to wait \
                        or reduce the frequency of requests.", 
                        e
                    );
                    self.to_mcp_error(&detailed_msg, error_codes::API_ERROR)
                }
            }
        }

        // Helper function to initialize Gmail service with detailed error handling
        async fn init_gmail_service(&self) -> McpResult<GmailService> {
            // Load configuration
            let config = Config::from_env().map_err(|err| {
                let msg = match err {
                    ConfigError::MissingEnvVar(var) => {
                        format!(
                            "Missing environment variable: {}. \
                            This variable is required for Gmail authentication. \
                            Please ensure you have set up your .env file correctly or exported the variable in your shell. \
                            Create an OAuth2 client in the Google Cloud Console to obtain these credentials.", 
                            var
                        )
                    }
                    ConfigError::EnvError(e) => {
                        format!(
                            "Environment variable error: {}. \
                            There was a problem reading the environment variables needed for Gmail authentication. \
                            Check permissions on your .env file and ensure it's properly formatted without special characters or quotes.", 
                            e
                        )
                    },
                };
                self.to_mcp_error(&msg, error_codes::CONFIG_ERROR)
            })?;

            // Create Gmail service
            GmailService::new(&config).map_err(|err| {
                error!("Failed to create Gmail service: {}", err);
                self.map_gmail_error(err)
            })
        }
    }

    // MCP server implementation with custom serialization
    #[mcp_server]
    impl McpServer for GmailServer {
        /// Gmail MCP Server
        ///
        /// This MCP server provides direct access to the Gmail API using reqwest.
        /// It requires the following environment variables to be set:
        /// - GMAIL_CLIENT_ID
        /// - GMAIL_CLIENT_SECRET
        /// - GMAIL_REFRESH_TOKEN
        ///
        /// You can provide these in a .env file in the same directory as the executable.
        #[prompt]
        async fn gmail_prompt(&self) -> McpResult<&str> {
            Ok(crate::prompts::GMAIL_MASTER_PROMPT)
        }

        /// Email Analysis Prompt
        ///
        /// Guidelines on how to analyze email content effectively
        #[prompt]
        async fn email_analysis_prompt(&self) -> McpResult<&str> {
            Ok(crate::prompts::EMAIL_ANALYSIS_PROMPT)
        }

        /// Email Summarization Prompt
        ///
        /// Guidelines on how to create concise email summaries
        #[prompt]
        async fn email_summarization_prompt(&self) -> McpResult<&str> {
            Ok(crate::prompts::EMAIL_SUMMARIZATION_PROMPT)
        }

        /// Email Search Prompt
        ///
        /// Guide to effective Gmail search strategies
        #[prompt]
        async fn email_search_prompt(&self) -> McpResult<&str> {
            Ok(crate::prompts::EMAIL_SEARCH_PROMPT)
        }

        /// Task Extraction Prompt
        ///
        /// Instructions for finding action items in emails
        #[prompt]
        async fn task_extraction_prompt(&self) -> McpResult<&str> {
            Ok(crate::prompts::TASK_EXTRACTION_PROMPT)
        }

        /// Meeting Extraction Prompt
        ///
        /// Instructions for finding meeting details in emails
        #[prompt]
        async fn meeting_extraction_prompt(&self) -> McpResult<&str> {
            Ok(crate::prompts::MEETING_EXTRACTION_PROMPT)
        }

        /// Contact Extraction Prompt
        ///
        /// Instructions for extracting contact information from emails
        #[prompt]
        async fn contact_extraction_prompt(&self) -> McpResult<&str> {
            Ok(crate::prompts::CONTACT_EXTRACTION_PROMPT)
        }

        /// Email Categorization Prompt
        ///
        /// Guide to categorizing emails effectively
        #[prompt]
        async fn email_categorization_prompt(&self) -> McpResult<&str> {
            Ok(crate::prompts::EMAIL_CATEGORIZATION_PROMPT)
        }

        /// Email Prioritization Prompt
        ///
        /// Guide to prioritizing emails effectively
        #[prompt]
        async fn email_prioritization_prompt(&self) -> McpResult<&str> {
            Ok(crate::prompts::EMAIL_PRIORITIZATION_PROMPT)
        }

        /// Email Drafting Prompt
        ///
        /// Guide to writing effective emails
        #[prompt]
        async fn email_drafting_prompt(&self) -> McpResult<&str> {
            Ok(crate::prompts::EMAIL_DRAFTING_PROMPT)
        }

        /// Get a list of emails from the inbox
        ///
        /// Returns emails with subject, sender, recipient, date and snippet information.
        ///
        /// Args:
        ///   max_results: Optional maximum number of results to return (default: 10). Can be a number (3) or a string ("3").
        ///   query: Optional Gmail search query string (e.g. "is:unread from:example.com")
        #[tool]
        async fn list_emails(
            &self,
            max_results: Option<serde_json::Value>,
            query: Option<String>,
        ) -> McpResult<String> {
            info!("=== START list_emails MCP command ===");
            debug!(
                "list_emails called with max_results={:?}, query={:?}",
                max_results, query
            );

            // Convert max_results using the helper function (default: 10)
            let max = helpers::parse_max_results(max_results, 10);

            // Get the Gmail service
            let mut service = self.init_gmail_service().await?;

            // Get messages with full metadata
            let result = match service.list_messages(max, query.as_deref()).await {
                Ok(messages) => {
                    // Convert to JSON
                    serde_json::to_string(&messages).map_err(|e| {
                        let error_msg = format!("Failed to serialize message list: {}", e);
                        error!("{}", error_msg);
                        self.to_mcp_error(&error_msg, error_codes::MESSAGE_FORMAT_ERROR)
                    })?
                }
                Err(err) => {
                    let query_info = query.as_deref().unwrap_or("none");
                    error!(
                        "Failed to list emails with max_results={}, query='{}': {}",
                        max, query_info, err
                    );

                    // Create detailed contextual error
                    error!("Context: Failed to list emails with parameters: max_results={}, query='{}'", 
                        max, query_info
                    );

                    return Err(self.map_gmail_error(err));
                }
            };

            info!("=== END list_emails MCP command (success) ===");
            Ok(result)
        }
        /// Get details for a specific email
        ///
        /// Returns the message with all metadata and content parsed into a structured format.
        ///
        /// Args:
        ///   message_id: The ID of the message to retrieve
        #[tool]
        async fn get_email(&self, message_id: String) -> McpResult<String> {
            info!("=== START get_email MCP command ===");
            debug!("get_email called with message_id={}", message_id);

            // Get the Gmail service
            let mut service = self.init_gmail_service().await?;

            // Get detailed message directly using the helper method
            let email = match service.get_message_details(&message_id).await {
                Ok(email) => email,
                Err(err) => {
                    error!(
                        "Failed to get email with message_id='{}': {}",
                        message_id, err
                    );

                    // Create detailed contextual error
                    error!(
                        "Context: Failed to retrieve email with ID: '{}'",
                        message_id
                    );

                    return Err(self.map_gmail_error(err));
                }
            };

            // Convert to JSON
            let result = serde_json::to_string(&email).map_err(|e| {
                let error_msg = format!("Failed to serialize email: {}", e);
                error!("{}", error_msg);
                self.to_mcp_error(&error_msg, error_codes::MESSAGE_FORMAT_ERROR)
            })?;

            info!("=== END get_email MCP command (success) ===");
            Ok(result)
        }
        /// Search for emails using a Gmail search query
        ///
        /// Returns emails with subject, sender, recipient, date and snippet information.
        ///
        /// Args:
        ///   query: Gmail search query string (e.g. "is:unread from:example.com")
        ///   max_results: Optional maximum number of results (default: 10). Can be a number (3) or a string ("3").
        #[tool]
        async fn search_emails(
            &self,
            query: String,
            max_results: Option<serde_json::Value>,
        ) -> McpResult<String> {
            info!("=== START search_emails MCP command ===");
            debug!(
                "search_emails called with query={:?}, max_results={:?}",
                query, max_results
            );

            // Get the parsed max_results value
            let max = helpers::parse_max_results(max_results, 10);

            // Get the Gmail service
            let mut service = self.init_gmail_service().await?;

            // Get messages with full metadata
            let result = match service.list_messages(max, Some(&query)).await {
                Ok(messages) => {
                    // Convert to JSON
                    serde_json::to_string(&messages).map_err(|e| {
                        let error_msg = format!("Failed to serialize message list: {}", e);
                        error!("{}", error_msg);
                        self.to_mcp_error(&error_msg, error_codes::MESSAGE_FORMAT_ERROR)
                    })?
                }
                Err(err) => {
                    error!(
                        "Failed to search emails with query='{}', max_results={}: {}",
                        query, max, err
                    );

                    // Create detailed contextual error with specific advice for search queries
                    error!("Context: Failed to search emails with query: '{}'", query);

                    return Err(self.map_gmail_error(err));
                }
            };

            info!("=== END search_emails MCP command (success) ===");
            Ok(result)
        }

        /// Get a list of email labels
        ///
        /// Returns the raw JSON response from the Gmail API without any transformation or modification.
        #[tool]
        async fn list_labels(&self) -> McpResult<String> {
            debug!("list_labels called");

            // Get the Gmail service
            let mut service = self.init_gmail_service().await?;

            // Get labels
            match service.list_labels().await {
                Ok(labels) => Ok(labels),
                Err(err) => {
                    error!("Failed to list labels: {}", err);

                    // Provide detailed error with troubleshooting steps
                    // Include detailed context in the error log
                    error!("Context: Failed to retrieve Gmail labels. This operation requires read access permissions.");

                    Err(self.map_gmail_error(err))
                }
            }
        }

        /// Check connection status with Gmail API
        ///
        /// Tests the connection to Gmail API by retrieving the user's profile.
        /// Returns the raw JSON response from the Gmail API without any transformation or modification.
        #[tool]
        async fn check_connection(&self) -> McpResult<String> {
            info!("=== START check_connection MCP command ===");
            debug!("check_connection called");

            // Get the Gmail service
            let mut service = self.init_gmail_service().await?;

            // Get profile as raw JSON
            let profile_json = match service.check_connection_raw().await {
                Ok(json) => json,
                Err(err) => {
                    error!("Connection check failed: {}", err);

                    // Provide helpful information on connectivity issues
                    // Include detailed context in the error log
                    error!("Context: Failed to connect to Gmail API. This is a basic connectivity test failure.");

                    return Err(self.map_gmail_error(err));
                }
            };

            info!("=== END check_connection MCP command (success) ===");
            Ok(profile_json)
        }

        /// Analyze an email to extract key information
        ///
        /// Takes an email ID and performs a detailed analysis on its content.
        /// Extracts information like action items, meeting details, contact information,
        /// sentiment, priority, and suggested next steps.
        ///
        /// Args:
        ///   message_id: The ID of the message to analyze
        ///   analysis_type: Optional type of analysis to perform. Can be "general", "tasks",
        ///                  "meetings", "contacts", or "all". Default is "general".
        #[tool]
        async fn analyze_email(
            &self,
            message_id: String,
            analysis_type: Option<String>,
        ) -> McpResult<String> {
            info!("=== START analyze_email MCP command ===");
            debug!(
                "analyze_email called with message_id={}, analysis_type={:?}",
                message_id, analysis_type
            );

            // Get the Gmail service
            let mut service = self.init_gmail_service().await?;

            // Get the specified email
            let email = match service.get_message_details(&message_id).await {
                Ok(msg) => msg,
                Err(err) => {
                    error!("Failed to get email for analysis: {}", err);
                    return Err(self.map_gmail_error(err));
                }
            };

            // Determine what type of analysis to perform
            let analysis = analysis_type.unwrap_or_else(|| "general".to_string());

            // Prepare the analysis result
            let result = match analysis.to_lowercase().as_str() {
                "tasks" | "task" => {
                    // Create a structured JSON for task analysis
                    json!({
                        "email_id": email.id,
                        "subject": email.subject,
                        "from": email.from,
                        "date": email.date,
                        "analysis_type": "tasks",
                        "content": email.body_text.unwrap_or_else(|| email.snippet.unwrap_or_default()),
                        "analysis_prompt": crate::prompts::TASK_EXTRACTION_PROMPT
                    })
                }
                "meetings" | "meeting" => {
                    // Create a structured JSON for meeting analysis
                    json!({
                        "email_id": email.id,
                        "subject": email.subject,
                        "from": email.from,
                        "date": email.date,
                        "analysis_type": "meetings",
                        "content": email.body_text.unwrap_or_else(|| email.snippet.unwrap_or_default()),
                        "analysis_prompt": crate::prompts::MEETING_EXTRACTION_PROMPT
                    })
                }
                "contacts" | "contact" => {
                    // Create a structured JSON for contact analysis
                    json!({
                        "email_id": email.id,
                        "subject": email.subject,
                        "from": email.from,
                        "date": email.date,
                        "analysis_type": "contacts",
                        "content": email.body_text.unwrap_or_else(|| email.snippet.unwrap_or_default()),
                        "analysis_prompt": crate::prompts::CONTACT_EXTRACTION_PROMPT
                    })
                }
                "summary" | "summarize" => {
                    // Create a structured JSON for email summarization
                    json!({
                        "email_id": email.id,
                        "subject": email.subject,
                        "from": email.from,
                        "date": email.date,
                        "analysis_type": "summary",
                        "content": email.body_text.unwrap_or_else(|| email.snippet.unwrap_or_default()),
                        "analysis_prompt": crate::prompts::EMAIL_SUMMARIZATION_PROMPT
                    })
                }
                "priority" | "prioritize" => {
                    // Create a structured JSON for email prioritization
                    json!({
                        "email_id": email.id,
                        "subject": email.subject,
                        "from": email.from,
                        "date": email.date,
                        "analysis_type": "priority",
                        "content": email.body_text.unwrap_or_else(|| email.snippet.unwrap_or_default()),
                        "analysis_prompt": crate::prompts::EMAIL_PRIORITIZATION_PROMPT
                    })
                }
                "all" => {
                    // Create comprehensive JSON with all analysis types
                    json!({
                        "email_id": email.id,
                        "subject": email.subject,
                        "from": email.from,
                        "to": email.to,
                        "date": email.date,
                        "analysis_type": "comprehensive",
                        "content": email.body_text.unwrap_or_else(|| email.snippet.unwrap_or_default()),
                        "html_content": email.body_html,
                        "analysis_prompts": {
                            "general": crate::prompts::EMAIL_ANALYSIS_PROMPT,
                            "tasks": crate::prompts::TASK_EXTRACTION_PROMPT,
                            "meetings": crate::prompts::MEETING_EXTRACTION_PROMPT,
                            "contacts": crate::prompts::CONTACT_EXTRACTION_PROMPT,
                            "priority": crate::prompts::EMAIL_PRIORITIZATION_PROMPT
                        }
                    })
                }
                _ => {
                    // Default to general analysis
                    json!({
                        "email_id": email.id,
                        "subject": email.subject,
                        "from": email.from,
                        "date": email.date,
                        "analysis_type": "general",
                        "content": email.body_text.unwrap_or_else(|| email.snippet.unwrap_or_default()),
                        "analysis_prompt": crate::prompts::EMAIL_ANALYSIS_PROMPT
                    })
                }
            };

            // Convert to string
            let result_json = serde_json::to_string_pretty(&result).map_err(|e| {
                let error_msg = format!("Failed to serialize analysis result: {}", e);
                error!("{}", error_msg);
                self.to_mcp_error(&error_msg, error_codes::MESSAGE_FORMAT_ERROR)
            })?;

            info!("=== END analyze_email MCP command (success) ===");
            Ok(result_json)
        }
        
        /// Batch analyze multiple emails
        ///
        /// Takes a list of email IDs and performs quick analysis on each one.
        /// Useful for getting an overview of multiple emails at once.
        ///
        /// Args:
        ///   message_ids: List of email IDs to analyze
        ///   analysis_type: Optional type of analysis to perform. Can be "summary", "tasks", 
        ///                  "priority", or "category". Default is "summary".
        #[tool]
        async fn batch_analyze_emails(&self, message_ids: Vec<String>, analysis_type: Option<String>) -> McpResult<String> {
            info!("=== START batch_analyze_emails MCP command ===");
            debug!(
                "batch_analyze_emails called with {} messages, analysis_type={:?}",
                message_ids.len(), analysis_type
            );
            
            // Get the Gmail service
            let mut service = self.init_gmail_service().await?;
            
            // Determine what type of analysis to perform
            let analysis = analysis_type.unwrap_or_else(|| "summary".to_string()).to_lowercase();
            
            // Analyze each email
            let mut results = Vec::new();
            for id in message_ids {
                debug!("Analyzing email {}", id);
                
                // Get the specified email
                match service.get_message_details(&id).await {
                    Ok(email) => {
                        // Prepare analysis based on type
                        let analysis_prompt = match analysis.as_str() {
                            "tasks" | "task" => crate::prompts::TASK_EXTRACTION_PROMPT,
                            "priority" => crate::prompts::EMAIL_PRIORITIZATION_PROMPT,
                            "category" => crate::prompts::EMAIL_CATEGORIZATION_PROMPT,
                            _ => crate::prompts::EMAIL_SUMMARIZATION_PROMPT, // Default to summary
                        };
                        
                        // Create analysis result
                        let result = json!({
                            "email_id": email.id,
                            "subject": email.subject,
                            "from": email.from,
                            "date": email.date,
                            "analysis_type": analysis,
                            "content": email.body_text.unwrap_or_else(|| email.snippet.unwrap_or_default()),
                            "analysis_prompt": analysis_prompt
                        });
                        
                        results.push(result);
                    },
                    Err(err) => {
                        // Log error but continue with other emails
                        error!("Failed to analyze email {}: {}", id, err);
                        
                        // Add error entry to results
                        results.push(json!({
                            "email_id": id,
                            "error": format!("Failed to retrieve email: {}", err)
                        }));
                    }
                }
            }
            
            // Create a batch result
            let batch_result = json!({
                "analysis_type": analysis,
                "email_count": results.len(),
                "results": results
            });
            
            // Convert to string
            let result_json = serde_json::to_string_pretty(&batch_result).map_err(|e| {
                let error_msg = format!("Failed to serialize batch analysis result: {}", e);
                error!("{}", error_msg);
                self.to_mcp_error(&error_msg, error_codes::MESSAGE_FORMAT_ERROR)
            })?;
            
            info!("=== END batch_analyze_emails MCP command (success) ===");
            Ok(result_json)
        }
    }
}

// Module with prompts for MCP
pub mod prompts {
    /// Gmail Assistant Prompts
    ///
    /// These prompts help Claude understand how to interact with Gmail data
    /// through the MCP server tools and provide useful analysis.

    /// Master prompt for system context
    pub const GMAIL_MASTER_PROMPT: &str = r#"
# Gmail Assistant

You have access to email data through a Gmail MCP server. Your role is to help users manage, analyze, and extract insights from their emails. You can search emails, read messages, and provide summaries and analyses.

## Capabilities
- List and search emails with various criteria
- Get detailed content of specific emails
- Analyze email content, sentiment, and context
- Extract action items, summaries, and key points

## Important Notes
- Handle email data with privacy and security in mind
- Format email data in a readable way
- Highlight important information from emails
- Extract action items and tasks when relevant
"#;

    /// Analysis prompt for email content
    pub const EMAIL_ANALYSIS_PROMPT: &str = r#"
When analyzing emails, consider these aspects:

1. Key Information:
   - Identify the primary purpose of the email
   - Extract key dates, deadlines, or time-sensitive information
   - Note important names, contacts, or organizations mentioned

2. Action Items:
   - Identify explicit requests or tasks assigned to the recipient
   - Note any deadlines mentioned for these actions
   - Highlight any decisions the recipient needs to make

3. Context and Background:
   - Determine if this is part of an ongoing conversation
   - Identify references to previous communications
   - Note any attached files or links to external resources

4. Tone and Sentiment:
   - Assess the formality level (formal, casual, urgent)
   - Note emotional tone (neutral, positive, negative, urgent)
   - Identify any sensitive or confidential content

5. Next Steps:
   - Suggest appropriate follow-up actions
   - Identify if a response is expected and by when
   - Note any calendar events that should be created

Format your analysis in a clear, structured way to help the user quickly understand the most important aspects of the email.
"#;

    /// Summarization prompt for emails
    pub const EMAIL_SUMMARIZATION_PROMPT: &str = r#"
When summarizing emails, follow these guidelines:

1. Length and Detail:
   - For short emails: Provide a 1-2 sentence summary
   - For medium emails: Provide 2-3 key bullet points
   - For long emails: Provide a structured summary with sections

2. Content Focus:
   - Prioritize action items and requests
   - Include deadlines or time-sensitive information
   - Maintain the core message and intent
   - Include only the most relevant details

3. Structure:
   - Start with the main purpose of the email
   - List any actions required of the recipient
   - Note any important details or context
   - End with deadline information if applicable

4. Style:
   - Use concise, clear language
   - Maintain a neutral tone
   - Use present tense for clarity
   - Avoid unnecessary details while keeping essential information

Your summary should allow the user to understand the email's purpose and any required actions without reading the full text.
"#;

    /// Email search strategies prompt
    pub const EMAIL_SEARCH_PROMPT: &str = r#"
When helping users search for emails, consider these effective strategies:

1. Gmail Search Operators:
   - from: (sender's email address)
   - to: (recipient's email address)
   - subject: (words in the subject line)
   - has:attachment (emails with attachments)
   - after:YYYY/MM/DD (emails after a specific date)
   - before:YYYY/MM/DD (emails before a specific date)
   - is:unread (unread emails)
   - label:x (emails with a specific label)
   - filename:xyz (emails with attachments of a specific name or type)

2. Combinatorial Search:
   - Use multiple operators with AND/OR logic
   - Example: "from:john@example.com AND has:attachment AND after:2023/01/01"

3. Phrase Search:
   - Use quotes for exact phrases
   - Example: "quarterly report"

4. Exclusion:
   - Use "-" to exclude terms
   - Example: "project update -meeting"

5. Search Refinement:
   - Start broad, then narrow with additional terms
   - Consider variations in spelling or phrasing
   - Try different date ranges if initial search is unsuccessful

When suggesting search queries, aim to be specific enough to find relevant emails but not so narrow that important messages are missed. Allow for progressive refinement based on initial results.
"#;

    /// Task extraction prompt
    pub const TASK_EXTRACTION_PROMPT: &str = r#"
When extracting tasks and action items from emails, look for:

1. Explicit Requests:
   - Direct questions that need answers
   - Phrases like "Could you", "Please", "I need you to"
   - Sentences ending with question marks requiring action
   - Requests for feedback, review, or input

2. Implied Tasks:
   - Mentions of deadlines without explicit requests
   - Information sharing that implies a need for response
   - Updates that might require acknowledgment
   - References to shared responsibilities

3. Task Components to Identify:
   - The specific action required
   - Who is responsible (if mentioned)
   - Deadline or timeframe
   - Priority level (if indicated)
   - Dependencies or prerequisites
   - Related resources or references

4. Format Tasks Clearly:
   - Use action-oriented language
   - Start with verbs (Respond, Review, Complete, etc.)
   - Include the deadline if available
   - Add context for clarity

Present tasks in a structured list format that can be easily transferred to a task management system. If dates are mentioned, format them consistently (YYYY-MM-DD) to facilitate calendar integration.
"#;

    /// Meeting extraction prompt
    pub const MEETING_EXTRACTION_PROMPT: &str = r#"
When extracting meeting information from emails, look for:

1. Key Meeting Details:
   - Date and time (including time zone if specified)
   - Duration (if mentioned)
   - Location (physical location or virtual meeting link)
   - Meeting title or purpose
   - Organizer name and contact information
   - Required and optional attendees

2. Meeting Context:
   - Agenda items or topics for discussion
   - Pre-meeting preparation requirements
   - Relevant documents or links
   - Background information or meeting objectives
   - Connection to previous or future meetings

3. Technical Details (for virtual meetings):
   - Platform (Zoom, Teams, Google Meet, etc.)
   - Meeting ID or conference code
   - Password information
   - Dial-in numbers for phone access
   - Technical requirements or instructions

4. Format Meeting Information:
   - Present in calendar-friendly format
   - Clearly separate core details (when, where, who) from supporting information
   - Highlight any required preparation or action before the meeting
   - Note if a response or RSVP is required

Present this information in a structured format that allows the user to quickly understand all relevant meeting details and easily add the meeting to their calendar.
"#;

    /// Contact information extraction prompt
    pub const CONTACT_EXTRACTION_PROMPT: &str = r#"
When extracting contact information from emails, look for:

1. Personal Identifiers:
   - Full name
   - Job title/position
   - Company/organization
   - Department/team

2. Contact Details:
   - Email address(es)
   - Phone number(s) with type (mobile, office, etc.)
   - Physical address(es)
   - Website/social media profiles
   - Messaging handles (Slack, Teams, etc.)

3. Context Information:
   - How they're connected to the sender/recipient
   - Their role in the discussed matter
   - Preferred contact method (if mentioned)
   - Time zone or working hours (if relevant)
   - Assistant or secondary contact information

4. Format Considerations:
   - Group related information together
   - Clearly label different types of contact information
   - Preserve formatting for complex items (e.g., international phone numbers)
   - Note if the information appears to be from an email signature

Present the extracted contact information in a structured, organized format that could be easily added to a contact management system or address book.
"#;

    /// Email categorization prompt
    pub const EMAIL_CATEGORIZATION_PROMPT: &str = r#"
When categorizing emails, consider these common categories and their characteristics:

1. Action Required:
   - Contains explicit requests or tasks
   - Requires a response or decision
   - Has deadlines or time-sensitive content
   - Directly asks for input or feedback

2. FYI/Information:
   - Provides updates without requiring action
   - Shares information for awareness
   - Contains newsletters or announcements
   - Serves as documentation or reference

3. Follow-up:
   - Continues a previous conversation
   - References earlier communications
   - Provides requested information
   - Confirms completion of a task

4. Administrative:
   - Relates to operational matters
   - Contains policy updates or procedural information
   - Includes HR-related communications
   - Addresses organizational announcements

5. Personal:
   - Non-work related content
   - Communication from friends or family
   - Personal updates or invitations
   - Not directly business-related

6. Important/Urgent:
   - Time-critical information
   - High-priority requests
   - Contains escalation language
   - From key stakeholders or leadership

7. Commercial/Marketing:
   - Promotional content
   - Product announcements or offers
   - Advertising materials
   - Subscription-based content

For each email, identify the primary category and any secondary categories that might apply. Consider the sender, subject line, content, and context in your categorization.
"#;

    /// Email prioritization prompt
    pub const EMAIL_PRIORITIZATION_PROMPT: &str = r#"
When helping users prioritize emails, consider these factors:

1. Urgency Indicators:
   - Explicit deadlines mentioned in the content
   - Time-sensitive language ("urgent," "ASAP," "today")
   - Proximity of mentioned dates to current date
   - Follow-ups or reminders about previous requests

2. Importance Factors:
   - Sender's role or relationship to the user
   - Topic's alignment with known user priorities
   - Organizational impact of the content
   - Whether the user is in the main recipient line (To vs. CC)

3. Action Requirements:
   - Direct questions or requests made of the user
   - Explicit asks for response or input
   - Decision points requiring the user's authority
   - Tasks that others are waiting on

4. Prioritization Categories:
   - Critical (requires immediate attention)
   - High (important and time-sensitive, but not immediate)
   - Medium (important but not time-critical)
   - Low (routine information or future reference)
   - No Action (purely informational)

5. Dependency Considerations:
   - Whether others are waiting on the user's response
   - If the email blocks progress on important projects
   - Connection to high-priority organizational goals
   - Relationship to upcoming meetings or deadlines

When suggesting prioritization, explain the reasoning briefly to help the user understand the recommendation and adjust their approach accordingly.
"#;

    /// Email drafting assistance prompt
    pub const EMAIL_DRAFTING_PROMPT: &str = r#"
When helping users draft effective emails, follow these guidelines:

1. Structure:
   - Clear, concise subject line that reflects content
   - Brief greeting appropriate to the relationship
   - Purpose statement in the first paragraph
   - Logically organized body with short paragraphs
   - Clear closing with next steps or expectations
   - Professional signature with relevant contact information

2. Content Considerations:
   - State the purpose immediately
   - Provide necessary context without overexplaining
   - Highlight key information or questions
   - Use bullet points for multiple items or requests
   - Be explicit about any deadlines or timelines
   - Clearly state requested actions or responses

3. Tone and Style:
   - Match formality to the relationship and context
   - Be respectful and professional
   - Use active voice for clarity
   - Keep sentences concise (15-20 words on average)
   - Avoid jargon unless appropriate for the audience
   - Be polite but direct with requests

4. Special Situations:
   - For introductions: explain context and mutual benefit
   - For requests: be specific and make responding easy
   - For follow-ups: reference previous communication
   - For sensitive topics: be diplomatic yet clear
   - For group emails: consider what everyone needs to know

5. Before Sending Checklist:
   - Verify all necessary information is included
   - Check that tone is appropriate
   - Ensure requests or questions are clear
   - Confirm any attachments are mentioned and included
   - Review for typos, grammar issues, or unclear phrasing

Adapt these guidelines based on the specific purpose, audience, and context of the email being drafted.
"#;
}

// OAuth authentication module for token refresh flow
pub mod auth {
    use crate::config::Config;
    use axum::extract::Query;
    use axum::response::Html;
    use axum::routing::get;
    use axum::Router;
    use dotenv::dotenv;
    use log::error;
    use rand::distributions::{Alphanumeric, DistString};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::env;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::net::SocketAddr;
    use std::path::Path;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use url::Url;

    // OAuth scopes needed for Gmail access
    const GMAIL_SCOPE: &str = "https://mail.google.com/";
    const OAUTH_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/auth";
    const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

    // Local server config
    const DEFAULT_PORT: u16 = 8080;
    const DEFAULT_HOST: &str = "127.0.0.1";

    // Structure to hold the OAuth state
    #[derive(Clone, Debug, Default)]
    struct OAuthState {
        auth_code: Option<String>,
        state_token: Option<String>,
        complete: bool,
    }

    // Structure for OAuth authorization parameters
    #[derive(Debug, Serialize)]
    struct AuthParams {
        client_id: String,
        redirect_uri: String,
        response_type: String,
        scope: String,
        state: String,
        access_type: String,
        prompt: String,
    }

    // Structure for the callback query parameters
    #[derive(Debug, Deserialize)]
    struct CallbackParams {
        code: Option<String>,
        state: Option<String>,
        error: Option<String>,
    }

    // Structure for the token response
    #[derive(Debug, Deserialize)]
    struct TokenResponse {
        access_token: String,
        expires_in: u64,
        refresh_token: String,
        token_type: String,
        scope: Option<String>,
    }

    // Run the OAuth flow to get a new refresh token
    pub async fn run_oauth_flow() -> Result<(), String> {
        // Attempt to load existing credentials
        let _ = dotenv();

        // Get client ID and secret from environment or prompt user
        let client_id = env::var("GMAIL_CLIENT_ID").unwrap_or_else(|_| {
            println!("Enter your Google OAuth client ID:");
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input");
            input.trim().to_string()
        });

        let client_secret = env::var("GMAIL_CLIENT_SECRET").unwrap_or_else(|_| {
            println!("Enter your Google OAuth client secret:");
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input");
            input.trim().to_string()
        });

        // Generate a random state token for CSRF protection
        let state_token = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);

        // Set up the redirect URI for the local callback server
        let port = env::var("OAUTH_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(DEFAULT_PORT);

        let host = env::var("OAUTH_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
        let redirect_uri = format!("http://{}:{}/oauth/callback", host, port);

        // Create a shared state to store the authorization code
        let oauth_state = Arc::new(Mutex::new(OAuthState {
            auth_code: None,
            state_token: Some(state_token.clone()),
            complete: false,
        }));

        // Build the authorization URL
        let auth_url = build_auth_url(
            &client_id,
            &redirect_uri,
            &state_token,
            &[GMAIL_SCOPE.to_string()],
        )?;

        // Start the local web server to handle the OAuth callback
        let server_handle = start_oauth_server(port, host.clone(), oauth_state.clone());

        // Open the authorization URL in the default browser
        println!("Opening browser to authorize with Google...");
        println!("\nAuthorization URL: {}", auth_url);

        if let Err(e) = webbrowser::open(&auth_url) {
            println!("Failed to open web browser automatically: {}", e);
            println!("Please manually open the URL in your browser to continue.");
        }

        // Wait for the authorization to complete
        println!("Waiting for authorization...");
        let auth_code = wait_for_auth_code(oauth_state).await?;

        // Exchange the authorization code for tokens
        println!("Exchanging authorization code for tokens...");
        let tokens =
            exchange_code_for_tokens(&client_id, &client_secret, &auth_code, &redirect_uri).await?;

        // Update the .env file with the new tokens
        println!("Updating credentials in .env file...");
        update_env_file(
            &client_id,
            &client_secret,
            &tokens.refresh_token,
            &tokens.access_token,
            &redirect_uri,
        )?;

        // Shut down the server
        server_handle.abort();

        println!("\n🎉 Authentication successful! New tokens have been saved to .env file.");

        Ok(())
    }

    // Build the authorization URL
    fn build_auth_url(
        client_id: &str,
        redirect_uri: &str,
        state: &str,
        scopes: &[String],
    ) -> Result<String, String> {
        let mut url = Url::parse(OAUTH_AUTH_URL).map_err(|e| e.to_string())?;

        // Add required OAuth parameters
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("client_id", client_id);
            query.append_pair("redirect_uri", redirect_uri);
            query.append_pair("response_type", "code");
            query.append_pair("scope", &scopes.join(" "));
            query.append_pair("state", state);
            query.append_pair("access_type", "offline");
            query.append_pair("prompt", "consent"); // Ensure we always get a refresh token
            query.finish();
        }

        // Return the URL
        Ok(url.to_string())
    }

    // Start a local web server to handle the OAuth callback
    fn start_oauth_server(
        port: u16,
        host: String,
        state: Arc<Mutex<OAuthState>>,
    ) -> tokio::task::JoinHandle<()> {
        // Create the router with callback and index routes
        let app = Router::new()
            .route(
                "/",
                get(|| async {
                    Html("<h1>Gmail OAuth Server</h1><p>Waiting for OAuth callback...</p>")
                }),
            )
            .route(
                "/oauth/callback",
                get(move |query| handle_callback(query, state.clone())),
            );

        // Start the server in a background task
        tokio::spawn(async move {
            let addr = format!("{host}:{port}").parse::<SocketAddr>().unwrap();
            println!("\nStarting OAuth callback server on http://{host}:{port}");

            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            if let Err(e) = axum::serve(listener, app).await {
                error!("Server error: {}", e);
            }
        })
    }

    // Handle the OAuth callback from Google
    async fn handle_callback(
        Query(params): Query<CallbackParams>,
        state: Arc<Mutex<OAuthState>>,
    ) -> Html<String> {
        let mut oauth_state = state.lock().await;

        // Check for errors
        if let Some(error) = params.error {
            oauth_state.complete = true;
            return Html(format!(
                "<html>
<head><title>OAuth Error</title></head>
<body>
    <h1>OAuth Error</h1>
    <p>An error occurred during authentication: {}</p>
    <p>Please close this window and try again.</p>
</body>
</html>",
                error
            ));
        }

        // Check state token to prevent CSRF attacks
        if params.state != oauth_state.state_token {
            oauth_state.complete = true;
            return Html(
                "<html>
<head><title>Authentication Failed</title></head>
<body>
    <h1>Authentication Failed</h1>
    <p>Invalid state parameter. This could be a CSRF attack attempt.</p>
    <p>Please close this window and try again.</p>
</body>
</html>"
                    .to_string(),
            );
        }

        // Store the authorization code
        if let Some(code) = params.code {
            oauth_state.auth_code = Some(code);
            oauth_state.complete = true;

            // Return success page
            Html(
                "<html>
<head>
    <title>Authentication Successful</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; }
        h1 { color: #4285f4; }
        .success { color: green; }
    </style>
</head>
<body>
    <h1>Gmail OAuth Authentication</h1>
    <h2 class=\"success\">Authentication Successful! ✅</h2>
    <p>You have successfully authenticated with Google.</p>
    <p>You can now close this window and return to the application.</p>
</body>
</html>"
                    .to_string(),
            )
        } else {
            oauth_state.complete = true;

            // Missing authorization code
            Html(
                "<html>
<head><title>Authentication Failed</title></head>
<body>
    <h1>Authentication Failed</h1>
    <p>No authorization code received from Google.</p>
    <p>Please close this window and try again.</p>
</body>
</html>"
                    .to_string(),
            )
        }
    }

    // Wait for the authorization code to be received
    async fn wait_for_auth_code(state: Arc<Mutex<OAuthState>>) -> Result<String, String> {
        // Poll for the authorization code with a timeout
        let max_wait_seconds = 300; // 5 minutes
        let poll_interval = std::time::Duration::from_secs(1);

        for _ in 0..max_wait_seconds {
            let oauth_state = state.lock().await;

            // Check if we have the authorization code
            if let Some(code) = oauth_state.auth_code.clone() {
                return Ok(code);
            }

            // Check if the flow completed with an error
            if oauth_state.complete {
                return Err("Authorization failed. Check the browser for details.".to_string());
            }

            // Release the lock and wait before trying again
            drop(oauth_state);
            tokio::time::sleep(poll_interval).await;
        }

        Err("Timed out waiting for authorization. Please try again.".to_string())
    }

    // Exchange the authorization code for access and refresh tokens
    async fn exchange_code_for_tokens(
        client_id: &str,
        client_secret: &str,
        auth_code: &str,
        redirect_uri: &str,
    ) -> Result<TokenResponse, String> {
        let client = reqwest::Client::new();

        // Prepare the token request parameters
        let params = [
            ("code", auth_code),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        // Make the token request
        let response = client
            .post(OAUTH_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Failed to exchange code for tokens: {}", e))?;

        // Check for error responses
        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<no response body>".to_string());

            return Err(format!(
                "Failed to exchange code for tokens. Status: {}, Error: {}",
                status, error_text
            ));
        }

        // Parse the token response
        let tokens: TokenResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse token response: {}", e))?;

        Ok(tokens)
    }

    // Update the .env file with the new tokens
    fn update_env_file(
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
        access_token: &str,
        redirect_uri: &str,
    ) -> Result<(), String> {
        // Check if .env file exists
        let env_path = ".env";
        let env_exists = Path::new(env_path).exists();

        // Create or update the .env file
        if env_exists {
            // Read existing .env content
            let content = std::fs::read_to_string(env_path)
                .map_err(|e| format!("Failed to read .env file: {}", e))?;

            // Parse the content into a HashMap
            let mut env_vars = HashMap::new();
            for line in content.lines() {
                // Skip comments and empty lines
                if line.starts_with('#') || line.trim().is_empty() {
                    continue;
                }

                // Parse key-value pairs
                if let Some(pos) = line.find('=') {
                    let key = line[..pos].trim().to_string();
                    let value = line[pos + 1..].trim().to_string();
                    env_vars.insert(key, value);
                }
            }

            // Update the values
            env_vars.insert("GMAIL_CLIENT_ID".to_string(), client_id.to_string());
            env_vars.insert("GMAIL_CLIENT_SECRET".to_string(), client_secret.to_string());
            env_vars.insert("GMAIL_REFRESH_TOKEN".to_string(), refresh_token.to_string());
            env_vars.insert("GMAIL_ACCESS_TOKEN".to_string(), access_token.to_string());
            env_vars.insert("GMAIL_REDIRECT_URI".to_string(), redirect_uri.to_string());

            // Build the new content
            let mut new_content = String::new();
            new_content.push_str("# Gmail API OAuth2 credentials\n");
            for (key, value) in &env_vars {
                new_content.push_str(&format!("{key}={value}\n"));
            }

            // Write the updated content back to the file
            std::fs::write(env_path, new_content)
                .map_err(|e| format!("Failed to write to .env file: {}", e))?;
        } else {
            // Create a new .env file
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(env_path)
                .map_err(|e| format!("Failed to create .env file: {}", e))?;

            // Write the credentials
            writeln!(file, "# Gmail API OAuth2 credentials")
                .map_err(|e| format!("Failed to write to .env file: {}", e))?;
            writeln!(file, "GMAIL_CLIENT_ID={}", client_id)
                .map_err(|e| format!("Failed to write to .env file: {}", e))?;
            writeln!(file, "GMAIL_CLIENT_SECRET={}", client_secret)
                .map_err(|e| format!("Failed to write to .env file: {}", e))?;
            writeln!(file, "GMAIL_REFRESH_TOKEN={}", refresh_token)
                .map_err(|e| format!("Failed to write to .env file: {}", e))?;
            writeln!(file, "GMAIL_ACCESS_TOKEN={}", access_token)
                .map_err(|e| format!("Failed to write to .env file: {}", e))?;
            writeln!(file, "GMAIL_REDIRECT_URI={}", redirect_uri)
                .map_err(|e| format!("Failed to write to .env file: {}", e))?;
        }

        Ok(())
    }

    // Utility to test the saved credentials
    pub async fn test_credentials() -> Result<String, String> {
        // Load the config from environment
        let config =
            Config::from_env().map_err(|e| format!("Failed to load credentials: {}", e))?;

        // Create a Gmail service client
        let mut service = crate::gmail_api::GmailService::new(&config)
            .map_err(|e| format!("Failed to create Gmail service: {}", e))?;

        // Try to check the connection
        match service.check_connection().await {
            Ok((email, count)) => Ok(format!(
                "Successfully connected to Gmail for {}! Found {} messages.",
                email, count
            )),
            Err(e) => Err(format!("Failed to connect to Gmail: {}", e)),
        }
    }
}
