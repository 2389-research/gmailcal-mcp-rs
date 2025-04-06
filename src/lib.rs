// Error exports
pub use crate::errors::{
    ConfigError, GmailApiError, PeopleApiError, CalendarApiError,
    GmailResult, PeopleResult, CalendarResult, error_codes
};
pub use crate::config::{Config, GMAIL_API_BASE_URL, OAUTH_TOKEN_URL, get_token_expiry_seconds};
pub use crate::gmail_api::{EmailMessage, DraftEmail, GmailService};
pub use crate::logging::setup_logging;
pub use crate::auth::TokenManager;
pub use crate::people_api::{
    Contact, EmailAddress, Organization, PeopleClient, 
    PersonName, PhoneNumber, Photo, ContactList
};
pub use crate::calendar_api::{
    CalendarClient, CalendarEvent, CalendarList, CalendarInfo,
    Attendee, EventOrganizer, ConferenceData, ConferenceSolution, EntryPoint
};
pub use crate::prompts::*;
pub use crate::utils::{
    parse_max_results, decode_base64, encode_base64_url_safe, 
    to_mcp_error, map_gmail_error, error_codes as utils_error_codes
};

// Module for error handling
pub mod errors;
// Module for configuration
pub mod config;
// Module for logging
pub mod logging;
// Module for utilities
pub mod utils;
// Module for authentication
pub mod auth;
// Module for Gmail API
pub mod gmail_api;
// Module for People API
pub mod people_api;
// Module for Calendar API
pub mod calendar_api;
// Module for prompt templates
pub mod prompts;
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

// Module with the server implementation
pub mod server {
    use log::{debug, error, info};
    use mcp_attr::jsoncall::ErrorCode;
    use mcp_attr::server::{mcp_server, McpServer};
    use mcp_attr::{Error as McpError, Result as McpResult};
    use serde_json::json;

    use crate::config::Config;
    use crate::errors::ConfigError;
    use crate::gmail_api::GmailService;
    use crate::errors::GmailApiError;
    use crate::utils::error_codes;

    // Helper functions
    mod helpers {
        // Re-export the parse_max_results function from utils
        pub use crate::utils::parse_max_results;
    }

    // Error codes have been moved to the utils module

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
            // Delegate to the utility function
            crate::utils::to_mcp_error(message, code)
        }

        // Helper function to map GmailApiError to detailed McpError with specific codes
        fn map_gmail_error(&self, err: GmailApiError) -> McpError {
            // Delegate to the utility function
            crate::utils::map_gmail_error(err)
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
        #[allow(clippy::too_many_arguments)]
        async fn create_draft_email(
            &self,
            // Required content
            to: String,
            subject: String,
            body: String,
            // Optional recipients
            cc: Option<String>,
            bcc: Option<String>,
            // Optional threading
            thread_id: Option<String>,
            in_reply_to: Option<String>,
            // Additional options
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
        #[allow(clippy::too_many_arguments)]
        async fn create_event(
            &self,
            // Calendar identification
            calendar_id: Option<String>,
            // Event core details
            summary: String,
            start_time: String,
            end_time: String,
            // Optional event details
            description: Option<String>,
            location: Option<String>,
            // Participants
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

// OAuth authentication module for token refresh flow
pub mod oauth {
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

