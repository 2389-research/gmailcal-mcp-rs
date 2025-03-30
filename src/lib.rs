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
pub use crate::config::Config;
pub use crate::gmail_service::EmailMessage;
pub use crate::logging::setup_logging;
pub use crate::gmail_custom::deserialize_custom_message;

// Module for centralized configuration
pub mod config {
    use std::env;
    use thiserror::Error;
    use dotenv::dotenv;
    use log::debug;

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
    use gmail::GmailClient;
    use gmail::model::Message;
    use log::{debug, error, info, warn};
    use thiserror::Error;
    use serde::{Serialize, Deserialize};
    use crate::config::Config;
    
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
            debug!("Converting Gmail Message to EmailMessage for ID: {}", message.id);
            
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
                    },
                    "From" => {
                        debug!("Found From header: {}", header.value);
                        from = Some(header.value.clone());
                    },
                    "To" => {
                        debug!("Found To header: {}", header.value);
                        to = Some(header.value.clone());
                    },
                    "Date" => {
                        debug!("Found Date header: {}", header.value);
                        date = Some(header.value.clone());
                    },
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
        
        pub async fn get_message(&self, message_id: &str) -> Result<Message> {
            debug!("Getting message with ID: {}", message_id);
            
            // Look at the details of the API request
            let request_details = format!(
                "Request details: User ID: 'me', Message ID: '{}', Format: 'full'",
                message_id
            );
            log::info!("{}", request_details);
            
            // Import the MessageExt trait
            use crate::gmail_custom::MessageExt;
            
            // Try with "full" format first
            let result = self.client.messages_get(message_id, "me").format("full").await;
            
            match result {
                Ok(message) => {
                    debug!("Successfully retrieved message with standard approach");
                    debug!("Message ID: {}, Thread ID: {}", message.id, message.thread_id);
                    debug!("Has internalDate: {}", !message.internal_date.is_empty());
                    debug!("Labels count: {}", message.label_ids.len());
                    debug!("Headers count: {}", message.payload.headers.len());
                    
                    // Ensure all required fields have values
                    let message = message.ensure_required_fields();
                    Ok(message)
                },
                Err(e) => {
                    // Check if this is a deserialization error related to missing fields
                    let error_str = e.to_string();
                    let is_missing_field = error_str.contains("missing field") || 
                                         error_str.contains("unknown field") ||
                                         error_str.contains("missing key") ||
                                         error_str.contains("expected value");
                    
                    if is_missing_field {
                        // Log detailed info for debugging
                        debug!("Detected missing field error: {}", error_str);
                        info!("Using custom message retrieval for message {} due to missing field", message_id);
                        
                        // Try with our custom retrieval that handles missing fields
                        match self.try_direct_message_retrieval(message_id).await {
                            Ok(msg) => {
                                debug!("Successfully retrieved message using custom approach");
                                Ok(msg.ensure_required_fields())
                            },
                            Err(fallback_err) => {
                                error!("Custom message retrieval failed: {}", fallback_err);
                                
                                // Last resort - try with minimal format and patch the message
                                match self.try_minimal_format(message_id).await {
                                    Ok(msg) => {
                                        debug!("Successfully retrieved message using minimal format");
                                        Ok(msg.ensure_required_fields())
                                    },
                                    Err(e) => {
                                        error!("All retrieval methods failed");
                                        Err(e)
                                    }
                                }
                            }
                        }
                    } else {
                        // This is not a missing field error, might be another API issue
                        error!("Non-deserialization API error: {}", e);
                        
                        // Still try fallback as a last resort
                        match self.try_direct_message_retrieval(message_id).await {
                            Ok(msg) => {
                                debug!("Successfully retrieved message using fallback approach");
                                Ok(msg.ensure_required_fields())
                            },
                            Err(fallback_err) => {
                                error!("All message retrieval approaches failed: {} and {}", e, fallback_err);
                                
                                // Provide a more detailed error message for debugging
                                let detailed_error = format!(
                                    "Gmail API message format error: Unable to retrieve message with ID {}. \
                                    The API response is missing required fields and all recovery attempts failed. \
                                    Original error: {}. Fallback error: {}", 
                                    message_id, e, fallback_err
                                );
                                
                                // Log the detailed error
                                error!("{}", detailed_error);
                                
                                // Return a more user-friendly error
                                Err(GmailServiceError::ApiError(
                                    "The Gmail API returned a message with missing required fields. \
                                    This might be due to an issue with the specific message format or \
                                    API limitations. Please try a different message ID.".to_string()
                                ))
                            }
                        }
                    }
                }
            }
        }
        
        pub async fn list_messages(&self, max_results: u32, query: Option<&str>) -> Result<Vec<Message>> {
            debug!("Listing messages with max_results={}, query={:?}", max_results, query);
            
            // Set up the request
            let mut request = self.client.messages_list("me");
            request = request.max_results(max_results.into());
            
            if let Some(q) = query {
                debug!("Using query: {}", q);
                request = request.q(q);
            }
            
            // Execute the request
            debug!("Executing messages.list request");
            let response = request.await
                .map_err(|e| {
                    error!("Failed to list messages: {}", e);
                    GmailServiceError::ApiError(e.to_string())
                })?;
                
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
                        },
                        Err(e) => {
                            error!("Failed to get message {}: {}", msg_ref.id, e);
                            
                            // Check if this is a missing field error
                            let error_str = e.to_string();
                            let is_missing_field = error_str.contains("missing field") || 
                                                error_str.contains("internalDate") ||
                                                error_str.contains("unknown field") ||
                                                error_str.contains("missing key");
                            
                            if is_missing_field {
                                // Try direct raw message retrieval with custom deserializer as last resort
                                debug!("Detected missing field, using custom retrieval for {}", msg_ref.id);
                                match self.try_direct_message_retrieval(&msg_ref.id).await {
                                    Ok(message) => {
                                        debug!("Successfully retrieved message {} using custom deserializer", msg_ref.id);
                                        // Ensure required fields are set
                                        messages.push(message.ensure_required_fields());
                                    },
                                    Err(fallback_err) => {
                                        error!("All retrieval methods failed for message {}: {} and {}", 
                                            msg_ref.id, e, fallback_err);
                                        
                                        // Skip this message rather than trying to construct a placeholder
                                        // The gmail-rs Message struct has fields we can't reconstruct easily
                                        debug!("Skipping message {} due to retrieval errors", msg_ref.id);
                                    }
                                }
                            } else {
                                error!("Non-format error retrieving message {}: {}", msg_ref.id, e);
                                // Continue with other messages instead of failing completely
                            }
                        },
                    }
                }
                
                info!("Successfully fetched {}/{} messages", messages.len(), count);
                Ok(messages)
            } else {
                debug!("No messages found in API response");
                Ok(Vec::new())
            }
        }
        
        // Last resort method that directly retrieves message with fallback options
        async fn try_direct_message_retrieval(&self, message_id: &str) -> Result<Message> {
            debug!("Attempting direct message retrieval with fallback options for ID: {}", message_id);
            
            // Since we can't directly access the raw JSON, try using minimal format
            // with minimal fields, which is less likely to have missing field issues
            self.try_minimal_format(message_id).await
        }
        
        // Helper method to try minimal format as absolute last resort
        async fn try_minimal_format(&self, message_id: &str) -> Result<Message> {
            debug!("Falling back to minimal format");
            let minimal_result = self.client.messages_get(message_id, "me").format("minimal").await;
            
            if let Ok(mut minimal_msg) = minimal_result {
                debug!("Successfully retrieved message with minimal format");
                
                // Ensure internalDate exists (this was the problematic field)
                if minimal_msg.internal_date.is_empty() {
                    debug!("internalDate is empty, setting default value");
                    minimal_msg.internal_date = "0".to_string();
                }
                
                // Try to get metadata for headers 
                debug!("Attempting to add headers via metadata request");
                match self.client.messages_get(message_id, "me")
                    .format("metadata")
                    .metadata_headers(&["Subject", "From", "To", "Date"])
                    .await 
                {
                    Ok(metadata_msg) => {
                        debug!("Successfully retrieved metadata with {} headers", 
                               metadata_msg.payload.headers.len());
                        
                        // Update headers from metadata
                        minimal_msg.payload.headers = metadata_msg.payload.headers;
                    },
                    Err(e) => {
                        debug!("Failed to get metadata headers: {}", e);
                    }
                }
                
                Ok(minimal_msg)
            } else {
                // If minimal format fails, return the error
                error!("All retrieval approaches failed for message {}", message_id);
                Err(GmailServiceError::ApiError(format!(
                    "Could not retrieve message {} with any format", 
                    message_id
                )))
            }
        }
        
        pub async fn list_labels(&self) -> Result<String> {
            debug!("Listing labels");
            let response = self.client.labels_list("me")
                .await
                .map_err(|e| GmailServiceError::ApiError(e.to_string()))?;
                
            if let Some(labels) = response.labels {
                match serde_json::to_string_pretty(&labels) {
                    Ok(json) => Ok(json),
                    Err(e) => Err(GmailServiceError::ApiError(format!("JSON serialization error: {}", e))),
                }
            } else {
                Ok("[]".to_string())
            }
        }
        
        pub async fn check_connection(&self) -> Result<(String, u64)> {
            debug!("Checking connection");
            let profile = self.client.get_profile("me")
                .await
                .map_err(|e| GmailServiceError::ApiError(e.to_string()))?;
                
            let email = profile.email_address.unwrap_or_else(|| "Unknown".to_string());
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
    use simplelog::*;
    use std::fs::OpenOptions;
    use std::io::Write;
    use chrono::Local;
    use log::LevelFilter;

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
    pub fn setup_logging(log_level: LevelFilter, log_file: Option<&str>) -> std::io::Result<String> {
        // Create a timestamp for the log file
        let timestamp = Local::now().format("%Y%m%d_%H").to_string();
        
        // Determine log file path
        let log_path = match log_file {
            Some(path) => path.to_string(),
            None => format!("gmail_mcp_{}.log", timestamp),
        };
        
        // Create the log file with append mode
        let log_file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&log_path)?;
            
        // Add a header to the log file
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(&log_path)?;
            
        writeln!(file, "====== GMAIL MCP SERVER LOG - Started at {} ======", 
            Local::now().format("%Y-%m-%d %H:%M:%S"))?;
        
        // Use the default config for simplicity
        let config = Config::default();
        
        // During development, consider uncommenting the second logger to see logs on console too
        CombinedLogger::init(vec![
            // File logger
            WriteLogger::new(log_level, config, log_file),
            
            // Uncomment for console logging during development
            // TermLogger::new(
            //     log_level,
            //     Config::default(),
            //     TerminalMode::Mixed,
            //     ColorChoice::Auto
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
    use log::{debug, warn, error};
    use gmail::model::{Message, MessagePart, Header};
    use serde_json::Value;
    
    // Simpler approach - just directly handle the issue in the deserializer
    // Since we can't reliably access the internal gmail crate functionality
    
    /// Custom message deserializer to handle missing fields in Gmail API responses
    /// This function attempts to deserialize a JSON response into a Message struct,
    /// filling in missing fields with default values
    pub fn deserialize_custom_message(json_str: &String) -> Result<Message, serde_json::Error> {
        debug!("Deserializing custom message from JSON");
        
        // First, parse the JSON into a generic Value
        let mut json_value: Value = serde_json::from_str(json_str)?;
        
        debug!("Parsed JSON into Value object, checking for missing fields");
        
        // Check if it's an object
        if let Value::Object(ref mut map) = json_value {
            // Extract the message ID for better logging and clone it to avoid borrow issues
            let message_id = map.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();  // Clone to avoid borrowing issues
                
            debug!("Processing message ID: {}", message_id);
            
            // Handle internalDate (required as String in gmail-rs)
            if !map.contains_key("internalDate") {
                warn!("Message {} is missing internalDate field, adding default value", message_id);
                map.insert("internalDate".to_string(), Value::String("0".to_string()));
            } else if let Value::Null = map["internalDate"] {
                warn!("Message {} has null internalDate field, replacing with default value", message_id);
                map.insert("internalDate".to_string(), Value::String("0".to_string()));
            }
            
            // Handle label_ids (required as Vec<String> in gmail-rs)
            if !map.contains_key("labelIds") {
                debug!("Adding missing labelIds field with empty array for message {}", message_id);
                map.insert("labelIds".to_string(), Value::Array(vec![]));
            } else if let Value::Null = map["labelIds"] {
                debug!("labelIds is null for message {}, replacing with empty array", message_id);
                map.insert("labelIds".to_string(), Value::Array(vec![]));
            }
            
            // Add other required fields with sensible defaults
            if !map.contains_key("snippet") || map["snippet"].is_null() {
                debug!("Adding missing snippet field with empty string for message {}", message_id);
                map.insert("snippet".to_string(), Value::String("".to_string()));
            }
            
            // Check threadId (required as String)
            if !map.contains_key("threadId") || map["threadId"].is_null() {
                debug!("Adding missing threadId field with message ID for message {}", message_id);
                // Use the message ID as threadId if missing
                map.insert("threadId".to_string(), Value::String(message_id.clone()));
            }
            
            // Ensure payload exists and has necessary structure
            if !map.contains_key("payload") || map["payload"].is_null() {
                debug!("Adding missing payload field with default structure for message {}", message_id);
                let mut payload = serde_json::Map::new();
                
                // Add headers with empty array
                payload.insert("headers".to_string(), Value::Array(vec![]));
                // Add mimeType (required)
                payload.insert("mimeType".to_string(), Value::String("text/plain".to_string()));
                
                // Add payload to the message
                map.insert("payload".to_string(), Value::Object(payload));
                warn!("Added default payload structure to message {}", message_id);
            } else if let Value::Object(ref mut payload) = map["payload"] {
                // Ensure headers exist in payload
                if !payload.contains_key("headers") || payload["headers"].is_null() {
                    debug!("Adding missing headers field to payload for message {}", message_id);
                    payload.insert("headers".to_string(), Value::Array(vec![]));
                }
                
                // Ensure mimeType exists
                if !payload.contains_key("mimeType") || payload["mimeType"].is_null() {
                    debug!("Adding missing mimeType field to payload for message {}", message_id);
                    payload.insert("mimeType".to_string(), Value::String("text/plain".to_string()));
                }
            }
        }
        
        // Now try to deserialize the patched JSON
        match serde_json::from_value::<Message>(json_value.clone()) {
            Ok(message) => {
                debug!("Successfully deserialized message with ID: {}", message.id);
                Ok(message)
            },
            Err(e) => {
                error!("Failed to deserialize message even after patching: {}", e);
                // For debugging, print the JSON structure after our patches
                debug!("Patched JSON structure: {}", serde_json::to_string_pretty(&json_value).unwrap_or_default());
                
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
                self.internal_date = "0".to_string();
            }
            
            // Add more field validations as needed
            
            self
        }
    }
}

// Module with the server implementation
pub mod server {    
    use mcp_attr::jsoncall::ErrorCode;
    use mcp_attr::server::{mcp_server, McpServer};
    use mcp_attr::{Error as McpError, Result as McpResult};
    use log::{info, debug, error};
    
    use crate::config::{Config, ConfigError};
    use crate::gmail_service::{GmailService, GmailServiceError, EmailMessage};
    
    // MCP server for accessing Gmail API
    #[derive(Clone)]
    pub struct GmailServer;
    
    impl GmailServer {
        pub fn new() -> Self {
            GmailServer {}
        }
        
        // Helper function to create McpError
        fn to_mcp_error(&self, message: &str) -> McpError {
            // Use a numeric error code of 1000 for application errors
            error!("Creating MCP error: {}", message);
            McpError::new(ErrorCode(1000))
        }
        
        // Helper function to map GmailServiceError to McpError
        fn map_gmail_error(&self, err: GmailServiceError) -> McpError {
            let msg = match err {
                GmailServiceError::ApiError(e) => format!("Gmail API error: {}", e),
                GmailServiceError::AuthError(e) => format!("Gmail authentication error: {}", e),
            };
            self.to_mcp_error(&msg)
        }
        
        // Helper function to initialize Gmail service
        async fn init_gmail_service(&self) -> McpResult<GmailService> {
            // Load configuration
            let config = Config::from_env().map_err(|err| {
                let msg = match err {
                    ConfigError::MissingEnvVar(var) => format!("Missing environment variable: {}", var),
                    ConfigError::EnvError(e) => format!("Environment variable error: {}", e),
                };
                self.to_mcp_error(&msg)
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
        /// Returns a JSON array of email messages from the user's inbox.
        /// 
        /// Args:
        ///   max_results: Optional maximum number of results to return (default: 10). Can be a number (3) or a string ("3").
        ///   query: Optional Gmail search query string (e.g. "is:unread from:example.com")
        #[tool]
        async fn list_emails(
            &self, 
            max_results: Option<serde_json::Value>, 
            query: Option<String>
        ) -> McpResult<String> {
            info!("=== START list_emails MCP command ===");
            debug!("list_emails called with max_results={:?}, query={:?}", max_results, query);
            
            // Convert max_results from string or number to u32
            let max = match max_results {
                Some(val) => {
                    match val {
                        serde_json::Value::Number(num) => {
                            // Handle number input
                            if let Some(n) = num.as_u64() {
                                // Ensure it fits in u32
                                if n <= u32::MAX as u64 {
                                    Some(n as u32)
                                } else {
                                    debug!("Number too large for u32, using default");
                                    None
                                }
                            } else {
                                debug!("Number not convertible to u32, using default");
                                None
                            }
                        },
                        serde_json::Value::String(s) => {
                            // Handle string input
                            match s.parse::<u32>() {
                                Ok(n) => Some(n),
                                Err(_) => {
                                    debug!("Could not parse string '{}' as u32, using default", s);
                                    None
                                }
                            }
                        },
                        _ => {
                            debug!("Unexpected value type for max_results: {:?}, using default", val);
                            None
                        }
                    }
                },
                None => None,
            }.unwrap_or(10);
            
            // Get the Gmail service
            let service = self.init_gmail_service().await?;
            
            // Get messages
            let messages = service.list_messages(max, query.as_deref()).await
                .map_err(|err| self.map_gmail_error(err))?;
            
            // Convert to EmailMessage objects
            let email_messages: Vec<EmailMessage> = messages.into_iter()
                .map(EmailMessage::from_gmail_message)
                .collect();
                
            // Return as JSON
            match serde_json::to_string_pretty(&email_messages) {
                Ok(json) => {
                    info!("=== END list_emails MCP command (success) ===");
                    Ok(json)
                },
                Err(e) => {
                    let error_msg = format!("JSON serialization error: {}", e);
                    error!("=== END list_emails MCP command (error) ===");
                    Err(self.to_mcp_error(&error_msg))
                }
            }
        }
        
        /// Get details for a specific email
        /// 
        /// Args:
        ///   message_id: The ID of the message to retrieve
        #[tool]
        async fn get_email(&self, message_id: String) -> McpResult<String> {
            debug!("get_email called with message_id={}", message_id);
            
            // Get the Gmail service
            let service = self.init_gmail_service().await?;
            
            // Get message
            let message = service.get_message(&message_id).await
                .map_err(|err| self.map_gmail_error(err))?;
            
            // Convert to EmailMessage
            let email = EmailMessage::from_gmail_message(message);
            
            // Return as JSON
            match serde_json::to_string_pretty(&email) {
                Ok(json) => Ok(json),
                Err(e) => {
                    let error_msg = format!("JSON serialization error: {}", e);
                    Err(self.to_mcp_error(&error_msg))
                }
            }
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
            max_results: Option<serde_json::Value>
        ) -> McpResult<String> {
            // Convert max_results from JSON value to Option<u32> for proper parameter passing
            let max_value = match max_results {
                Some(val) => {
                    match val {
                        serde_json::Value::Number(num) => {
                            // Handle number input
                            if let Some(n) = num.as_u64() {
                                // Ensure it fits in u32
                                if n <= u32::MAX as u64 {
                                    Some(serde_json::Value::Number(serde_json::Number::from(n)))
                                } else {
                                    debug!("Number too large for u32, using default");
                                    None
                                }
                            } else {
                                debug!("Number not convertible to u32, using default");
                                None
                            }
                        },
                        serde_json::Value::String(s) => {
                            // Handle string input
                            match s.parse::<u32>() {
                                Ok(n) => Some(serde_json::Value::Number(serde_json::Number::from(n))),
                                Err(_) => {
                                    debug!("Could not parse string '{}' as u32, using default", s);
                                    None
                                }
                            }
                        },
                        _ => {
                            debug!("Unexpected value type for max_results: {:?}, using default", val);
                            None
                        }
                    }
                },
                None => None,
            };
            
            // This is essentially the same as list_emails but with a required query parameter
            self.list_emails(max_value, Some(query)).await
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
            service.list_labels().await
                .map_err(|err| self.map_gmail_error(err))
        }
        
        /// Check connection status with Gmail API
        /// 
        /// Tests the connection to Gmail API by retrieving the user's profile
        #[tool]
        async fn check_connection(&self) -> McpResult<String> {
            debug!("check_connection called");
            
            // Get the Gmail service
            let service = self.init_gmail_service().await?;
            
            // Check connection
            let (email, messages_total) = service.check_connection().await
                .map_err(|err| self.map_gmail_error(err))?;
                
            // Format response
            Ok(format!("Connection successful!\nEmail: {}\nTotal messages: {}", email, messages_total))
        }
    }
}