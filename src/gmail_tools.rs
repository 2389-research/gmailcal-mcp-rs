// ABOUTME: Gmail tools module for Gmail MCP server
// ABOUTME: Implements all Gmail tools using rust-mcp-sdk patterns

use crate::oauth_tools::OAuthState;
use rust_mcp_sdk::{
    schema::schema_utils::CallToolError,
    schema::{CallToolResult, CallToolResultContentItem, TextContent},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Error;

// Draft email request structure
#[derive(Debug, Serialize, Deserialize)]
pub struct DraftEmailRequest {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub cc: Option<String>,
    pub bcc: Option<String>,
    pub thread_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
}

// Simple Gmail service implementation for production use
pub struct SimpleGmailService {
    client: reqwest::Client,
    access_token: String,
}

impl SimpleGmailService {
    pub fn new(access_token: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            access_token,
        }
    }

    async fn request_gmail_api(
        &self,
        endpoint: &str,
        query: Option<&[(&str, &str)]>,
    ) -> Result<String, CallToolError> {
        let url = format!("https://gmail.googleapis.com/gmail/v1{}", endpoint);

        let mut req_builder = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Accept", "application/json");

        if let Some(q) = query {
            req_builder = req_builder.query(q);
        }

        let response = req_builder.send().await.map_err(|e| {
            CallToolError::new(Error::other(format!("Gmail API request failed: {}", e)))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CallToolError::new(Error::other(format!(
                "Gmail API error - Status: {}, Error: {}",
                status, error_text
            ))));
        }

        response.text().await.map_err(|e| {
            CallToolError::new(Error::other(format!("Failed to read response: {}", e)))
        })
    }

    pub async fn list_messages(
        &self,
        max_results: u32,
        query: Option<&str>,
    ) -> Result<String, CallToolError> {
        let max_results_str = max_results.to_string();
        let endpoint = "/users/me/messages";

        let params = if let Some(q) = query {
            vec![("maxResults", max_results_str.as_str()), ("q", q)]
        } else {
            vec![("maxResults", max_results_str.as_str())]
        };

        self.request_gmail_api(endpoint, Some(&params)).await
    }

    pub async fn get_message(&self, message_id: &str) -> Result<String, CallToolError> {
        let endpoint = format!("/users/me/messages/{}", message_id);
        let params = [("format", "full")];

        self.request_gmail_api(&endpoint, Some(&params)).await
    }

    pub async fn list_labels(&self) -> Result<String, CallToolError> {
        let endpoint = "/users/me/labels";
        self.request_gmail_api(endpoint, None).await
    }

    pub async fn get_profile(&self) -> Result<String, CallToolError> {
        let endpoint = "/users/me/profile";
        self.request_gmail_api(endpoint, None).await
    }

    pub async fn create_draft(
        &self,
        draft_email: &DraftEmailRequest,
    ) -> Result<String, CallToolError> {
        let endpoint = "/users/me/drafts";

        // Build the email message in RFC 2822 format
        let mut message = String::new();
        message.push_str(&format!("To: {}\r\n", draft_email.to));
        if let Some(ref cc) = draft_email.cc {
            message.push_str(&format!("Cc: {}\r\n", cc));
        }
        if let Some(ref bcc) = draft_email.bcc {
            message.push_str(&format!("Bcc: {}\r\n", bcc));
        }
        message.push_str(&format!("Subject: {}\r\n", draft_email.subject));
        if let Some(ref in_reply_to) = draft_email.in_reply_to {
            message.push_str(&format!("In-Reply-To: {}\r\n", in_reply_to));
        }
        if let Some(ref references) = draft_email.references {
            message.push_str(&format!("References: {}\r\n", references));
        }
        message.push_str("\r\n");
        message.push_str(&draft_email.body);

        // Encode message in base64url
        let encoded_message = base64::encode_config(&message, base64::URL_SAFE_NO_PAD);

        let draft_request = serde_json::json!({
            "message": {
                "raw": encoded_message,
                "threadId": draft_email.thread_id
            }
        });

        let response = self
            .client
            .post(format!("https://gmail.googleapis.com/gmail/v1{}", endpoint))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(&draft_request)
            .send()
            .await
            .map_err(|e| {
                CallToolError::new(Error::other(format!("Draft creation failed: {}", e)))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CallToolError::new(Error::other(format!(
                "Draft creation error - Status: {}, Error: {}",
                status, error_text
            ))));
        }

        response.text().await.map_err(|e| {
            CallToolError::new(Error::other(format!(
                "Failed to read draft response: {}",
                e
            )))
        })
    }
}

