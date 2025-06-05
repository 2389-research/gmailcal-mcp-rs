// ABOUTME: People/Contacts tools module for Gmail MCP server
// ABOUTME: Implements list_contacts, search_contacts, and get_contact tools using rust-mcp-sdk patterns

use crate::oauth_tools::OAuthState;
use rust_mcp_sdk::{
    schema::schema_utils::CallToolError,
    schema::{CallToolResult, CallToolResultContentItem, TextContent},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::io::Error;

// Simple People API service implementation
pub struct SimplePeopleService {
    client: reqwest::Client,
    access_token: String,
}

impl SimplePeopleService {
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

    async fn request_people_api(
        &self,
        endpoint: &str,
        query: Option<&[(&str, &str)]>,
    ) -> Result<String, CallToolError> {
        let url = format!("https://people.googleapis.com/v1{}", endpoint);

        let mut req_builder = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Accept", "application/json");

        if let Some(q) = query {
            req_builder = req_builder.query(q);
        }

        let response = req_builder.send().await.map_err(|e| {
            CallToolError::new(Error::other(format!("People API request failed: {}", e)))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CallToolError::new(Error::other(format!(
                "People API error - Status: {}, Error: {}",
                status, error_text
            ))));
        }

        response.text().await.map_err(|e| {
            CallToolError::new(Error::other(format!("Failed to read response: {}", e)))
        })
    }

    pub async fn list_contacts(&self, max_results: Option<u32>) -> Result<String, CallToolError> {
        let endpoint = "/people/me/connections";

        let mut params = vec![(
            "personFields",
            "names,emailAddresses,phoneNumbers,organizations,photos",
        )];

        let max_str;
        if let Some(max) = max_results {
            max_str = max.to_string();
            params.push(("pageSize", &max_str));
        }

        self.request_people_api(endpoint, Some(&params)).await
    }

    pub async fn search_contacts(
        &self,
        query: &str,
        max_results: Option<u32>,
    ) -> Result<String, CallToolError> {
        let endpoint = "/people:searchContacts";

        let mut params = vec![
            ("query", query),
            (
                "readMask",
                "names,emailAddresses,phoneNumbers,organizations,photos",
            ),
        ];

        let max_str;
        if let Some(max) = max_results {
            max_str = max.to_string();
            params.push(("pageSize", &max_str));
        }

        self.request_people_api(endpoint, Some(&params)).await
    }

    pub async fn get_contact(&self, resource_name: &str) -> Result<String, CallToolError> {
        let endpoint = format!("/{}", resource_name);
        let params = [(
            "personFields",
            "names,emailAddresses,phoneNumbers,organizations,photos,addresses,birthdays",
        )];

        self.request_people_api(&endpoint, Some(&params)).await
    }
}

// Tool 1: list_contacts
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListContactsTool {
    /// Optional maximum number of contacts to return
    pub max_results: Option<u32>,
}

impl ListContactsTool {
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

        let service = SimplePeopleService::new(oauth_tokens.access_token.clone());
        let result = service.list_contacts(self.max_results).await?;

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                result, None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}

// Tool 2: search_contacts
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchContactsTool {
    /// The search query
    pub query: String,
    /// Optional maximum number of contacts to return
    pub max_results: Option<u32>,
}

impl SearchContactsTool {
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

        let service = SimplePeopleService::new(oauth_tokens.access_token.clone());
        let result = service
            .search_contacts(&self.query, self.max_results)
            .await?;

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                result, None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}

// Tool 3: get_contact
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetContactTool {
    /// The resource name of the contact to retrieve
    pub resource_name: String,
}

impl GetContactTool {
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

        let service = SimplePeopleService::new(oauth_tokens.access_token.clone());
        let result = service.get_contact(&self.resource_name).await?;

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                result, None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}
