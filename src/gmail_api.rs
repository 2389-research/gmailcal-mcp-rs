use crate::auth::TokenManager;
use crate::config::Config;
use crate::config::GMAIL_API_BASE_URL;
use crate::errors::{GmailApiError, GmailResult};
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

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

// Alias for backward compatibility within this module
type Result<T> = GmailResult<T>;

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
                GmailApiError::MessageFormatError("Message missing 'threadId' field".to_string())
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
            GmailApiError::MessageFormatError("Missing 'messages' array in response".to_string())
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
        // Encode the message as base64url format for Gmail API
        // Note: For large messages with attachments or nested MIME structures,
        // we would need enhanced handling to process them in chunks or parts
        // This handles the basic email case - more complex handling would be needed for
        // large attachments, nested content, or multipart messages exceeding size limits
        let encoded_message = base64::encode(message.as_bytes())
            .replace('+', "-")
            .replace('/', "_");

        // Log the message size for debugging large messages
        debug!("Encoded message size: {} bytes", encoded_message.len());

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
                GmailApiError::MessageFormatError(format!("Failed to parse draft response: {}", e))
            })?;

        // Extract the draft ID
        let draft_id = response_json["id"]
            .as_str()
            .ok_or_else(|| {
                GmailApiError::MessageFormatError("Draft response missing 'id' field".to_string())
            })?
            .to_string();

        debug!("Draft created successfully with ID: {}", draft_id);

        Ok(draft_id)
    }
}
