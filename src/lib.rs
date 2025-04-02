pub use crate::config::Config;
pub use crate::gmail_api::EmailMessage;
pub use crate::logging::setup_logging;
pub use crate::people_api::PeopleClient;
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
                // If we have an initial access token, respect the expiry_in if provided
                // or use a configurable default
                let default_expiry_seconds = std::env::var("TOKEN_EXPIRY_SECONDS")
                    .ok()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(600); // Default 10 minutes if not configured

                SystemTime::now() + Duration::from_secs(default_expiry_seconds)
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

    // Draft email model for creating new emails
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct DraftEmail {
        pub to: String,
        pub subject: String,
        pub body: String,
        pub cc: Option<String>,
        pub bcc: Option<String>,
        pub thread_id: Option<String>,
        pub in_reply_to: Option<String>,
        pub references: Option<String>,
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

        /// Create a draft email in Gmail
        pub async fn create_draft(&mut self, draft: &DraftEmail) -> Result<String> {
            debug!("Creating draft email to: {}", draft.to);

            // Construct the RFC 5322 formatted message
            let mut message = format!(
                "From: me\r\n\
                 To: {}\r\n\
                 Subject: {}\r\n",
                draft.to, draft.subject
            );

            // Add optional CC and BCC fields
            if let Some(cc) = &draft.cc {
                message.push_str(&format!("Cc: {}\r\n", cc));
            }

            if let Some(bcc) = &draft.bcc {
                message.push_str(&format!("Bcc: {}\r\n", bcc));
            }

            // Add threading headers for replies
            if let Some(in_reply_to) = &draft.in_reply_to {
                message.push_str(&format!("In-Reply-To: {}\r\n", in_reply_to));
            }

            if let Some(references) = &draft.references {
                message.push_str(&format!("References: {}\r\n", references));
            }

            // Add body
            message.push_str("\r\n");
            message.push_str(&draft.body);

            // Base64 encode the message
            let encoded_message = base64::encode(message.as_bytes())
                .replace('+', "-")
                .replace('/', "_");

            // Create the JSON payload
            let mut message_payload = serde_json::json!({
                "raw": encoded_message
            });

            // Add thread_id if specified
            if let Some(thread_id) = &draft.thread_id {
                message_payload = serde_json::json!({
                    "raw": encoded_message,
                    "threadId": thread_id
                });
            }

            let payload = serde_json::json!({
                "message": message_payload
            });

            // Make the request to create a draft
            let endpoint = "/users/me/drafts";

            // Get valid access token
            let token = self.token_manager.get_token(&self.client).await?;

            let url = format!("{}{}", GMAIL_API_BASE_URL, endpoint);
            debug!("Creating draft at: {}", url);

            // Send the request
            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await
                .map_err(|e| {
                    error!("Network error creating draft: {}", e);
                    GmailApiError::NetworkError(e.to_string())
                })?;

            // Handle response
            let status = response.status();
            debug!("Draft creation response status: {}", status);

            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<no response body>".to_string());

                error!("Failed to create draft: {}", error_text);
                return Err(GmailApiError::ApiError(format!(
                    "Failed to create draft. Status: {}, Error: {}",
                    status, error_text
                )));
            }

            // Parse the response to get the draft ID
            let response_text = response.text().await.map_err(|e| {
                error!("Failed to get response body: {}", e);
                GmailApiError::NetworkError(format!("Failed to get response body: {}", e))
            })?;

            // Parse the JSON response
            let response_json: serde_json::Value =
                serde_json::from_str(&response_text).map_err(|e| {
                    error!("Failed to parse draft response: {}", e);
                    GmailApiError::MessageFormatError(format!(
                        "Failed to parse draft response: {}",
                        e
                    ))
                })?;

            // Extract the draft ID
            let draft_id = response_json["id"]
                .as_str()
                .ok_or_else(|| {
                    GmailApiError::MessageFormatError(
                        "Draft response missing 'id' field".to_string(),
                    )
                })?
                .to_string();

            debug!("Draft created successfully with ID: {}", draft_id);

            Ok(draft_id)
        }
    }
}

// Module for logging configuration
pub mod logging {
    use chrono::Local;
    use log::LevelFilter;
    use simplelog::{self, CombinedLogger, TermLogger, WriteLogger};
    use std::fs::OpenOptions;
    use std::io::Write;

    /// Sets up logging to file and stderr
    ///
    /// # Arguments
    ///
    /// * `log_level` - The level of log messages to capture
    /// * `log_file` - Optional path to log file. If None, creates a timestamped file
    ///
    /// # Returns
    ///
    /// Sets up the logging system
    ///
    /// # Arguments
    ///
    /// * `log_level` - The level of logging to use
    /// * `log_file` - Optional log file name or "memory" to use in-memory logging
    ///
    /// # Returns
    ///
    /// The path to the log file or a description of the logging destination
    pub fn setup_logging(
        log_level: LevelFilter,
        log_file: Option<&str>,
    ) -> std::io::Result<String> {
        // Use the default config for simplicity - explicitly use simplelog::Config to avoid ambiguity
        let log_config = simplelog::Config::default();

        // Check if we should use memory-only logging
        if log_file == Some("memory") {
            // For memory-only logging, just use stderr
            TermLogger::init(
                log_level,
                log_config,
                simplelog::TerminalMode::Stderr,
                simplelog::ColorChoice::Auto,
            )
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            log::info!("Logging initialized to stderr only (memory mode)");
            log::debug!("Debug logging enabled");

            return Ok(String::from("stderr-only (memory mode)"));
        }

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
            .append(true)
            .open(&log_path)?;

        writeln!(
            log_file,
            "====== GMAIL MCP SERVER LOG - Started at {} ======",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        )?;

        // Setup loggers to write to both file and stderr
        CombinedLogger::init(vec![
            // File logger
            WriteLogger::new(log_level, log_config.clone(), log_file),
            // Terminal logger for stderr
            TermLogger::new(
                log_level,
                log_config,
                simplelog::TerminalMode::Stderr,
                simplelog::ColorChoice::Auto,
            ),
        ])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        log::info!("Logging initialized to file: {} and stderr", log_path);
        log::debug!("Debug logging enabled");

        Ok(log_path)
    }
}

// People API module for contact information
pub mod people_api {
    use crate::config::Config;
    use log::{debug, error};
    use reqwest::Client;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use thiserror::Error;
    use tokio::sync::Mutex;

