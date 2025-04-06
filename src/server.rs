use log::{debug, error, info};
use mcp_attr::server::{mcp_server, McpServer};
use mcp_attr::{Error as McpError, Result as McpResult};
use serde_json::json;

use crate::config::Config;
use crate::errors::ConfigError;
use crate::errors::GmailApiError;
use crate::gmail_api::GmailService;
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
    async fn init_calendar_service(&self) -> Result<crate::calendar_api::CalendarClient, McpError> {
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
                error!(
                    "Context: Failed to list emails with parameters: max_results={}, query='{}'",
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

                    // Add error entry to results with more detailed information
                    results.push(json!({
                        "email_id": id,
                        "error": format!("Failed to retrieve email: {}", err),
                        "message": "This email failed processing but other emails in the batch will continue to process",
                        "status": "error"
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
    async fn search_contacts(&self, query: String, max_results: Option<u32>) -> McpResult<String> {
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
                    let error_msg = format!("Failed to serialize contact search results: {}", e);
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
                    let error_msg = format!("Invalid time_min format (expected RFC3339): {}", e);
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
                    let error_msg = format!("Invalid time_max format (expected RFC3339): {}", e);
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
    async fn get_event(&self, calendar_id: Option<String>, event_id: String) -> McpResult<String> {
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