// Helper function to parse max_results parameter
fn parse_max_results(max_results: Option<serde_json::Value>, default: u32) -> u32 {
    match max_results {
        Some(val) => match val {
            serde_json::Value::Number(n) => n.as_u64().unwrap_or(default as u64) as u32,
            serde_json::Value::String(s) => s.parse::<u32>().unwrap_or(default),
            _ => default,
        },
        None => default,
    }
}

// Tool 1: list_emails
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListEmailsTool {
    /// Maximum number of results to return (default: 10)
    pub max_results: Option<serde_json::Value>,
    /// Optional Gmail search query string (e.g. "is:unread from:example.com")
    pub query: Option<String>,
}

impl ListEmailsTool {
    pub async fn call_tool(
        &self,
        oauth_state: &OAuthState,
    ) -> Result<CallToolResult, CallToolError> {
        let tokens = oauth_state.tokens.read().await;
        let oauth_tokens = tokens.as_ref().ok_or_else(|| {
            CallToolError::new(Error::other(
                "Not authenticated - use get_oauth_url to start OAuth flow",
            ))
        })?;

        let max = parse_max_results(self.max_results.clone(), 10);
        let service = SimpleGmailService::new(oauth_tokens.access_token.clone());
        let result = service.list_messages(max, self.query.as_deref()).await?;

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                result, None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}

// Tool 2: get_email
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetEmailTool {
    /// The ID of the message to retrieve
    pub message_id: String,
}

impl GetEmailTool {
    pub async fn call_tool(
        &self,
        oauth_state: &OAuthState,
    ) -> Result<CallToolResult, CallToolError> {
        let tokens = oauth_state.tokens.read().await;
        let oauth_tokens = tokens.as_ref().ok_or_else(|| {
            CallToolError::new(Error::other(
                "Not authenticated - use get_oauth_url to start OAuth flow",
            ))
        })?;

        let service = SimpleGmailService::new(oauth_tokens.access_token.clone());
        let result = service.get_message(&self.message_id).await?;

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                result, None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}

// Tool 3: search_emails
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchEmailsTool {
    /// Gmail search query string (e.g. "is:unread from:example.com")
    pub query: String,
    /// Optional maximum number of results (default: 10)
    pub max_results: Option<serde_json::Value>,
}

impl SearchEmailsTool {
    pub async fn call_tool(
        &self,
        oauth_state: &OAuthState,
    ) -> Result<CallToolResult, CallToolError> {
        let tokens = oauth_state.tokens.read().await;
        let oauth_tokens = tokens.as_ref().ok_or_else(|| {
            CallToolError::new(Error::other(
                "Not authenticated - use get_oauth_url to start OAuth flow",
            ))
        })?;

        let max = parse_max_results(self.max_results.clone(), 10);
        let service = SimpleGmailService::new(oauth_tokens.access_token.clone());
        let result = service.list_messages(max, Some(&self.query)).await?;

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                result, None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}

// Tool 4: list_labels
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListLabelsTool {}

impl ListLabelsTool {
    pub async fn call_tool(
        &self,
        oauth_state: &OAuthState,
    ) -> Result<CallToolResult, CallToolError> {
        let tokens = oauth_state.tokens.read().await;
        let oauth_tokens = tokens.as_ref().ok_or_else(|| {
            CallToolError::new(Error::other(
                "Not authenticated - use get_oauth_url to start OAuth flow",
            ))
        })?;

        let service = SimpleGmailService::new(oauth_tokens.access_token.clone());
        let result = service.list_labels().await?;

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                result, None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}

// Tool 5: check_connection
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CheckConnectionTool {}

impl CheckConnectionTool {
    pub async fn call_tool(
        &self,
        oauth_state: &OAuthState,
    ) -> Result<CallToolResult, CallToolError> {
        let tokens = oauth_state.tokens.read().await;
        let oauth_tokens = tokens.as_ref().ok_or_else(|| {
            CallToolError::new(Error::other(
                "Not authenticated - use get_oauth_url to start OAuth flow",
            ))
        })?;

        let service = SimpleGmailService::new(oauth_tokens.access_token.clone());
        let result = service.get_profile().await?;

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                result, None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}

// Tool 6: analyze_email (simplified version for now)
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeEmailTool {
    /// The ID of the message to analyze
    pub message_id: String,
    /// Optional type of analysis to perform
    pub analysis_type: Option<String>,
}

impl AnalyzeEmailTool {
    pub async fn call_tool(
        &self,
        oauth_state: &OAuthState,
    ) -> Result<CallToolResult, CallToolError> {
        let tokens = oauth_state.tokens.read().await;
        let oauth_tokens = tokens.as_ref().ok_or_else(|| {
            CallToolError::new(Error::other(
                "Not authenticated - use get_oauth_url to start OAuth flow",
            ))
        })?;

        let service = SimpleGmailService::new(oauth_tokens.access_token.clone());
        let email_json = service.get_message(&self.message_id).await?;

        // Parse the email for basic analysis
        let analysis_type = self.analysis_type.as_deref().unwrap_or("general");

        let result = json!({
            "email_id": self.message_id,
            "analysis_type": analysis_type,
            "email_data": serde_json::from_str::<serde_json::Value>(&email_json).unwrap_or(json!({})),
            "message": "Email analysis complete - content returned for further processing"
        });

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                result.to_string(),
                None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}

// Tool 7: batch_analyze_emails
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BatchAnalyzeEmailsTool {
    /// List of email IDs to analyze
    pub message_ids: Vec<String>,
    /// Optional type of analysis to perform
    pub analysis_type: Option<String>,
}

impl BatchAnalyzeEmailsTool {
    pub async fn call_tool(
        &self,
        oauth_state: &OAuthState,
    ) -> Result<CallToolResult, CallToolError> {
        let tokens = oauth_state.tokens.read().await;
        let oauth_tokens = tokens.as_ref().ok_or_else(|| {
            CallToolError::new(Error::other(
                "Not authenticated - use get_oauth_url to start OAuth flow",
            ))
        })?;

        let service = SimpleGmailService::new(oauth_tokens.access_token.clone());
        let analysis_type = self.analysis_type.as_deref().unwrap_or("summary");

        let mut results = Vec::new();
        for id in &self.message_ids {
            match service.get_message(id).await {
                Ok(email_json) => {
                    let result = json!({
                        "email_id": id,
                        "analysis_type": analysis_type,
                        "email_data": serde_json::from_str::<serde_json::Value>(&email_json).unwrap_or(json!({})),
                        "status": "success"
                    });
                    results.push(result);
                }
                Err(_) => {
                    results.push(json!({
                        "email_id": id,
                        "error": "Failed to retrieve email",
                        "status": "error"
                    }));
                }
            }
        }

        let batch_result = json!({
            "analysis_type": analysis_type,
            "email_count": results.len(),
            "results": results
        });

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                batch_result.to_string(),
                None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}

// Tool 8: create_draft_email
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateDraftEmailTool {
    /// Email address(es) of the recipient(s). Multiple addresses should be comma-separated.
    pub to: String,
    /// Subject line of the email
    pub subject: String,
    /// Plain text content of the email
    pub body: String,
    /// Optional CC recipient(s). Multiple addresses should be comma-separated.
    pub cc: Option<String>,
    /// Optional BCC recipient(s). Multiple addresses should be comma-separated.
    pub bcc: Option<String>,
    /// Optional Gmail thread ID to associate this email with
    pub thread_id: Option<String>,
    /// Optional Message-ID that this email is replying to
    pub in_reply_to: Option<String>,
    /// Optional comma-separated list of Message-IDs in the email thread
    pub references: Option<String>,
}

impl CreateDraftEmailTool {
    pub async fn call_tool(
        &self,
        oauth_state: &OAuthState,
    ) -> Result<CallToolResult, CallToolError> {
        let tokens = oauth_state.tokens.read().await;
        let oauth_tokens = tokens.as_ref().ok_or_else(|| {
            CallToolError::new(Error::other(
                "Not authenticated - use get_oauth_url to start OAuth flow",
            ))
        })?;

        // Validate required fields
        if self.to.is_empty() {
            return Err(CallToolError::new(Error::other(
                "Recipient (to) is required for creating a draft email",
            )));
        }

        let draft_request = DraftEmailRequest {
            to: self.to.clone(),
            subject: self.subject.clone(),
            body: self.body.clone(),
            cc: self.cc.clone(),
            bcc: self.bcc.clone(),
            thread_id: self.thread_id.clone(),
            in_reply_to: self.in_reply_to.clone(),
            references: self.references.clone(),
        };

        let service = SimpleGmailService::new(oauth_tokens.access_token.clone());
        let draft_response = service.create_draft(&draft_request).await?;

        // Parse the response to extract draft ID
        let draft_data: serde_json::Value = serde_json::from_str(&draft_response).map_err(|e| {
            CallToolError::new(Error::other(format!(
                "Failed to parse draft response: {}",
                e
            )))
        })?;

        let mut result = json!({
            "status": "success",
            "message": "Draft email created successfully.",
            "draft_response": draft_data
        });

        // Add threading info if provided
        if let Some(ref thread_id) = self.thread_id {
            result["thread_id"] = json!(thread_id);
        }

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                result.to_string(),
                None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}