    const PEOPLE_API_BASE_URL: &str = "https://people.googleapis.com/v1";

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

    type Result<T> = std::result::Result<T, PeopleApiError>;

    // Contact information representation
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Contact {
        pub resource_name: String,
        pub name: Option<PersonName>,
        pub email_addresses: Vec<EmailAddress>,
        pub phone_numbers: Vec<PhoneNumber>,
        pub organizations: Vec<Organization>,
        pub photos: Vec<Photo>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PersonName {
        pub display_name: String,
        pub given_name: Option<String>,
        pub family_name: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EmailAddress {
        pub value: String,
        pub type_: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PhoneNumber {
        pub value: String,
        pub type_: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Organization {
        pub name: Option<String>,
        pub title: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Photo {
        pub url: String,
        pub default: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ContactList {
        pub contacts: Vec<Contact>,
        pub next_page_token: Option<String>,
        pub total_items: Option<u32>,
    }

    // People API client
    #[derive(Debug, Clone)]
    pub struct PeopleClient {
        client: Client,
        token_manager: Arc<Mutex<crate::gmail_api::TokenManager>>,
    }

    impl PeopleClient {
        pub fn new(config: &Config) -> Self {
            let client = Client::new();
            // Reuse the Gmail token manager since they share the same OAuth flow
            let token_manager = Arc::new(Mutex::new(crate::gmail_api::TokenManager::new(config)));

            Self {
                client,
                token_manager,
            }
        }

        // Get a list of contacts
        pub async fn list_contacts(&self, max_results: Option<u32>) -> Result<ContactList> {
            let token = self
                .token_manager
                .lock()
                .await
                .get_token(&self.client)
                .await
                .map_err(|e| PeopleApiError::AuthError(e.to_string()))?;

            let mut url = format!("{}/people/me/connections", PEOPLE_API_BASE_URL);

            // Build query parameters
            let mut query_parts = Vec::new();

            // Request specific fields
            let fields = [
                "names",
                "emailAddresses",
                "phoneNumbers",
                "organizations",
                "photos",
            ];
            query_parts.push(format!("personFields={}", fields.join(",")));

            if let Some(max) = max_results {
                query_parts.push(format!("pageSize={}", max));
            }

            if !query_parts.is_empty() {
                url = format!("{}?{}", url, query_parts.join("&"));
            }

            debug!("Listing contacts from: {}", url);

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .map_err(|e| PeopleApiError::NetworkError(e.to_string()))?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<no response body>".to_string());
                return Err(PeopleApiError::ApiError(format!(
                    "Failed to list contacts. Status: {}, Error: {}",
                    status, error_text
                )));
            }

            let json_response = response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| PeopleApiError::ParseError(e.to_string()))?;

            let mut contacts = Vec::new();

            if let Some(connections) = json_response.get("connections").and_then(|v| v.as_array()) {
                for connection in connections {
                    if let Ok(contact) = self.parse_contact(connection) {
                        contacts.push(contact);
                    } else {
                        // Log parsing error but continue with other contacts
                        error!("Failed to parse contact: {:?}", connection);
                    }
                }
            }

            let next_page_token = json_response
                .get("nextPageToken")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let total_items = json_response
                .get("totalItems")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);

            Ok(ContactList {
                contacts,
                next_page_token,
                total_items,
            })
        }

        // Search contacts by query
        pub async fn search_contacts(
            &self,
            query: &str,
            max_results: Option<u32>,
        ) -> Result<ContactList> {
            let token = self
                .token_manager
                .lock()
                .await
                .get_token(&self.client)
                .await
                .map_err(|e| PeopleApiError::AuthError(e.to_string()))?;

            let mut url = format!("{}/people:searchContacts", PEOPLE_API_BASE_URL);

            // Build query parameters
            let mut query_parts = Vec::new();

            // Add search query
            query_parts.push(format!("query={}", query));

            // Request specific fields
            let fields = [
                "names",
                "emailAddresses",
                "phoneNumbers",
                "organizations",
                "photos",
            ];
            query_parts.push(format!("readMask={}", fields.join(",")));

            if let Some(max) = max_results {
                query_parts.push(format!("pageSize={}", max));
            }

            if !query_parts.is_empty() {
                url = format!("{}?{}", url, query_parts.join("&"));
            }

            debug!("Searching contacts: {}", url);

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .map_err(|e| PeopleApiError::NetworkError(e.to_string()))?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<no response body>".to_string());
                return Err(PeopleApiError::ApiError(format!(
                    "Failed to search contacts. Status: {}, Error: {}",
                    status, error_text
                )));
            }

            let json_response = response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| PeopleApiError::ParseError(e.to_string()))?;

            let mut contacts = Vec::new();

            if let Some(results) = json_response.get("results").and_then(|v| v.as_array()) {
                for result in results {
                    if let Some(person) = result.get("person") {
                        if let Ok(contact) = self.parse_contact(person) {
                            contacts.push(contact);
                        } else {
                            // Log parsing error but continue with other contacts
                            error!("Failed to parse contact: {:?}", person);
                        }
                    }
                }
            }

            let next_page_token = json_response
                .get("nextPageToken")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let total_items = json_response
                .get("totalPeople")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);

            Ok(ContactList {
                contacts,
                next_page_token,
                total_items,
            })
        }

        // Get contact by resource name
        pub async fn get_contact(&self, resource_name: &str) -> Result<Contact> {
            let token = self
                .token_manager
                .lock()
                .await
                .get_token(&self.client)
                .await
                .map_err(|e| PeopleApiError::AuthError(e.to_string()))?;

            let mut url = format!("{}/{}", PEOPLE_API_BASE_URL, resource_name);

            // Build query parameters for fields
            let fields = [
                "names",
                "emailAddresses",
                "phoneNumbers",
                "organizations",
                "photos",
            ];
            url = format!("{}?personFields={}", url, fields.join(","));

            debug!("Getting contact: {}", url);

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .map_err(|e| PeopleApiError::NetworkError(e.to_string()))?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<no response body>".to_string());
                return Err(PeopleApiError::ApiError(format!(
                    "Failed to get contact. Status: {}, Error: {}",
                    status, error_text
                )));
            }

            let json_response = response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| PeopleApiError::ParseError(e.to_string()))?;

