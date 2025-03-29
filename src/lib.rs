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
// Re-export GmailServer for use in tests
pub use crate::server::GmailServer;
pub use crate::logging::setup_logging;

// Module for logging configuration
pub mod logging {
    use simplelog::*;
    use std::fs::{OpenOptions};
    use std::io::Write;
    use chrono::Local;
    use log::LevelFilter;

    /// Sets up logging to both console and file
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
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        
        // Determine log file path
        let log_path = match log_file {
            Some(path) => path.to_string(),
            None => format!("gmail_mcp_{}.log", timestamp),
        };
        
        println!("Setting up logging to file: {}", log_path);
        
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
            
        // Initialize the logger
        CombinedLogger::init(vec![
            TermLogger::new(
                log_level,
                config.clone(),
                TerminalMode::Mixed,
                ColorChoice::Auto,
            ),
            WriteLogger::new(
                log_level, 
                config,
                log_file,
            ),
        ])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        // Log initial message to confirm logging is working
        log::info!("Logging initialized to file: {}", log_path);
        log::debug!("Debug logging enabled");
        
        Ok(log_path)
    }
    
    /// Helper function to write a direct message to the log file
    /// Useful for debugging when the logging system itself may have issues
    pub fn write_direct_to_log(log_path: &str, message: &str) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(log_path)?;
            
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        writeln!(file, "[{}] DIRECT: {}", timestamp, message)
    }
}

// Module with the server implementation
mod server {
    use std::env;
    
    use dotenv::dotenv;
    use gmail::GmailClient;
    use gmail::model::Message;
    use mcp_attr::jsoncall::ErrorCode;
    use mcp_attr::server::{mcp_server, McpServer, serve_stdio};
    use mcp_attr::{Error as McpError, Result as McpResult};
    use log::{info, debug, error, warn};
    use serde::{Deserialize, Serialize};
    
    // Helper struct for converting headers to a more convenient format
    #[derive(Serialize, Deserialize, Debug)]
    struct EmailMessage {
        id: String,
        thread_id: String,
        subject: Option<String>,
        from: Option<String>,
        to: Option<String>,
        date: Option<String>,
        snippet: Option<String>,
    }
    
    // MCP server for accessing Gmail API
    #[derive(Clone)]
    pub struct GmailServer;
    
    impl GmailServer {
        pub fn new() -> Self {
            // Load environment variables from .env file if present
            dotenv().ok();
            GmailServer {}
        }
    
        // Extract email details from a Gmail message
        fn extract_email_details(&self, message: Message) -> EmailMessage {
            debug!("extract_email_details for message ID: {}", message.id);
            debug!("Message thread ID: {}", message.thread_id);
            debug!("Message snippet: {}", message.snippet);
            
            // Initialize header values
            let mut subject = None;
            let mut from = None;
            let mut to = None;
            let mut date = None;
            
            // Extract headers if payload.headers exists and is not empty
            let headers = &message.payload.headers;
            debug!("Found {} headers in message", headers.len());
            
            if !headers.is_empty() {
                for header in headers {
                    debug!("Processing header: {} = {}", header.name, header.value);
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
                        _ => {}
                    }
                }
            }
            
            debug!("Creating EmailMessage with extracted details");
            debug!("Subject: {:?}", subject);
            debug!("From: {:?}", from);
            debug!("To: {:?}", to);
            debug!("Date: {:?}", date);
            
            let email = EmailMessage {
                id: message.id.clone(),
                thread_id: message.thread_id.clone(),
                subject,
                from,
                to,
                date,
                snippet: Some(message.snippet.clone()),
            };
            
            debug!("EmailMessage created successfully");
            email
        }
        
        // Helper function to check required environment variables
        pub fn check_required_env_vars(&self) -> Result<(), String> {
            debug!("Checking GMAIL_CLIENT_ID");
            match env::var("GMAIL_CLIENT_ID") {
                Ok(val) => debug!("GMAIL_CLIENT_ID is set (length: {})", val.len()),
                Err(e) => {
                    error!("GMAIL_CLIENT_ID is missing: {}", e);
                    return Err("Missing environment variable: GMAIL_CLIENT_ID".to_string());
                }
            }
            
            debug!("Checking GMAIL_CLIENT_SECRET");
            match env::var("GMAIL_CLIENT_SECRET") {
                Ok(val) => debug!("GMAIL_CLIENT_SECRET is set (length: {})", val.len()),
                Err(e) => {
                    error!("GMAIL_CLIENT_SECRET is missing: {}", e);
                    return Err("Missing environment variable: GMAIL_CLIENT_SECRET".to_string());
                }
            }
            
            debug!("Checking GMAIL_REFRESH_TOKEN");
            match env::var("GMAIL_REFRESH_TOKEN") {
                Ok(val) => debug!("GMAIL_REFRESH_TOKEN is set (length: {})", val.len()),
                Err(e) => {
                    error!("GMAIL_REFRESH_TOKEN is missing: {}", e);
                    return Err("Missing environment variable: GMAIL_REFRESH_TOKEN".to_string());
                }
            }
            
            debug!("All required environment variables are present");
            Ok(())
        }
        
