pub use crate::config::Config;
pub use crate::gmail_custom::deserialize_custom_message;
pub use crate::gmail_service::EmailMessage;
pub use crate::logging::setup_logging;
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

// Gmail service module
pub mod gmail_service {
    use crate::config::Config;
    use gmail::model::Message;
    use gmail::GmailClient;
    use log::{debug, error, info};
    use serde::{Deserialize, Serialize};
    use thiserror::Error;

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
    }

    impl EmailMessage {
        pub fn from_gmail_message(message: Message) -> Self {
            debug!(
                "Converting Gmail Message to EmailMessage for ID: {}",
                message.id
            );

            // Initialize header values
            let mut subject = None;
            let mut from = None;
            let mut to = None;
            let mut date = None;

            // Extract headers if payload.headers exists and is not empty
            for header in &message.payload.headers {
                match header.name.as_str() {
                    "Subject" => {
                        debug!("Found Subject header: {}", header.value);
                        subject = Some(header.value.clone());
                    }
                    "From" => {
                        debug!("Found From header: {}", header.value);
                        from = Some(header.value.clone());
                    }
                    "To" => {
                        debug!("Found To header: {}", header.value);
                        to = Some(header.value.clone());
                    }
                    "Date" => {
                        debug!("Found Date header: {}", header.value);
                        date = Some(header.value.clone());
                    }
                    _ => {} // Ignore other headers
                }
            }

            // Get snippet
            let snippet = if message.snippet.is_empty() {
                None
            } else {
                Some(message.snippet)
            };

            EmailMessage {
                id: message.id,
                thread_id: message.thread_id,
                subject,
                from,
                to,
                date,
                snippet,
            }
        }
    }

    // Gmail service error types
    #[derive(Debug, Error)]
    pub enum GmailServiceError {
        #[error("Gmail API error: {0}")]
        ApiError(String),

        #[error("Authentication error: {0}")]
        AuthError(String),
    }

    pub type Result<T> = std::result::Result<T, GmailServiceError>;

    pub struct GmailService {
        client: GmailClient,
    }

    impl GmailService {
        pub fn new(config: &Config) -> Result<Self> {
            debug!("Creating new GmailService with config");
            let client = create_client(config)?;
            Ok(Self { client })
        }

        /// Attempts to retrieve a message using the standard "full" format
        async fn get_message_standard_format(&self, message_id: &str) -> Result<Message> {
            debug!("Attempting to retrieve message with standard format: {}", message_id);
            
            // Try with "full" format
            let result = self
                .client
                .messages_get(message_id, "me")
                .format("full")
                .await;
                
            match result {
                Ok(message) => {
                    debug!("Successfully retrieved message with standard approach");
                    debug!(
                        "Message ID: {}, Thread ID: {}",
                        message.id, message.thread_id
                    );
                    debug!("Has internalDate: {}", !message.internal_date.is_empty());
                    debug!("Labels count: {}", message.label_ids.len());
                    debug!("Headers count: {}", message.payload.headers.len());

                    // Import the MessageExt trait
                    use crate::gmail_custom::MessageExt;
                    
                    // Ensure all required fields have values
                    let message = message.ensure_required_fields();
                    Ok(message)
                }
                Err(e) => Err(GmailServiceError::ApiError(e.to_string()))
            }
        }
        
        /// Checks if an error is related to missing fields in the API response
        fn is_missing_field_error(&self, error: &str) -> bool {
            error.contains("missing field")
                || error.contains("unknown field")
                || error.contains("missing key")
                || error.contains("expected value")
        }
        
        /// Handles message retrieval when the standard approach fails
        async fn handle_retrieval_error(&self, message_id: &str, original_error: GmailServiceError) -> Result<Message> {
            // Import the MessageExt trait
            use crate::gmail_custom::MessageExt;
            
            let error_str = original_error.to_string();
            
            // Check if this is a deserialization error related to missing fields
            if self.is_missing_field_error(&error_str) {
                debug!("Detected missing field error: {}", error_str);
                info!(
                    "Using custom message retrieval for message {} due to missing field",
                    message_id
                );

                // Try with custom retrieval that handles missing fields
                match self.try_direct_message_retrieval(message_id).await {
                    Ok(msg) => {
                        debug!("Successfully retrieved message using custom approach");
                        Ok(msg.ensure_required_fields())
                    }
                    Err(fallback_err) => {
                        error!("Custom message retrieval failed: {}", fallback_err);

                        // Last resort - try with minimal format and patch the message
                        self.try_minimal_format(message_id).await.map(|msg| msg.ensure_required_fields())
                    }
                }
            } else {
                // This is not a missing field error, might be another API issue
                error!("Non-deserialization API error: {}", error_str);

                // Still try fallback as a last resort
                match self.try_direct_message_retrieval(message_id).await {
                    Ok(msg) => {
                        debug!("Successfully retrieved message using fallback approach");
                        Ok(msg.ensure_required_fields())
                    }
                    Err(fallback_err) => {
                        self.handle_all_retrieval_methods_failed(message_id, original_error, fallback_err)
                    }
                }
            }
        }
        
        /// Create a minimal valid Message with defaults for all required fields
        fn create_minimal_message(&self, message_id: &str) -> Message {
            use gmail::model::{Message, MessagePart, MessagePartBody};
            use crate::gmail_custom::defaults;
            
            debug!("Creating minimal message structure with default values for ID: {}", message_id);
            
            // Create a minimal valid Message with defaults for all required fields
            Message {
                id: message_id.to_string(),
                thread_id: message_id.to_string(), // Default to using message_id
                label_ids: Vec::new(),
                snippet: String::new(),
                history_id: "0".to_string(),
                internal_date: defaults::INTERNAL_DATE.to_string(),
                payload: MessagePart {
                    part_id: String::new(),
                    mime_type: defaults::MIME_TYPE.to_string(),
                    filename: String::new(),
                    headers: Vec::new(),
                    body: MessagePartBody::Empty { size: 0 },
                    parts: Vec::new(),
                },
                size_estimate: 0,
                raw: None,
            }
        }
        
        /// Creates a detailed error when all retrieval methods have failed
        fn handle_all_retrieval_methods_failed(
            &self, 
            message_id: &str, 
            original_error: GmailServiceError, 
            fallback_error: GmailServiceError
        ) -> Result<Message> {
            error!(
                "All message retrieval approaches failed: {} and {}",
                original_error, fallback_error
            );

            // Provide a more detailed error message for debugging
            let detailed_error = format!(
                "Gmail API message format error: Unable to retrieve message with ID {}. \
                The API response is missing required fields and all recovery attempts failed. \
                Original error: {}. Fallback error: {}", 
                message_id, original_error, fallback_error
            );

            // Log the detailed error
            error!("{}", detailed_error);
            
            // Last resort - create a minimal message with default values
            if original_error.to_string().contains("internalDate") || 
               fallback_error.to_string().contains("internalDate") {
                info!("Using last resort minimal message creation for message with ID: {}", message_id);
                return Ok(self.create_minimal_message(message_id));
            }

            // Return a more user-friendly error
            Err(GmailServiceError::ApiError(
                "The Gmail API returned a message with missing required fields. \
                This might be due to an issue with the specific message format or \
                API limitations. Please try a different message ID.".to_string()
            ))
        }
        
        /// Main method to get a message with fallback strategies
        pub async fn get_message(&self, message_id: &str) -> Result<Message> {
            debug!("Getting message with ID: {}", message_id);

            // Log request details
            let request_details = format!(
                "Request details: User ID: 'me', Message ID: '{}', Format: 'full'",
                message_id
            );
            log::info!("{}", request_details);

            // Try standard format first
            match self.get_message_standard_format(message_id).await {
                Ok(message) => Ok(message),
                Err(e) => self.handle_retrieval_error(message_id, e).await
            }
        }
        
        /// Get a message by ID and return as raw JSON instead of a Message struct
        pub async fn get_message_raw(&self, message_id: &str) -> Result<String> {
            debug!("Getting raw message with ID: {}", message_id);
            
            // Log request details
            let request_details = format!(
                "Request details: User ID: 'me', Message ID: '{}', Format: 'full'",
                message_id
            );
            log::info!("{}", request_details);
            
            // Execute the request
            let mut request = self.client.messages_get(message_id, "me");
            request = request.format("full");
            
            // Get the raw response
            let response = request.await.map_err(|e| {
                error!("Failed to get message: {}", e);
                GmailServiceError::ApiError(e.to_string())
            })?;
            
            // Convert to JSON
            match serde_json::to_string_pretty(&response) {
                Ok(json) => Ok(json),
                Err(e) => Err(GmailServiceError::ApiError(format!(
                    "JSON serialization error: {}",
                    e
                )))
            }
        }

        /// Return the raw JSON response from Gmail API without any transformation or modification
        pub async fn list_messages_raw(&self, max_results: u32, query: Option<&str>) -> Result<String> {
            debug!("Listing raw messages with max_results={}, query={:?}", max_results, query);
            
            // Set up the request
            let mut request = self.client.messages_list("me");
            request = request.max_results(max_results.into());
            
            if let Some(q) = query {
                debug!("Using query: {}", q);
                request = request.q(q);
            }
            
            // Execute the request
            debug!("Executing messages.list request");
            let response = request.await.map_err(|e| {
                error!("Failed to list messages: {}", e);
                GmailServiceError::ApiError(e.to_string())
            })?;
            
            // Convert directly to JSON string without any processing or transformation
            match serde_json::to_string_pretty(&response) {
                Ok(json) => Ok(json),
                Err(e) => Err(GmailServiceError::ApiError(format!(
                    "JSON serialization error: {}",
                    e
                )))
            }
        }

        pub async fn list_messages(
            &self,
            max_results: u32,
            query: Option<&str>,
        ) -> Result<Vec<Message>> {
            debug!(
                "Listing messages with max_results={}, query={:?}",
                max_results, query
            );

            // Set up the request
            let mut request = self.client.messages_list("me");
            request = request.max_results(max_results.into());

            if let Some(q) = query {
                debug!("Using query: {}", q);
                request = request.q(q);
            }

            // Execute the request
            debug!("Executing messages.list request");
            let response = request.await.map_err(|e| {
                error!("Failed to list messages: {}", e);
                GmailServiceError::ApiError(e.to_string())
            })?;
            
            // Add debug logging to see raw response
            if let Ok(json_str) = serde_json::to_string_pretty(&response) {
                let preview = if json_str.len() > 500 {
                    format!("{}{}", json_str.chars().take(500).collect::<String>(), "...")
                } else {
                    json_str.clone()
                };
                debug!("Raw response from Gmail API: {}", preview);
                
                // Explicitly check for internalDate field in the response
                if !json_str.contains("internalDate") {
                    debug!("internalDate field not found in the API response - will need patching");
                }
            }

            // Check if we have messages
            if let Some(message_refs) = response.messages {
                let count = message_refs.len();
                info!("Found {} message references", count);

                if count == 0 {
                    return Ok(Vec::new());
                }

                // Fetch each message
                let mut messages = Vec::with_capacity(count);
                for (idx, msg_ref) in message_refs.iter().enumerate() {
                    info!("Fetching message {}/{}: ID {}", idx + 1, count, msg_ref.id);
                    // Import the MessageExt trait
                    use crate::gmail_custom::MessageExt;

                    match self.get_message(&msg_ref.id).await {
                        Ok(message) => {
                            debug!("Successfully fetched message {}", msg_ref.id);
                            // Ensure required fields are set before adding to list
                            messages.push(message.ensure_required_fields());
                        }
                        Err(e) => {
                            error!("Failed to get message {}: {}", msg_ref.id, e);

                            // Check if this is a missing field error
                            let error_str = e.to_string();
                            let is_missing_field = error_str.contains("missing field")
                                || error_str.contains("internalDate")
                                || error_str.contains("unknown field")
                                || error_str.contains("missing key");

                            if is_missing_field {
                                // Try direct raw message retrieval with custom deserializer as last resort
                                debug!(
                                    "Detected missing field, using custom retrieval for {}",
                                    msg_ref.id
                                );
                                match self.try_direct_message_retrieval(&msg_ref.id).await {
                                    Ok(message) => {
                                        debug!("Successfully retrieved message {} using custom deserializer", msg_ref.id);
                                        // Ensure required fields are set
                                        messages.push(message.ensure_required_fields());
                                    }
                                    Err(fallback_err) => {
                                        error!("All retrieval methods failed for message {}: {} and {}", 
                                            msg_ref.id, e, fallback_err);

                                        // If the error specifically mentions internalDate, use our last resort placeholder
                                        if e.to_string().contains("internalDate") || 
                                           fallback_err.to_string().contains("internalDate") {
                                            debug!("Using minimal message placeholder for {} due to internalDate error", msg_ref.id);
                                            messages.push(self.create_minimal_message(&msg_ref.id).ensure_required_fields());
                                        } else {
                                            // Skip this message for non-internalDate related errors
                                            debug!("Skipping message {} due to retrieval errors", msg_ref.id);
                                        }
                                    }
                                }
                            } else {
                                error!("Non-format error retrieving message {}: {}", msg_ref.id, e);
                                // Continue with other messages instead of failing completely
                            }
                        }
                    }
                }

                info!("Successfully fetched {}/{} messages", messages.len(), count);
                Ok(messages)
            } else {
                debug!("No messages found in API response");
                Ok(Vec::new())
            }
        }

        /// Apply JSON patching functionality for messages with missing fields
        /// This approach handles various field requirements for the Message struct
        async fn patch_and_deserialize_message(&self, message_id: &str, format: &str) -> Result<Message> {
            debug!(
                "Attempting to retrieve message {} with format '{}' and apply JSON patches",
                message_id, format
            );
            
            // This is a cleaner approach to handle API response issues
            // First get the raw response from the API (should be JSON)
            let mut request = self.client.messages_get(message_id, "me");
            
            // Set the format
            request = request.format(format);
            
            // Add headers for metadata format
            if format == "metadata" {
                request = request.metadata_headers(&["Subject", "From", "To", "Date"]);
            }
            
            // Execute the request
            let response = request.await.map_err(|e| {
                error!("Failed to get message with format {}: {}", format, e);
                GmailServiceError::ApiError(e.to_string())
            })?;
            
            // Process using our custom deserializer functions in the gmail_custom module
            // to add any missing fields
            use crate::gmail_custom::MessageExt;
            Ok(response.ensure_required_fields())
        }
        
        /// Try to retrieve a message with minimal format, which is less likely 
        /// to have complex field structure that could cause parsing errors
        async fn try_minimal_format(&self, message_id: &str) -> Result<Message> {
            debug!("Falling back to minimal format");
            
            let minimal_msg = self.patch_and_deserialize_message(message_id, "minimal").await?;
            debug!("Successfully retrieved message with minimal format");
            
            // If minimal format has empty headers, try to get headers via metadata request
            if minimal_msg.payload.headers.is_empty() {
                debug!("No headers in minimal format, attempting to get metadata");
                match self.patch_and_deserialize_message(message_id, "metadata").await {
                    Ok(metadata_msg) => {
                        debug!(
                            "Successfully retrieved metadata with {} headers",
                            metadata_msg.payload.headers.len()
                        );
                        
                        // Create a new message with headers from metadata and other fields from minimal
                        let mut enhanced_msg = minimal_msg.clone();
                        enhanced_msg.payload.headers = metadata_msg.payload.headers;
                        Ok(enhanced_msg)
                    }
                    Err(e) => {
                        debug!("Failed to get metadata, but continuing with minimal format: {}", e);
                        Ok(minimal_msg)
                    }
                }
            } else {
                Ok(minimal_msg)
            }
        }
        
        /// Attempt direct message retrieval as a fallback
        /// This is primarily a wrapper for try_minimal_format for clarity in the calling code
        async fn try_direct_message_retrieval(&self, message_id: &str) -> Result<Message> {
            debug!(
                "Attempting direct message retrieval with fallback options for ID: {}",
                message_id
            );
            
            // Direct retrieval now uses the cleaner try_minimal_format approach
            self.try_minimal_format(message_id).await
        }

        pub async fn list_labels(&self) -> Result<String> {
            debug!("Listing labels");
            let response = self
                .client
                .labels_list("me")
                .await
                .map_err(|e| GmailServiceError::ApiError(e.to_string()))?;
            
            // Convert response to raw JSON string
            match serde_json::to_string_pretty(&response) {
                Ok(json) => Ok(json),
                Err(e) => Err(GmailServiceError::ApiError(format!(
                    "JSON serialization error: {}",
                    e
                )))
            }
        }

        pub async fn check_connection_raw(&self) -> Result<String> {
            debug!("Checking connection raw");
            let profile = self
                .client
                .get_profile("me")
                .await
                .map_err(|e| GmailServiceError::ApiError(e.to_string()))?;
            
            // Convert response to raw JSON string
            match serde_json::to_string_pretty(&profile) {
                Ok(json) => Ok(json),
                Err(e) => Err(GmailServiceError::ApiError(format!(
                    "JSON serialization error: {}",
                    e
                )))
            }
        }
        
        pub async fn check_connection(&self) -> Result<(String, u64)> {
            debug!("Checking connection");
            let profile = self
                .client
                .get_profile("me")
                .await
                .map_err(|e| GmailServiceError::ApiError(e.to_string()))?;

            let email = profile
                .email_address
                .unwrap_or_else(|| "Unknown".to_string());
            let messages_total = profile.messages_total.unwrap_or(0) as u64;

            Ok((email, messages_total))
        }
    }

    fn create_client(config: &Config) -> Result<GmailClient> {
        debug!("Creating Gmail client with OAuth credentials");

        // Create auth
        let auth = gmail::GmailAuth::oauth2(
            config.access_token.clone().unwrap_or_default(),
            config.refresh_token.clone(),
            None,
        );

        // Create and return client
        let client = GmailClient::with_auth(auth);
        debug!("Gmail client created successfully");

        Ok(client)
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

// Custom Gmail message handling module to handle API response issues
pub mod gmail_custom {
    use gmail::model::Message;
    use log::{debug, error};
    
    // Constants for default values used throughout the module
    pub mod defaults {
        /// Default value for missing or invalid internalDate fields
        /// Set to "0" to represent the Unix epoch (January 1, 1970)
        pub const INTERNAL_DATE: &str = "0";
        
        /// Default value for missing snippet fields
        pub const SNIPPET: &str = "";
        
        /// Default MIME type when not provided
        pub const MIME_TYPE: &str = "text/plain";
        
        /// Default headers capacity for new header collections
        pub const HEADERS_CAPACITY: usize = 4;
    }
    use serde_json::Value;

    // JSON patching module - pure functions for fixing missing fields
    mod json_patch {
        use super::defaults;
        use log::debug;
        use serde_json::{Map, Value};
        
        /// Patch a missing or null internalDate field
        pub fn ensure_internal_date(json_obj: &mut Map<String, Value>, message_id: &str) -> bool {
            let mut modified = false;
            
            // Check at root level
            if !json_obj.contains_key("internalDate") || json_obj["internalDate"].is_null() {
                debug!("Adding internalDate at root level for message {}", message_id);
                json_obj.insert(
                    "internalDate".to_string(), 
                    Value::String(defaults::INTERNAL_DATE.to_string())
                );
                modified = true;
            }
            
            // Also log the structure to diagnose
            if modified {
                debug!("JSON structure after patching: {}", 
                       serde_json::to_string_pretty(json_obj).unwrap_or_default().chars().take(500).collect::<String>());
            }
            
            modified
        }
        
        /// Patch a missing or null labelIds field
        pub fn ensure_label_ids(json_obj: &mut Map<String, Value>, _message_id: &str) -> bool {
            if !json_obj.contains_key("labelIds") || json_obj["labelIds"].is_null() {
                json_obj.insert("labelIds".to_string(), Value::Array(vec![]));
                true
            } else {
                false
            }
        }
        
        /// Patch a missing or null snippet field
        pub fn ensure_snippet(json_obj: &mut Map<String, Value>, _message_id: &str) -> bool {
            if !json_obj.contains_key("snippet") || json_obj["snippet"].is_null() {
                json_obj.insert(
                    "snippet".to_string(),
                    Value::String(defaults::SNIPPET.to_string())
                );
                true
            } else {
                false
            }
        }
        
        /// Patch a missing or null threadId field, using message_id as fallback
        pub fn ensure_thread_id(json_obj: &mut Map<String, Value>, message_id: &str) -> bool {
            if !json_obj.contains_key("threadId") || json_obj["threadId"].is_null() {
                json_obj.insert("threadId".to_string(), Value::String(message_id.to_string()));
                true
            } else {
                false
            }
        }
        
        /// Patch a missing or null payload field with default structure
        pub fn ensure_payload(json_obj: &mut Map<String, Value>, _message_id: &str) -> bool {
            if !json_obj.contains_key("payload") || json_obj["payload"].is_null() {
                let mut payload = Map::new();
                
                // Add headers with empty array
                payload.insert("headers".to_string(), Value::Array(vec![]));
                
                // Add mimeType (required)
                payload.insert(
                    "mimeType".to_string(),
                    Value::String(defaults::MIME_TYPE.to_string())
                );
                
                // Add payload to the message
                json_obj.insert("payload".to_string(), Value::Object(payload));
                true
            } else if let Value::Object(ref mut payload) = json_obj["payload"] {
                let mut modified = false;
                
                // Ensure headers exist in payload
                if !payload.contains_key("headers") || payload["headers"].is_null() {
                    payload.insert("headers".to_string(), Value::Array(vec![]));
                    modified = true;
                }
                
                // Ensure mimeType exists
                if !payload.contains_key("mimeType") || payload["mimeType"].is_null() {
                    payload.insert(
                        "mimeType".to_string(),
                        Value::String(defaults::MIME_TYPE.to_string())
                    );
                    modified = true;
                }
                
                modified
            } else {
                false
            }
        }
        
        /// Apply all patches to ensure a valid Gmail message JSON structure
        pub fn patch_gmail_message(json_value: &mut Value) -> (bool, String) {
            let mut modified = false;
            let message_id;
            
            // Handle the JSON object
            if let Value::Object(ref mut map) = json_value {
                // Extract the message ID for better logging and diagnostic
                message_id = map
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                // Apply all patches
                modified |= ensure_internal_date(map, &message_id);
                modified |= ensure_label_ids(map, &message_id);
                modified |= ensure_snippet(map, &message_id);
                modified |= ensure_thread_id(map, &message_id);
                modified |= ensure_payload(map, &message_id);
            } else {
                message_id = "unknown".to_string();
            }
            
            (modified, message_id)
        }
    }

    /// Custom message deserializer to handle missing fields in Gmail API responses
    /// This function attempts to deserialize a JSON response into a Message struct,
    /// filling in missing fields with default values
    pub fn deserialize_custom_message(json_str: &String) -> Result<Message, serde_json::Error> {
        use log::{debug, error, info};
        
        debug!("Deserializing custom message from JSON");
        
        // Log a preview of the JSON string to help with debugging
        if json_str.len() > 500 {
            debug!("JSON preview (first 500 chars): {}", json_str.chars().take(500).collect::<String>());
        } else {
            debug!("JSON string: {}", json_str);
        }

        // First, parse the JSON into a generic Value
        let mut json_value: Value = serde_json::from_str(json_str)?;

        debug!("Parsed JSON into Value object, checking for missing fields");
        
        // Log message ID if present
        if let Some(id) = json_value.get("id").and_then(|id| id.as_str()) {
            info!("Processing message with ID: {}", id);
        }
        
        // Check for internalDate field specifically before patching
        if !json_value.get("internalDate").is_some() {
            debug!("internalDate field is missing from the original JSON");
        }

        // Apply all the patches to ensure required fields exist
        let (modified, message_id) = json_patch::patch_gmail_message(&mut json_value);
        
        if modified {
            debug!("Applied JSON patches to message ID: {}", message_id);
        }

        // Now try to deserialize the patched JSON
        match serde_json::from_value::<Message>(json_value.clone()) {
            Ok(message) => {
                debug!("Successfully deserialized message with ID: {}", message.id);
                Ok(message)
            }
            Err(e) => {
                error!("Failed to deserialize message even after patching: {}", e);
                // For debugging, print the JSON structure after our patches
                debug!(
                    "Patched JSON structure: {}",
                    serde_json::to_string_pretty(&json_value).unwrap_or_default()
                );

                // We can't manually construct a Message
                error!("Failed to parse message due to: {}", e);
                Err(e)
            }
        }
    }

    /// Extend gmail-rs Message for our needs
    pub trait MessageExt {
        /// Add default values to any missing fields in a Message
        fn ensure_required_fields(self) -> Self;
    }

    impl MessageExt for Message {
        fn ensure_required_fields(mut self) -> Self {
            // Ensure internalDate is not empty
            if self.internal_date.is_empty() {
                debug!("Adding default internal_date for message {}", self.id);
                self.internal_date = defaults::INTERNAL_DATE.to_string();
            }

            // Add more field validations as needed

            self
        }
    }
}

// Module with the server implementation
pub mod server {
    use log::{debug, error, info};
    use mcp_attr::jsoncall::ErrorCode;
    use mcp_attr::server::{mcp_server, McpServer};
    use mcp_attr::{Error as McpError, Result as McpResult};

    use crate::config::{Config, ConfigError};
    use crate::gmail_service::{GmailService, GmailServiceError};
    
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
                                    debug!("Could not parse string '{}' as u32, using default {}", s, default);
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

    // MCP server for accessing Gmail API
    #[derive(Clone)]
    pub struct GmailServer;

    // Enum of error codes used by the Gmail MCP server
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
    }

    impl GmailServer {
        pub fn new() -> Self {
            GmailServer {}
        }

        // Helper function to create McpError with appropriate error code
        fn to_mcp_error(&self, message: &str, code: u32) -> McpError {
            error!("Creating MCP error: {} (code: {})", message, code);
            McpError::new(ErrorCode(code as i64))
        }

        // Helper function to map GmailServiceError to McpError with specific codes
        fn map_gmail_error(&self, err: GmailServiceError) -> McpError {
            match err {
                GmailServiceError::ApiError(e) => {
                    let msg = format!("Gmail API error: {}", e);
                    
                    // Determine more specific error type from the message content
                    let code = if e.contains("authentication") || e.contains("auth") || e.contains("token") {
                        error_codes::AUTH_ERROR
                    } else if e.contains("format") || e.contains("missing field") || e.contains("parse") {
                        error_codes::MESSAGE_FORMAT_ERROR
                    } else {
                        error_codes::API_ERROR
                    };
                    
                    self.to_mcp_error(&msg, code)
                },
                GmailServiceError::AuthError(e) => {
                    let msg = format!("Gmail authentication error: {}", e);
                    self.to_mcp_error(&msg, error_codes::AUTH_ERROR)
                },
            }
        }

        // Helper function to initialize Gmail service
        async fn init_gmail_service(&self) -> McpResult<GmailService> {
            // Load configuration
            let config = Config::from_env().map_err(|err| {
                let msg = match err {
                    ConfigError::MissingEnvVar(var) => {
                        format!("Missing environment variable: {}", var)
                    }
                    ConfigError::EnvError(e) => format!("Environment variable error: {}", e),
                };
                self.to_mcp_error(&msg, error_codes::CONFIG_ERROR)
            })?;

            // Create Gmail service
            GmailService::new(&config).map_err(|err| self.map_gmail_error(err))
        }
    }

    // MCP server implementation with custom serialization
    #[mcp_server]
    impl McpServer for GmailServer {
        /// Gmail MCP Server
        ///
        /// This MCP server provides access to Gmail using the gmail-rs crate.
        /// It requires the following environment variables to be set:
        /// - GMAIL_CLIENT_ID
        /// - GMAIL_CLIENT_SECRET
        /// - GMAIL_REFRESH_TOKEN
        ///
        /// You can provide these in a .env file in the same directory as the executable.
        #[prompt]
        async fn gmail_prompt(&self) -> McpResult<&str> {
            Ok("Gmail MCP Server")
        }

        /// Get a list of emails from the inbox
        ///
        /// Returns the raw JSON response from the Gmail API without any transformation.
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
            let service = self.init_gmail_service().await?;

            // Get raw message list JSON directly from the API without transformation
            let messages_json = service
                .list_messages_raw(max, query.as_deref())
                .await
                .map_err(|err| self.map_gmail_error(err))?;

            info!("=== END list_emails MCP command (success) ===");
            Ok(messages_json)
        }

        /// Get details for a specific email
        ///
        /// Args:
        ///   message_id: The ID of the message to retrieve
        #[tool]
        async fn get_email(&self, message_id: String) -> McpResult<String> {
            info!("=== START get_email MCP command ===");
            debug!("get_email called with message_id={}", message_id);

            // Get the Gmail service
            let service = self.init_gmail_service().await?;

            // Get message as raw JSON
            let message_json = service
                .get_message_raw(&message_id)
                .await
                .map_err(|err| self.map_gmail_error(err))?;

            info!("=== END get_email MCP command (success) ===");
            Ok(message_json)
        }

        /// Search for emails using a Gmail search query
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
            let service = self.init_gmail_service().await?;
            
            // Get raw message list JSON
            let messages_json = service
                .list_messages_raw(max, Some(&query))
                .await
                .map_err(|err| self.map_gmail_error(err))?;
                
            info!("=== END search_emails MCP command (success) ===");
            Ok(messages_json)
        }

        /// Get a list of email labels
        ///
        /// Returns a list of all labels in the user's mailbox
        #[tool]
        async fn list_labels(&self) -> McpResult<String> {
            debug!("list_labels called");

            // Get the Gmail service
            let service = self.init_gmail_service().await?;

            // Get labels
            service
                .list_labels()
                .await
                .map_err(|err| self.map_gmail_error(err))
        }

        /// Check connection status with Gmail API
        ///
        /// Tests the connection to Gmail API by retrieving the user's profile
        #[tool]
        async fn check_connection(&self) -> McpResult<String> {
            info!("=== START check_connection MCP command ===");
            debug!("check_connection called");

            // Get the Gmail service
            let service = self.init_gmail_service().await?;

            // Get profile as raw JSON
            let profile_json = service
                .check_connection_raw()
                .await
                .map_err(|err| self.map_gmail_error(err))?;

            info!("=== END check_connection MCP command (success) ===");
            Ok(profile_json)
        }
    }
}