            self.parse_contact(&json_response)
        }

        // Helper method to parse a contact from API response
        fn parse_contact(&self, data: &serde_json::Value) -> Result<Contact> {
            let resource_name = data
                .get("resourceName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| PeopleApiError::ParseError("Missing resourceName".to_string()))?
                .to_string();

            // Parse name
            let name = if let Some(names) = data.get("names").and_then(|v| v.as_array()) {
                if let Some(primary_name) = names.first() {
                    let display_name = primary_name
                        .get("displayName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    let given_name = primary_name
                        .get("givenName")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let family_name = primary_name
                        .get("familyName")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    Some(PersonName {
                        display_name,
                        given_name,
                        family_name,
                    })
                } else {
                    None
                }
            } else {
                None
            };

            // Parse email addresses
            let mut email_addresses = Vec::new();
            if let Some(emails) = data.get("emailAddresses").and_then(|v| v.as_array()) {
                for email in emails {
                    if let Some(value) = email.get("value").and_then(|v| v.as_str()) {
                        let type_ = email
                            .get("type")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        email_addresses.push(EmailAddress {
                            value: value.to_string(),
                            type_,
                        });
                    }
                }
            }

            // Parse phone numbers
            let mut phone_numbers = Vec::new();
            if let Some(phones) = data.get("phoneNumbers").and_then(|v| v.as_array()) {
                for phone in phones {
                    if let Some(value) = phone.get("value").and_then(|v| v.as_str()) {
                        let type_ = phone
                            .get("type")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        phone_numbers.push(PhoneNumber {
                            value: value.to_string(),
                            type_,
                        });
                    }
                }
            }

            // Parse organizations
            let mut organizations = Vec::new();
            if let Some(orgs) = data.get("organizations").and_then(|v| v.as_array()) {
                for org in orgs {
                    let name = org
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let title = org
                        .get("title")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    organizations.push(Organization { name, title });
                }
            }

            // Parse photos
            let mut photos = Vec::new();
            if let Some(pics) = data.get("photos").and_then(|v| v.as_array()) {
                for pic in pics {
                    if let Some(url) = pic.get("url").and_then(|v| v.as_str()) {
                        let default = pic
                            .get("default")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        photos.push(Photo {
                            url: url.to_string(),
                            default,
                        });
                    }
                }
            }

            Ok(Contact {
                resource_name,
                name,
                email_addresses,
                phone_numbers,
                organizations,
                photos,
            })
        }
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

    impl Default for GmailServer {
        fn default() -> Self {
            Self::new()
        }
    }

    impl GmailServer {
        pub fn new() -> Self {
            GmailServer {}
        }

        // Private method to initialize the Calendar service
        async fn init_calendar_service(
            &self,
        ) -> Result<crate::calendar_api::CalendarClient, McpError> {
            // Load the config
            let config = Config::from_env().map_err(|e| {
                error!("Failed to load OAuth configuration: {}", e);
                self.to_mcp_error(
                    &format!("Configuration error: {}", e),
                    error_codes::CONFIG_ERROR,
                )
            })?;

            // Create the calendar client
            Ok(crate::calendar_api::CalendarClient::new(&config))
        }

        // Private method to initialize the People API service
        async fn init_people_service(&self) -> Result<crate::people_api::PeopleClient, McpError> {
            // Load the config
            let config = Config::from_env().map_err(|e| {
                error!("Failed to load OAuth configuration: {}", e);
                self.to_mcp_error(
                    &format!("Configuration error: {}", e),
                    error_codes::CONFIG_ERROR,
                )
            })?;

            // Create the people client
            Ok(crate::people_api::PeopleClient::new(&config))
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
        async fn batch_analyze_emails(
            &self,
            message_ids: Vec<String>,
            analysis_type: Option<String>,
        ) -> McpResult<String> {
            info!("=== START batch_analyze_emails MCP command ===");
            debug!(
                "batch_analyze_emails called with {} messages, analysis_type={:?}",
                message_ids.len(),
                analysis_type
            );

            // Get the Gmail service
            let mut service = self.init_gmail_service().await?;

            // Determine what type of analysis to perform
            let analysis = analysis_type
                .unwrap_or_else(|| "summary".to_string())
                .to_lowercase();

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
                    }
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

        /// Create a draft email
        ///
        /// Creates a new draft email in Gmail with the specified content.
        /// The email will be saved as a draft and can be edited before sending.
        ///
        /// Args:
        ///   to: Email address(es) of the recipient(s). Multiple addresses should be comma-separated.
        ///   subject: Subject line of the email
        ///   body: Plain text content of the email
        ///   cc: Optional CC recipient(s). Multiple addresses should be comma-separated.
        ///   bcc: Optional BCC recipient(s). Multiple addresses should be comma-separated.
        ///   thread_id: Optional Gmail thread ID to associate this email with
        ///   in_reply_to: Optional Message-ID that this email is replying to
        ///   references: Optional comma-separated list of Message-IDs in the email thread
        #[tool]
        async fn create_draft_email(
            &self,
            to: String,
            subject: String,
            body: String,
            cc: Option<String>,
            bcc: Option<String>,
            thread_id: Option<String>,
            in_reply_to: Option<String>,
            references: Option<String>,
        ) -> McpResult<String> {
            info!("=== START create_draft_email MCP command ===");
            debug!(
                "create_draft_email called with to={}, subject={}, cc={:?}, bcc={:?}, thread_id={:?}, in_reply_to={:?}",
                to, subject, cc, bcc, thread_id, in_reply_to
            );

            // Validate email addresses
            if to.is_empty() {
                let error_msg = "Recipient (to) is required for creating a draft email";
                error!("{}", error_msg);
                return Err(self.to_mcp_error(error_msg, error_codes::MESSAGE_FORMAT_ERROR));
            }

            // Create the draft email object
            let draft = crate::gmail_api::DraftEmail {
                to,
                subject,
                body,
                cc,
                bcc,
                thread_id,
                in_reply_to,
                references,
            };

            // Get the Gmail service
            let mut service = self.init_gmail_service().await?;

            // Create the draft
            match service.create_draft(&draft).await {
                Ok(draft_id) => {
                    // Create success response
                    let mut result = json!({
                        "status": "success",
                        "draft_id": draft_id,
                        "message": "Draft email created successfully."
                    });

                    // Add threading info to response if provided
                    if let Some(ref thread_id_val) = draft.thread_id {
                        result["thread_id"] = json!(thread_id_val);
                    }

                    // Convert to string
                    let result_json = serde_json::to_string_pretty(&result).map_err(|e| {
                        let error_msg = format!("Failed to serialize draft creation result: {}", e);
                        error!("{}", error_msg);
                        self.to_mcp_error(&error_msg, error_codes::MESSAGE_FORMAT_ERROR)
                    })?;

                    info!("=== END create_draft_email MCP command (success) ===");
                    Ok(result_json)
                }
                Err(err) => {
                    error!("Failed to create draft email: {}", err);

                    // Create detailed error context for the user
                    error!(
                        "Context: Failed to create draft email with subject: '{}'",
                        draft.subject
                    );

                    Err(self.map_gmail_error(err))
                }
            }
        }

        /// List contacts
        ///
        /// This command retrieves a list of contacts from Google Contacts.
        ///
        /// # Parameters
        ///
        /// * `max_results` - Optional. The maximum number of contacts to return.
        ///
        /// # Returns
        ///
        /// A JSON string containing the contact list
        #[tool]
        async fn list_contacts(&self, max_results: Option<u32>) -> McpResult<String> {
            info!("=== START list_contacts MCP command ===");
            debug!("list_contacts called with max_results={:?}", max_results);

            // Initialize the People API client
            let people_client = self.init_people_service().await?;

            match people_client.list_contacts(max_results).await {
                Ok(contacts) => {
                    // Convert to JSON
                    serde_json::to_string(&contacts).map_err(|e| {
                        let error_msg = format!("Failed to serialize contact list: {}", e);
                        error!("{}", error_msg);
                        self.to_mcp_error(&error_msg, error_codes::GENERAL_ERROR)
                    })
                }
                Err(err) => {
                    error!("Failed to list contacts: {}", err);
                    Err(self.to_mcp_error(
                        &format!("Failed to list contacts: {}", err),
                        error_codes::API_ERROR,
                    ))
                }
            }
        }

        /// Search contacts
        ///
        /// This command searches for contacts matching the query.
        ///
        /// # Parameters
        ///
        /// * `query` - The search query.
        /// * `max_results` - Optional. The maximum number of contacts to return.
        ///
        /// # Returns
        ///
        /// A JSON string containing the matching contacts
        #[tool]
        async fn search_contacts(
            &self,
            query: String,
            max_results: Option<u32>,
        ) -> McpResult<String> {
            info!("=== START search_contacts MCP command ===");
            debug!(
                "search_contacts called with query=\"{}\" and max_results={:?}",
                query, max_results
            );

            // Initialize the People API client
            let people_client = self.init_people_service().await?;

            match people_client.search_contacts(&query, max_results).await {
                Ok(contacts) => {
                    // Convert to JSON
                    serde_json::to_string(&contacts).map_err(|e| {
                        let error_msg =
                            format!("Failed to serialize contact search results: {}", e);
                        error!("{}", error_msg);
                        self.to_mcp_error(&error_msg, error_codes::GENERAL_ERROR)
                    })
                }
                Err(err) => {
                    error!("Failed to search contacts: {}", err);
                    Err(self.to_mcp_error(
                        &format!("Failed to search contacts: {}", err),
                        error_codes::API_ERROR,
                    ))
                }
            }
        }

        /// Get contact
        ///
        /// This command retrieves a specific contact by resource name.
        ///
        /// # Parameters
        ///
        /// * `resource_name` - The resource name of the contact to retrieve.
        ///
        /// # Returns
        ///
        /// A JSON string containing the contact details
        #[tool]
        async fn get_contact(&self, resource_name: String) -> McpResult<String> {
            info!("=== START get_contact MCP command ===");
            debug!("get_contact called with resource_name={}", resource_name);

            // Initialize the People API client
            let people_client = self.init_people_service().await?;

            match people_client.get_contact(&resource_name).await {
                Ok(contact) => {
                    // Convert to JSON
                    serde_json::to_string(&contact).map_err(|e| {
                        let error_msg = format!("Failed to serialize contact: {}", e);
                        error!("{}", error_msg);
                        self.to_mcp_error(&error_msg, error_codes::GENERAL_ERROR)
                    })
                }
                Err(err) => {
                    error!("Failed to get contact: {}", err);
                    Err(self.to_mcp_error(
                        &format!("Failed to get contact: {}", err),
                        error_codes::API_ERROR,
                    ))
                }
            }
        }

        /// List all available calendars
        ///
        /// This command retrieves a list of all calendars the user has access to.
        ///
        /// # Returns
        ///
        /// A JSON string containing the calendar list
        #[tool]
        async fn list_calendars(&self) -> McpResult<String> {
            info!("=== START list_calendars MCP command ===");
            debug!("list_calendars called");

            // Initialize the calendar service
            let service = self.init_calendar_service().await?;

            // Get the calendars
            match service.list_calendars().await {
                Ok(calendars) => {
                    // Convert to JSON
                    serde_json::to_string(&calendars).map_err(|e| {
                        let error_msg = format!("Failed to serialize calendar list: {}", e);
                        error!("{}", error_msg);
                        self.to_mcp_error(&error_msg, error_codes::MESSAGE_FORMAT_ERROR)
                    })
                }
                Err(err) => {
                    error!("Failed to list calendars: {}", err);
                    Err(self.to_mcp_error(
                        &format!("Failed to list calendars: {}", err),
                        error_codes::API_ERROR,
                    ))
                }
            }
        }

        /// List events from a calendar
        ///
        /// This command retrieves events from a specified calendar, with options for filtering.
        ///
        /// # Arguments
        ///
        /// * `calendar_id` - The ID of the calendar to get events from (optional, defaults to primary)
        /// * `max_results` - Optional maximum number of events to return
        /// * `time_min` - Optional minimum time bound (RFC3339 timestamp)
        /// * `time_max` - Optional maximum time bound (RFC3339 timestamp)
        ///
        /// # Returns
        ///
        /// A JSON string containing the event list
        #[tool]
        async fn list_events(
            &self,
            calendar_id: Option<String>,
            max_results: Option<serde_json::Value>,
            time_min: Option<String>,
            time_max: Option<String>,
        ) -> McpResult<String> {
            info!("=== START list_events MCP command ===");
            debug!(
                "list_events called with calendar_id={:?}, max_results={:?}, time_min={:?}, time_max={:?}",
                calendar_id, max_results, time_min, time_max
            );

            // Use primary calendar if not specified
            let calendar_id = calendar_id.unwrap_or_else(|| "primary".to_string());

            // Convert max_results using the helper function (default: 10)
            let max = helpers::parse_max_results(max_results, 10);

            // Parse time bounds if provided
            let time_min_parsed = if let Some(t) = time_min {
                match chrono::DateTime::parse_from_rfc3339(&t) {
                    Ok(dt) => Some(dt.with_timezone(&chrono::Utc)),
                    Err(e) => {
                        let error_msg =
                            format!("Invalid time_min format (expected RFC3339): {}", e);
                        error!("{}", error_msg);
                        return Err(self.to_mcp_error(&error_msg, error_codes::API_ERROR));
                    }
                }
            } else {
                None
            };

            let time_max_parsed = if let Some(t) = time_max {
                match chrono::DateTime::parse_from_rfc3339(&t) {
                    Ok(dt) => Some(dt.with_timezone(&chrono::Utc)),
                    Err(e) => {
                        let error_msg =
                            format!("Invalid time_max format (expected RFC3339): {}", e);
                        error!("{}", error_msg);
                        return Err(self.to_mcp_error(&error_msg, error_codes::API_ERROR));
                    }
                }
            } else {
                None
            };

            // Initialize the calendar service
            let service = self.init_calendar_service().await?;

            // Get the events
            match service
                .list_events(&calendar_id, Some(max), time_min_parsed, time_max_parsed)
                .await
            {
                Ok(events) => {
                    // Convert to JSON
                    serde_json::to_string(&events).map_err(|e| {
                        let error_msg = format!("Failed to serialize events list: {}", e);
                        error!("{}", error_msg);
                        self.to_mcp_error(&error_msg, error_codes::MESSAGE_FORMAT_ERROR)
                    })
                }
                Err(err) => {
                    error!(
                        "Failed to list events from calendar {}: {}",
                        calendar_id, err
                    );
                    Err(self.to_mcp_error(
                        &format!(
                            "Failed to list events from calendar {}: {}",
                            calendar_id, err
                        ),
                        error_codes::API_ERROR,
                    ))
                }
            }
        }

        /// Get a single calendar event
        ///
        /// This command retrieves a specific event from a calendar.
        ///
        /// # Arguments
        ///
        /// * `calendar_id` - The ID of the calendar (optional, defaults to primary)
        /// * `event_id` - The ID of the event to retrieve
        ///
        /// # Returns
        ///
        /// A JSON string containing the event details
        #[tool]
        async fn get_event(
            &self,
            calendar_id: Option<String>,
            event_id: String,
        ) -> McpResult<String> {
            info!("=== START get_event MCP command ===");
            debug!(
                "get_event called with calendar_id={:?}, event_id={}",
                calendar_id, event_id
            );

            // Use primary calendar if not specified
            let calendar_id = calendar_id.unwrap_or_else(|| "primary".to_string());

            // Initialize the calendar service
            let service = self.init_calendar_service().await?;

            // Get the event
            match service.get_event(&calendar_id, &event_id).await {
                Ok(event) => {
                    // Convert to JSON
                    serde_json::to_string(&event).map_err(|e| {
                        let error_msg = format!("Failed to serialize event: {}", e);
                        error!("{}", error_msg);
                        self.to_mcp_error(&error_msg, error_codes::MESSAGE_FORMAT_ERROR)
                    })
                }
                Err(err) => {
                    error!(
                        "Failed to get event {} from calendar {}: {}",
                        event_id, calendar_id, err
                    );
                    Err(self.to_mcp_error(
                        &format!(
                            "Failed to get event {} from calendar {}: {}",
                            event_id, calendar_id, err
                        ),
                        error_codes::API_ERROR,
                    ))
                }
            }
        }

        /// Create a new calendar event
        ///
        /// This command creates a new event in the specified calendar.
        ///
        /// # Arguments
        ///
        /// * `calendar_id` - The ID of the calendar (optional, defaults to primary)
        /// * `summary` - The title of the event
        /// * `description` - Optional description of the event
        /// * `location` - Optional location of the event
        /// * `start_time` - Start time in RFC3339 format
        /// * `end_time` - End time in RFC3339 format
        /// * `attendees` - Optional list of attendee emails
        ///
        /// # Returns
        ///
        /// A JSON string containing the created event details
        #[tool]
        async fn create_event(
            &self,
            calendar_id: Option<String>,
            summary: String,
            description: Option<String>,
            location: Option<String>,
            start_time: String,
            end_time: String,
            attendees: Option<Vec<String>>,
        ) -> McpResult<String> {
            info!("=== START create_event MCP command ===");
            debug!(
                "create_event called with calendar_id={:?}, summary={}, description={:?}, location={:?}, start_time={}, end_time={}, attendees={:?}",
                calendar_id, summary, description, location, start_time, end_time, attendees
            );

            // Use primary calendar if not specified
            let calendar_id = calendar_id.unwrap_or_else(|| "primary".to_string());

            // Parse start and end times
            let start_dt = match chrono::DateTime::parse_from_rfc3339(&start_time) {
                Ok(dt) => dt.with_timezone(&chrono::Utc),
                Err(e) => {
                    let error_msg = format!("Invalid start_time format (expected RFC3339): {}", e);
                    error!("{}", error_msg);
                    return Err(self.to_mcp_error(&error_msg, error_codes::API_ERROR));
                }
            };

            let end_dt = match chrono::DateTime::parse_from_rfc3339(&end_time) {
                Ok(dt) => dt.with_timezone(&chrono::Utc),
                Err(e) => {
                    let error_msg = format!("Invalid end_time format (expected RFC3339): {}", e);
                    error!("{}", error_msg);
                    return Err(self.to_mcp_error(&error_msg, error_codes::API_ERROR));
                }
            };

            // Create attendee objects from email strings
            let attendee_objs = attendees
                .unwrap_or_default()
                .into_iter()
                .map(|email| crate::calendar_api::Attendee {
                    email,
                    display_name: None,
                    response_status: Some("needsAction".to_string()),
                    optional: None,
                })
                .collect();

            // Create the event
            let event = crate::calendar_api::CalendarEvent {
                id: None,
                summary,
                description,
                location,
                start_time: start_dt,
                end_time: end_dt,
                attendees: attendee_objs,
                conference_data: None,
                html_link: None,
                creator: None,
                organizer: None,
            };

            // Initialize the calendar service
            let service = self.init_calendar_service().await?;

            // Create the event
            match service.create_event(&calendar_id, event).await {
                Ok(created_event) => {
                    // Convert to JSON
                    serde_json::to_string(&created_event).map_err(|e| {
                        let error_msg = format!("Failed to serialize created event: {}", e);
                        error!("{}", error_msg);
                        self.to_mcp_error(&error_msg, error_codes::MESSAGE_FORMAT_ERROR)
                    })
                }
                Err(err) => {
                    error!(
                        "Failed to create event in calendar {}: {}",
                        calendar_id, err
                    );
                    Err(self.to_mcp_error(
                        &format!(
                            "Failed to create event in calendar {}: {}",
                            calendar_id, err
                        ),
                        error_codes::API_ERROR,
                    ))
                }
            }
        }
    }
}

// Module with prompts for MCP
pub mod prompts {
    /// Gmail Assistant Prompts
    ///
    /// These prompts help Claude understand how to interact with Gmail data
    /// through the MCP server tools and provide useful analysis.
    ///
    /// Master prompt for system context
    pub const GMAIL_MASTER_PROMPT: &str = r#"
# Gmail Assistant

You have access to email data through a Gmail MCP server. Your role is to help users manage, analyze, and extract insights from their emails. You can search emails, read messages, provide summaries and analyses, and create draft emails.

## Capabilities
- List and search emails with various criteria
- Get detailed content of specific emails
- Analyze email content, sentiment, and context
- Extract action items, summaries, and key points
- Create draft emails that can be edited before sending

## Important Notes
- Handle email data with privacy and security in mind
- Format email data in a readable way
- Highlight important information from emails
- Extract action items and tasks when relevant
- When creating draft emails, follow best email writing practices
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
    use serde_json;
    use std::collections::HashMap;
    use std::env;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::net::SocketAddr;
    use std::path::Path;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use url::Url;

    // OAuth scopes needed for Gmail, Calendar, and People API access
    const GMAIL_SCOPE: &str = "https://mail.google.com/";
    const CALENDAR_READ_SCOPE: &str = "https://www.googleapis.com/auth/calendar.readonly";
    const CALENDAR_WRITE_SCOPE: &str = "https://www.googleapis.com/auth/calendar";
    const CONTACTS_READ_SCOPE: &str = "https://www.googleapis.com/auth/contacts.readonly";
    const DIRECTORY_READ_SCOPE: &str = "https://www.googleapis.com/auth/directory.readonly";
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

        // Build the authorization URL with Gmail, Calendar, and People API scopes
        let auth_url = build_auth_url(
            &client_id,
            &redirect_uri,
            &state_token,
            &[
                GMAIL_SCOPE.to_string(),
                CALENDAR_READ_SCOPE.to_string(),
                CALENDAR_WRITE_SCOPE.to_string(),
                CONTACTS_READ_SCOPE.to_string(),
                DIRECTORY_READ_SCOPE.to_string(),
            ],
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

        println!("\n🎉 Authentication successful!");
        println!("✅ New tokens have been saved to .env file");
        println!("✅ Claude Desktop config saved to claude_desktop_config.json");

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

    // Update the .env file with the new tokens and generate Claude Desktop config
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

            // Create a backup of the .env file
            let backup_path = format!(
                ".env.backup.{}",
                chrono::Local::now().format("%Y%m%d_%H%M%S")
            );
            std::fs::write(&backup_path, &content)
                .map_err(|e| format!("Failed to create backup file {}: {}", backup_path, e))?;
            println!("✅ Created backup of .env file at {}", backup_path);

            // Ask for confirmation before proceeding
            println!("⚠️ About to update .env file with new OAuth credentials.");
            println!("🔄 Press Enter to continue or Ctrl+C to abort...");
            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).is_err() {
                println!("❌ Failed to read input, continuing anyway");
            }

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

        // Also generate the Claude Desktop config file
        generate_claude_desktop_config(client_id, client_secret, refresh_token, access_token)
            .map_err(|e| format!("Failed to create Claude Desktop config: {}", e))?;

        Ok(())
    }

    // Generate the Claude Desktop configuration file
    fn generate_claude_desktop_config(
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
        access_token: &str,
    ) -> Result<(), String> {
        use serde_json::{json, to_string_pretty};

        // Determine the executable path
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?;

        // Get the target/release version of the path if possible
        let mut command_path = current_exe.to_string_lossy().to_string();
        if let Some(debug_index) = command_path.find("target/debug") {
            // If we're running in debug mode, use the release path for the config
            command_path = format!(
                "{}target/release/mcp-gmailcal",
                &command_path[0..debug_index]
            );
        }

        // Create the config JSON
        let config = json!({
            "mcpServers": {
                "gmailcal": {
                    "command": command_path,
                    "args": ["--memory-only"],
                    "env": {
                        "GMAIL_CLIENT_ID": client_id,
                        "GMAIL_CLIENT_SECRET": client_secret,
                        "GMAIL_REFRESH_TOKEN": refresh_token,
                        "GMAIL_ACCESS_TOKEN": access_token
                    }
                }
            }
        });

        // Convert to pretty JSON
        let json_string =
            to_string_pretty(&config).map_err(|e| format!("Failed to serialize config: {}", e))?;

        // Write to file
        let config_path = "claude_desktop_config.json";
        std::fs::write(config_path, json_string)
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        println!("Claude Desktop config saved to {}", config_path);

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

// Module for Google Calendar API integration
pub mod calendar_api {
    use crate::config::Config;
    use chrono::{DateTime, Utc};
    use log::{debug, error};
    use reqwest::Client;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use thiserror::Error;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    const CALENDAR_API_BASE_URL: &str = "https://www.googleapis.com/calendar/v3";

    #[derive(Debug, Error)]
    pub enum CalendarApiError {
        #[error("Network error: {0}")]
        NetworkError(String),

        #[error("Authentication error: {0}")]
        AuthError(String),

        #[error("Calendar API error: {0}")]
        ApiError(String),

        #[error("Invalid input: {0}")]
        InvalidInput(String),

        #[error("Parse error: {0}")]
        ParseError(String),
    }

    type Result<T> = std::result::Result<T, CalendarApiError>;

    // Calendar event representation
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CalendarEvent {
        pub id: Option<String>,
        pub summary: String,
        pub description: Option<String>,
        pub location: Option<String>,
        pub start_time: DateTime<Utc>,
        pub end_time: DateTime<Utc>,
        pub attendees: Vec<Attendee>,
        pub conference_data: Option<ConferenceData>,
        pub html_link: Option<String>,
        pub creator: Option<EventOrganizer>,
        pub organizer: Option<EventOrganizer>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EventOrganizer {
        pub email: String,
        pub display_name: Option<String>,
        pub self_: Option<bool>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Attendee {
        pub email: String,
        pub display_name: Option<String>,
        pub response_status: Option<String>,
        pub optional: Option<bool>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ConferenceData {
        pub conference_solution: Option<ConferenceSolution>,
        pub entry_points: Vec<EntryPoint>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ConferenceSolution {
        pub name: String,
        pub key: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EntryPoint {
        pub entry_point_type: String,
        pub uri: String,
        pub label: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CalendarList {
        pub calendars: Vec<CalendarInfo>,
        pub next_page_token: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CalendarInfo {
        pub id: String,
        pub summary: String,
        pub description: Option<String>,
        pub primary: Option<bool>,
    }

    // Calendar API client
    #[derive(Debug, Clone)]
    pub struct CalendarClient {
        client: Client,
        token_manager: Arc<Mutex<crate::gmail_api::TokenManager>>,
    }

    impl CalendarClient {
        pub fn new(config: &Config) -> Self {
            let client = Client::new();
            // Reuse the Gmail token manager since they share the same OAuth scope
            let token_manager = Arc::new(Mutex::new(crate::gmail_api::TokenManager::new(config)));

            Self {
                client,
                token_manager,
            }
        }

        // Get a list of all calendars
        pub async fn list_calendars(&self) -> Result<CalendarList> {
            let token = self
                .token_manager
                .lock()
                .await
                .get_token(&self.client)
                .await
                .map_err(|e| CalendarApiError::AuthError(e.to_string()))?;

            let url = format!("{}/users/me/calendarList", CALENDAR_API_BASE_URL);
            debug!("Listing calendars from: {}", url);

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .map_err(|e| CalendarApiError::NetworkError(e.to_string()))?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<no response body>".to_string());
                return Err(CalendarApiError::ApiError(format!(
                    "Failed to list calendars. Status: {}, Error: {}",
                    status, error_text
                )));
            }

            let json_response = response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| CalendarApiError::ParseError(e.to_string()))?;

            let mut calendars = Vec::new();

            if let Some(items) = json_response.get("items").and_then(|v| v.as_array()) {
                for item in items {
                    let id = item
                        .get("id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            CalendarApiError::ParseError("Missing calendar id".to_string())
                        })?
                        .to_string();

                    let summary = item
                        .get("summary")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown Calendar")
                        .to_string();

                    let description = item
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let primary = item.get("primary").and_then(|v| v.as_bool());

                    calendars.push(CalendarInfo {
                        id,
                        summary,
                        description,
                        primary,
                    });
                }
            }

            let next_page_token = json_response
                .get("nextPageToken")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            Ok(CalendarList {
                calendars,
                next_page_token,
            })
        }

        // Get events from a specific calendar
        pub async fn list_events(
            &self,
            calendar_id: &str,
            max_results: Option<u32>,
            time_min: Option<DateTime<Utc>>,
            time_max: Option<DateTime<Utc>>,
        ) -> Result<Vec<CalendarEvent>> {
            let token = self
                .token_manager
                .lock()
                .await
                .get_token(&self.client)
                .await
                .map_err(|e| CalendarApiError::AuthError(e.to_string()))?;

            let mut url = format!("{}/calendars/{}/events", CALENDAR_API_BASE_URL, calendar_id);

            // Build query parameters
            let mut query_parts = Vec::new();

            if let Some(max) = max_results {
                query_parts.push(format!("maxResults={}", max));
            }

            if let Some(min_time) = time_min {
                query_parts.push(format!("timeMin={}", min_time.to_rfc3339()));
            }

            if let Some(max_time) = time_max {
                query_parts.push(format!("timeMax={}", max_time.to_rfc3339()));
            }

            // Add single events mode to expand recurring events
            query_parts.push("singleEvents=true".to_string());

            // Order by start time
            query_parts.push("orderBy=startTime".to_string());

            if !query_parts.is_empty() {
                url = format!("{}?{}", url, query_parts.join("&"));
            }

            debug!("Listing events from: {}", url);

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .map_err(|e| CalendarApiError::NetworkError(e.to_string()))?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<no response body>".to_string());
                return Err(CalendarApiError::ApiError(format!(
                    "Failed to list events. Status: {}, Error: {}",
                    status, error_text
                )));
            }

            let json_response = response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| CalendarApiError::ParseError(e.to_string()))?;

            let mut events = Vec::new();

            if let Some(items) = json_response.get("items").and_then(|v| v.as_array()) {
                for item in items {
                    if let Ok(event) = self.parse_event(item) {
                        events.push(event);
                    } else {
                        // Log parsing error but continue with other events
                        error!("Failed to parse event: {:?}", item);
                    }
                }
            }

            Ok(events)
        }

        // Create a new calendar event
        pub async fn create_event(
            &self,
            calendar_id: &str,
            event: CalendarEvent,
        ) -> Result<CalendarEvent> {
            let token = self
                .token_manager
                .lock()
                .await
                .get_token(&self.client)
                .await
                .map_err(|e| CalendarApiError::AuthError(e.to_string()))?;

            let url = format!("{}/calendars/{}/events", CALENDAR_API_BASE_URL, calendar_id);
            debug!("Creating new event in calendar {}", calendar_id);

            // Convert our CalendarEvent to Google Calendar API format
            let mut event_data = serde_json::Map::new();
            event_data.insert(
                "summary".to_string(),
                serde_json::Value::String(event.summary),
            );

            if let Some(desc) = event.description {
                event_data.insert("description".to_string(), serde_json::Value::String(desc));
            }

            if let Some(loc) = event.location {
                event_data.insert("location".to_string(), serde_json::Value::String(loc));
            }

            // Add start time
            let mut start = serde_json::Map::new();
            start.insert(
                "dateTime".to_string(),
                serde_json::Value::String(event.start_time.to_rfc3339()),
            );
            start.insert(
                "timeZone".to_string(),
                serde_json::Value::String("UTC".to_string()),
            );
            event_data.insert("start".to_string(), serde_json::Value::Object(start));

            // Add end time
            let mut end = serde_json::Map::new();
            end.insert(
                "dateTime".to_string(),
                serde_json::Value::String(event.end_time.to_rfc3339()),
            );
            end.insert(
                "timeZone".to_string(),
                serde_json::Value::String("UTC".to_string()),
            );
            event_data.insert("end".to_string(), serde_json::Value::Object(end));

            // Add attendees if any
            if !event.attendees.is_empty() {
                let attendees = event
                    .attendees
                    .iter()
                    .map(|a| {
                        let mut attendee = serde_json::Map::new();
                        attendee.insert(
                            "email".to_string(),
                            serde_json::Value::String(a.email.clone()),
                        );

                        if let Some(name) = &a.display_name {
                            attendee.insert(
                                "displayName".to_string(),
                                serde_json::Value::String(name.clone()),
                            );
                        }

                        if let Some(status) = &a.response_status {
                            attendee.insert(
                                "responseStatus".to_string(),
                                serde_json::Value::String(status.clone()),
                            );
                        }

                        if let Some(optional) = a.optional {
                            attendee
                                .insert("optional".to_string(), serde_json::Value::Bool(optional));
                        }

                        serde_json::Value::Object(attendee)
                    })
                    .collect::<Vec<_>>();

                event_data.insert("attendees".to_string(), serde_json::Value::Array(attendees));
            }

            // Generate unique ID for request for idempotency
            let request_id = Uuid::new_v4().to_string();

            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                // Add idempotency header
                .header("X-Goog-Request-ID", request_id)
                .json(&event_data)
                .send()
                .await
                .map_err(|e| CalendarApiError::NetworkError(e.to_string()))?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<no response body>".to_string());
                return Err(CalendarApiError::ApiError(format!(
                    "Failed to create event. Status: {}, Error: {}",
                    status, error_text
                )));
            }

            let json_response = response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| CalendarApiError::ParseError(e.to_string()))?;

            self.parse_event(&json_response)
        }

        // Get a specific event
        pub async fn get_event(&self, calendar_id: &str, event_id: &str) -> Result<CalendarEvent> {
            let token = self
                .token_manager
                .lock()
                .await
                .get_token(&self.client)
                .await
                .map_err(|e| CalendarApiError::AuthError(e.to_string()))?;

            let url = format!(
                "{}/calendars/{}/events/{}",
                CALENDAR_API_BASE_URL, calendar_id, event_id
            );
            debug!("Getting event {} from calendar {}", event_id, calendar_id);

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .map_err(|e| CalendarApiError::NetworkError(e.to_string()))?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<no response body>".to_string());
                return Err(CalendarApiError::ApiError(format!(
                    "Failed to get event. Status: {}, Error: {}",
                    status, error_text
                )));
            }

            let json_response = response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| CalendarApiError::ParseError(e.to_string()))?;

            self.parse_event(&json_response)
        }

        // Helper to parse Google Calendar event format into our CalendarEvent struct
        fn parse_event(&self, item: &serde_json::Value) -> Result<CalendarEvent> {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let summary = item
                .get("summary")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CalendarApiError::ParseError("Missing event summary".to_string()))?
                .to_string();

            let description = item
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let location = item
                .get("location")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Parse datetime structures
            let start_time = item
                .get("start")
                .and_then(|v| v.get("dateTime"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| CalendarApiError::ParseError("Missing start time".to_string()))?;

            let end_time = item
                .get("end")
                .and_then(|v| v.get("dateTime"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| CalendarApiError::ParseError("Missing end time".to_string()))?;

            // Parse RFC3339 format to DateTime<Utc>
            let start_dt = DateTime::parse_from_rfc3339(start_time)
                .map_err(|e| CalendarApiError::ParseError(format!("Invalid start time: {}", e)))?
                .with_timezone(&Utc);

            let end_dt = DateTime::parse_from_rfc3339(end_time)
                .map_err(|e| CalendarApiError::ParseError(format!("Invalid end time: {}", e)))?
                .with_timezone(&Utc);

            // Parse attendees
            let mut attendees = Vec::new();
            if let Some(attendee_list) = item.get("attendees").and_then(|v| v.as_array()) {
                for attendee in attendee_list {
                    let email = attendee
                        .get("email")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            CalendarApiError::ParseError("Missing attendee email".to_string())
                        })?
                        .to_string();

                    let display_name = attendee
                        .get("displayName")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let response_status = attendee
                        .get("responseStatus")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let optional = attendee.get("optional").and_then(|v| v.as_bool());

                    attendees.push(Attendee {
                        email,
                        display_name,
                        response_status,
                        optional,
                    });
                }
            }

            // Parse conference data
            let conference_data = if let Some(conf_data) = item.get("conferenceData") {
                let mut entry_points = Vec::new();

                if let Some(entry_point_list) =
                    conf_data.get("entryPoints").and_then(|v| v.as_array())
                {
                    for entry_point in entry_point_list {
                        if let (Some(entry_type), Some(uri)) = (
                            entry_point.get("entryPointType").and_then(|v| v.as_str()),
                            entry_point.get("uri").and_then(|v| v.as_str()),
                        ) {
                            entry_points.push(EntryPoint {
                                entry_point_type: entry_type.to_string(),
                                uri: uri.to_string(),
                                label: entry_point
                                    .get("label")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                            });
                        }
                    }
                }

                let conference_solution = conf_data.get("conferenceSolution").and_then(|sol| {
                    sol.get("name")
                        .and_then(|v| v.as_str())
                        .map(|name| ConferenceSolution {
                            name: name.to_string(),
                            key: sol
                                .get("key")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                        })
                });

                if !entry_points.is_empty() || conference_solution.is_some() {
                    Some(ConferenceData {
                        conference_solution,
                        entry_points,
                    })
                } else {
                    None
                }
            } else {
                None
            };

            // Parse html link
            let html_link = item
                .get("htmlLink")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Parse creator
            let creator = item.get("creator").and_then(|c| {
                c.get("email")
                    .and_then(|v| v.as_str())
                    .map(|email| EventOrganizer {
                        email: email.to_string(),
                        display_name: c
                            .get("displayName")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        self_: c.get("self").and_then(|v| v.as_bool()),
                    })
            });

            // Parse organizer
            let organizer = item.get("organizer").and_then(|o| {
                o.get("email")
                    .and_then(|v| v.as_str())
                    .map(|email| EventOrganizer {
                        email: email.to_string(),
                        display_name: o
                            .get("displayName")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        self_: o.get("self").and_then(|v| v.as_bool()),
                    })
            });

            Ok(CalendarEvent {
                id,
                summary,
                description,
                location,
                start_time: start_dt,
                end_time: end_dt,
                attendees,
                conference_data,
                html_link,
                creator,
                organizer,
            })
        }
    }
}