        // Helper function to create McpError
        fn to_mcp_error(&self, message: &str) -> McpError {
            // Use a numeric error code of 1000 for application errors
            error!("Creating MCP error: {}", message);
            McpError::new(ErrorCode(1000))
        }
        
        // Helper function to create a Gmail client safely
        fn create_gmail_client(&self) -> Result<GmailClient, String> {
            debug!("create_gmail_client called");
            
            // First check that all required environment variables are set
            debug!("Checking required environment variables");
            if let Err(e) = self.check_required_env_vars() {
                error!("Environment variables check failed: {}", e);
                return Err(e);
            }
            debug!("All required environment variables are present");
            
            // Get the access token if available (optional)
            let access_token = env::var("GMAIL_ACCESS_TOKEN").ok();
            if let Some(token) = &access_token {
                debug!("Found access token (length: {})", token.len());
                if token.len() >= 10 {
                    debug!("Access token starts with: {}", &token.chars().take(10).collect::<String>());
                }
            } else {
                debug!("No access token found, will use refresh token only");
            }
            
            // Get the required refresh token
            debug!("Getting refresh token");
            let refresh_token = match env::var("GMAIL_REFRESH_TOKEN") {
                Ok(token) => {
                    debug!("Found refresh token (length: {})", token.len());
                    if token.len() >= 10 {
                        debug!("Refresh token starts with: {}", &token.chars().take(10).collect::<String>());
                    }
                    token
                },
                Err(e) => {
                    error!("Error getting refresh token: {}", e);
                    return Err(format!("Failed to get GMAIL_REFRESH_TOKEN: {}", e));
                }
            };
            
            // Create the auth context using oauth2
            debug!("Creating GmailAuth object with oauth2");
            let auth = gmail::GmailAuth::oauth2(
                access_token.unwrap_or_default(),
                refresh_token,
                None, // No callback for refresh
            );
            debug!("GmailAuth object created successfully");
            
            // Create and return the client
            debug!("Creating GmailClient with auth");
            let client = GmailClient::with_auth(auth);
            debug!("GmailClient created successfully");
            
            Ok(client)
        }
    }
    
    // MCP server implementation
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
        ///   max_results: Optional maximum number of results to return (default: 10)
        ///   query: Optional Gmail search query string (e.g. "is:unread from:example.com")
        #[tool]
        async fn list_emails(&self, max_results: Option<u32>, query: Option<String>) -> McpResult<String> {
            info!("=== START list_emails MCP command ===");
            debug!("list_emails called with max_results={:?}, query={:?}", max_results, query);
            
            // Log MCP command execution to monitor performance
            let start_time = std::time::Instant::now();
            debug!("Command execution started at {:?}", chrono::Local::now());
            
            // Process parameters
            let max = max_results.unwrap_or(10);
            debug!("Resolved max_results parameter to {} messages", max);
            debug!("Query parameter is {}", if query.is_some() { "present" } else { "not present" });
            
            debug!("======== ENVIRONMENT CHECK ========");
            // Check environment before doing anything
            match std::env::var("GMAIL_CLIENT_ID") {
                Ok(val) => debug!("GMAIL_CLIENT_ID is set with length {}", val.len()),
                Err(e) => warn!("GMAIL_CLIENT_ID not available: {}", e),
            }
            
            match std::env::var("GMAIL_CLIENT_SECRET") {
                Ok(val) => debug!("GMAIL_CLIENT_SECRET is set with length {}", val.len()),
                Err(e) => warn!("GMAIL_CLIENT_SECRET not available: {}", e),
            }
            
            match std::env::var("GMAIL_REFRESH_TOKEN") {
                Ok(val) => debug!("GMAIL_REFRESH_TOKEN is set with length {}", val.len()),
                Err(e) => warn!("GMAIL_REFRESH_TOKEN not available: {}", e),
            }
            
            match std::env::var("GMAIL_ACCESS_TOKEN") {
                Ok(val) => debug!("GMAIL_ACCESS_TOKEN is set with length {}", val.len()),
                Err(e) => debug!("GMAIL_ACCESS_TOKEN not available: {}", e),
            }
            
            match dotenv::dotenv() {
                Ok(path) => debug!("Loaded .env file from: {:?}", path),
                Err(e) => debug!("No .env file loaded: {}", e),
            }
            debug!("======== END ENVIRONMENT CHECK ========");
            
            // Create Gmail client
            debug!("======== CLIENT CREATION ========");
            debug!("Creating Gmail client for API access");
            let client_start = std::time::Instant::now();
            let client = match self.create_gmail_client() {
                Ok(client) => {
                    let elapsed = client_start.elapsed();
                    debug!("Gmail client created successfully in {:?}", elapsed);
                    client
                },
                Err(err) => {
                    error!("Failed to create Gmail client: {}", err);
                    error!("This is likely due to authentication issues");
                    return Err(self.to_mcp_error(&err));
                }
            };
            debug!("======== END CLIENT CREATION ========");
            
            // Set up request
            debug!("======== REQUEST PREPARATION ========");
            debug!("Setting up Gmail messages_list request");
            let mut request = client.messages_list("me");
            debug!("Set user_id parameter to 'me'");
            
            request = request.max_results(max.into());
            debug!("Set max_results parameter to {}", max);
            
            if let Some(q) = query.clone() {
                debug!("Adding query parameter: {}", q);
                request = request.q(&q);
                debug!("Gmail search query parameter added successfully");
            } else {
                debug!("No search query provided, will return messages based on default sorting");
            }
            debug!("======== END REQUEST PREPARATION ========");
            
            // Send request
            debug!("======== API REQUEST EXECUTION ========");
            debug!("Sending request to Gmail API (messages.list endpoint)");
            let request_start = std::time::Instant::now();
            let response = match request.await {
                Ok(r) => {
                    let elapsed = request_start.elapsed();
                    debug!("Received successful response from Gmail API in {:?}", elapsed);
                    debug!("Response received at {:?}", chrono::Local::now());
                    r
                },
                Err(e) => {
                    error!("Gmail API error when calling messages.list: {}", e);
                    error!("API request failed after {:?}", request_start.elapsed());
                    let error_msg = format!("Gmail API error: {}", e);
                    return Err(self.to_mcp_error(&error_msg));
                }
            };
            debug!("======== END API REQUEST EXECUTION ========");
            
            // Process response
            debug!("======== RESPONSE PROCESSING ========");
            if let Some(messages) = response.messages.clone() {
                let count = messages.len();
                info!("Found {} messages in response", count);
                
                if count == 0 {
                    debug!("No messages returned despite successful API call");
                    return Ok("[]".to_string());
                }
                
                debug!("Will process {} message(s) to extract details", count);
                let mut email_details = Vec::with_capacity(count);
                
                debug!("Starting individual message retrieval loop");
                for (index, msg_ref) in messages.iter().enumerate() {
                    debug!("-------- Processing message {}/{} --------", index + 1, count);
                    debug!("Message ID: {} | Thread ID: {:?}", msg_ref.id, msg_ref.thread_id);
                    
                    let msg_start = std::time::Instant::now();
                    debug!("Fetching full message details for ID: {}", msg_ref.id);
                    debug!("API call: messages.get with ID {}", msg_ref.id);
                    let message = match client.messages_get(&msg_ref.id, "me").format("full").await {
                        Ok(m) => {
                            let elapsed = msg_start.elapsed();
                            debug!("Successfully retrieved message details for ID: {} in {:?}", msg_ref.id, elapsed);
                            debug!("Message size estimate: {:?} bytes", m.size_estimate);
                            debug!("Message has {:?} labels", m.label_ids);
                            m
                        },
                        Err(e) => {
                            error!("Error retrieving message details for ID {}: {}", msg_ref.id, e);
                            error!("Individual message retrieval failed after {:?}", msg_start.elapsed());
                            let error_msg = format!("Gmail API error when retrieving message {}: {}", msg_ref.id, e);
                            return Err(self.to_mcp_error(&error_msg));
                        }
                    };
                    
                    debug!("Extracting email details for message ID: {}", message.id);
                    let extract_start = std::time::Instant::now();
                    let email = self.extract_email_details(message);
                    debug!("Email extraction completed in {:?}", extract_start.elapsed());
                    
                    // Log summary of extracted email
                    debug!("Extracted email summary:");
                    debug!("  Subject: {:?}", email.subject);
                    debug!("  From: {:?}", email.from);
                    debug!("  To: {:?}", email.to);
                    debug!("  Date: {:?}", email.date);
                    debug!("  Snippet length: {} characters", email.snippet.as_ref().map_or(0, |s| s.len()));
                    
                    debug!("Adding email to results list");
                    email_details.push(email);
                    debug!("-------- End processing message {}/{} --------", index + 1, count);
                }
                
                debug!("All {} messages processed successfully", count);
                debug!("Converting {} email details to JSON", email_details.len());
                
                let json_start = std::time::Instant::now();
                match serde_json::to_string_pretty(&email_details) {
                    Ok(json) => {
                        let elapsed = json_start.elapsed();
                        debug!("JSON serialization successful in {:?}", elapsed);
                        info!("Returning JSON response with {} characters", json.len());
                        
                        if json.len() > 200 {
                            debug!("First 200 chars of JSON: {}", &json[..200.min(json.len())]);
                            debug!("Last 100 chars of JSON: {}", &json[json.len()-100.min(json.len())..]);
                        } else {
                            debug!("Full JSON content: {}", json);
                        }
                        
                        info!("=== END list_emails MCP command (success) ===");
                        info!("Total execution time: {:?}", start_time.elapsed());
                        Ok(json)
                    },
                    Err(e) => {
                        error!("JSON serialization error: {}", e);
                        error!("Failed to serialize {} email objects", email_details.len());
                        let error_msg = format!("JSON serialization error: {}", e);
                        error!("=== END list_emails MCP command (error) ===");
                        Err(self.to_mcp_error(&error_msg))
                    }
                }
            } else {
                debug!("No 'messages' field in API response or it was empty");
                info!("No messages found in response, returning empty array");
                info!("=== END list_emails MCP command (empty result) ===");
                info!("Total execution time: {:?}", start_time.elapsed());
                Ok("[]".to_string())
            }
        }
        
        /// Get details for a specific email
        /// 
        /// Args:
        ///   message_id: The ID of the message to retrieve
        #[tool]
        async fn get_email(&self, message_id: String) -> McpResult<String> {
            // Create Gmail client
            let client = match self.create_gmail_client() {
                Ok(client) => client,
                Err(err) => return Err(self.to_mcp_error(&err)),
            };
            
            // Get message
            let message = match client.messages_get(&message_id, "me").format("full").await {
                Ok(m) => m,
                Err(e) => {
                    let error_msg = format!("Gmail API error: {}", e);
                    return Err(self.to_mcp_error(&error_msg));
                }
            };
            
            // Process message
            let email = self.extract_email_details(message);
            
            // Return JSON response
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
        ///   max_results: Optional maximum number of results (default: 10)
        #[tool]
        async fn search_emails(&self, query: String, max_results: Option<u32>) -> McpResult<String> {
            // This is essentially the same as list_emails but with a required query parameter
            self.list_emails(max_results, Some(query)).await
        }
        
        /// Get a list of email labels
        /// 
        /// Returns a list of all labels in the user's mailbox
        #[tool]
        async fn list_labels(&self) -> McpResult<String> {
            // Create Gmail client
            let client = match self.create_gmail_client() {
                Ok(client) => client,
                Err(err) => return Err(self.to_mcp_error(&err)),
            };
            
            // Get labels
            let response = match client.labels_list("me").await {
                Ok(r) => r,
                Err(e) => {
                    let error_msg = format!("Gmail API error: {}", e);
                    return Err(self.to_mcp_error(&error_msg));
                }
            };
            
            // Return labels as JSON
            if let Some(labels) = response.labels {
                match serde_json::to_string_pretty(&labels) {
                    Ok(json) => Ok(json),
                    Err(e) => {
                        let error_msg = format!("JSON serialization error: {}", e);
                        Err(self.to_mcp_error(&error_msg))
                    }
                }
            } else {
                Ok("[]".to_string())
            }
        }
        
        /// Check connection status with Gmail API
        /// 
        /// Tests the connection to Gmail API by retrieving the user's profile
        #[tool]
        async fn check_connection(&self) -> McpResult<String> {
            // Create Gmail client
            let client = match self.create_gmail_client() {
                Ok(client) => client,
                Err(err) => return Err(self.to_mcp_error(&err)),
            };
            
            // Get profile
            let profile = match client.get_profile("me").await {
                Ok(p) => p,
                Err(e) => {
                    let error_msg = format!("Gmail API error: {}", e);
                    return Err(self.to_mcp_error(&error_msg));
                }
            };
            
            // Format response
            let email = profile.email_address.unwrap_or_else(|| "Unknown".to_string());
            let messages_total = profile.messages_total.unwrap_or(0);
            
            Ok(format!("Connection successful!\nEmail: {}\nTotal messages: {}", email, messages_total))
        }
    }
    
}