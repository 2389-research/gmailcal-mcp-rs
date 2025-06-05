// ABOUTME: Calendar tools module for Gmail MCP server
// ABOUTME: Implements list_calendars, list_events, get_event, and create_event tools using rust-mcp-sdk patterns

use crate::oauth_tools::OAuthState;
use rust_mcp_sdk::{
    schema::schema_utils::CallToolError,
    schema::{CallToolResult, CallToolResultContentItem, TextContent},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Error;

// Simple Calendar API service implementation
pub struct SimpleCalendarService {
    client: reqwest::Client,
    access_token: String,
}

impl SimpleCalendarService {
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

    async fn request_calendar_api(
        &self,
        endpoint: &str,
        query: Option<&[(&str, &str)]>,
    ) -> Result<String, CallToolError> {
        let url = format!("https://www.googleapis.com/calendar/v3{}", endpoint);

        let mut req_builder = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Accept", "application/json");

        if let Some(q) = query {
            req_builder = req_builder.query(q);
        }

        let response = req_builder.send().await.map_err(|e| {
            CallToolError::new(Error::other(format!("Calendar API request failed: {}", e)))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CallToolError::new(Error::other(format!(
                "Calendar API error - Status: {}, Error: {}",
                status, error_text
            ))));
        }

        response.text().await.map_err(|e| {
            CallToolError::new(Error::other(format!("Failed to read response: {}", e)))
        })
    }

    pub async fn list_calendars(&self) -> Result<String, CallToolError> {
        let endpoint = "/users/me/calendarList";
        self.request_calendar_api(endpoint, None).await
    }

    pub async fn list_events(
        &self,
        calendar_id: &str,
        max_results: Option<u32>,
        time_min: Option<&str>,
        time_max: Option<&str>,
    ) -> Result<String, CallToolError> {
        let endpoint = format!("/calendars/{}/events", urlencoding::encode(calendar_id));

        let mut params = Vec::new();

        if let Some(max) = max_results {
            params.push(("maxResults", max.to_string()));
        }
        if let Some(min) = time_min {
            params.push(("timeMin", min.to_string()));
        }
        if let Some(max) = time_max {
            params.push(("timeMax", max.to_string()));
        }

        // Convert to string references
        let params_refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        self.request_calendar_api(
            &endpoint,
            if params_refs.is_empty() {
                None
            } else {
                Some(&params_refs)
            },
        )
        .await
    }

    pub async fn get_event(
        &self,
        calendar_id: &str,
        event_id: &str,
    ) -> Result<String, CallToolError> {
        let endpoint = format!(
            "/calendars/{}/events/{}",
            urlencoding::encode(calendar_id),
            urlencoding::encode(event_id)
        );

        self.request_calendar_api(&endpoint, None).await
    }

    pub async fn create_event(
        &self,
        calendar_id: &str,
        event_data: &serde_json::Value,
    ) -> Result<String, CallToolError> {
        let endpoint = format!("/calendars/{}/events", urlencoding::encode(calendar_id));
        let url = format!("https://www.googleapis.com/calendar/v3{}", endpoint);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(event_data)
            .send()
            .await
            .map_err(|e| {
                CallToolError::new(Error::other(format!("Event creation failed: {}", e)))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CallToolError::new(Error::other(format!(
                "Event creation error - Status: {}, Error: {}",
                status, error_text
            ))));
        }

        response.text().await.map_err(|e| {
            CallToolError::new(Error::other(format!(
                "Failed to read event response: {}",
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

// Tool 1: list_calendars
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListCalendarsTool {}

impl ListCalendarsTool {
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

        let service = SimpleCalendarService::new(oauth_tokens.access_token.clone());
        let result = service.list_calendars().await?;

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                result, None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}

// Tool 2: list_events
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListEventsTool {
    /// The ID of the calendar to get events from (optional, defaults to primary)
    pub calendar_id: Option<String>,
    /// Optional maximum number of events to return
    pub max_results: Option<serde_json::Value>,
    /// Optional minimum time bound (RFC3339 timestamp)
    pub time_min: Option<String>,
    /// Optional maximum time bound (RFC3339 timestamp)
    pub time_max: Option<String>,
}

impl ListEventsTool {
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

        let calendar_id = self.calendar_id.as_deref().unwrap_or("primary");
        let max = parse_max_results(self.max_results.clone(), 10);

        let service = SimpleCalendarService::new(oauth_tokens.access_token.clone());
        let result = service
            .list_events(
                calendar_id,
                Some(max),
                self.time_min.as_deref(),
                self.time_max.as_deref(),
            )
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

// Tool 3: get_event
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetEventTool {
    /// The ID of the calendar (optional, defaults to primary)
    pub calendar_id: Option<String>,
    /// The ID of the event to retrieve
    pub event_id: String,
}

impl GetEventTool {
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

        let calendar_id = self.calendar_id.as_deref().unwrap_or("primary");

        let service = SimpleCalendarService::new(oauth_tokens.access_token.clone());
        let result = service.get_event(calendar_id, &self.event_id).await?;

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                result, None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}

// Tool 4: create_event
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateEventTool {
    /// The ID of the calendar (optional, defaults to primary)
    pub calendar_id: Option<String>,
    /// The title of the event
    pub summary: String,
    /// Start time in RFC3339 format
    pub start_time: String,
    /// End time in RFC3339 format
    pub end_time: String,
    /// Optional description of the event
    pub description: Option<String>,
    /// Optional location of the event
    pub location: Option<String>,
    /// Optional list of attendee emails
    pub attendees: Option<Vec<String>>,
}

impl CreateEventTool {
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

        let calendar_id = self.calendar_id.as_deref().unwrap_or("primary");

        // Build event data
        let mut event = json!({
            "summary": self.summary,
            "start": {
                "dateTime": self.start_time
            },
            "end": {
                "dateTime": self.end_time
            }
        });

        if let Some(ref description) = self.description {
            event["description"] = json!(description);
        }

        if let Some(ref location) = self.location {
            event["location"] = json!(location);
        }

        if let Some(ref attendees) = self.attendees {
            let attendee_objects: Vec<serde_json::Value> = attendees
                .iter()
                .map(|email| json!({"email": email}))
                .collect();
            event["attendees"] = json!(attendee_objects);
        }

        let service = SimpleCalendarService::new(oauth_tokens.access_token.clone());
        let result = service.create_event(calendar_id, &event).await?;

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                result, None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}
