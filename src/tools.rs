// ABOUTME: Tools coordination module for Gmail MCP server
// ABOUTME: Combines OAuth and Gmail tools with rust-mcp-sdk patterns

use rust_mcp_sdk::{
    schema::schema_utils::CallToolError,
    schema::{CallToolRequest, CallToolResult, ListToolsResult, Tool, ToolInputSchema},
};
use schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::Error;

use crate::calendar_tools::{CreateEventTool, GetEventTool, ListCalendarsTool, ListEventsTool};
use crate::gmail_tools::{
    AnalyzeEmailTool, BatchAnalyzeEmailsTool, CheckConnectionTool, CreateDraftEmailTool,
    GetEmailTool, ListEmailsTool, ListLabelsTool, SearchEmailsTool,
};
use crate::oauth_tools::{AuthStatusTool, CompleteOAuthTool, GetOAuthUrlTool, OAuthState};
use crate::people_tools::{GetContactTool, ListContactsTool, SearchContactsTool};

// Helper function to convert schemars Schema to ToolInputSchema
fn schema_to_tool_input_schema(schema: Schema) -> Result<ToolInputSchema, String> {
    match schema {
        Schema::Object(schema_obj) => {
            // Extract properties
            let properties = schema_obj.object.as_ref().map(|obj| {
                obj.properties
                    .iter()
                    .map(|(key, schema)| {
                        let value = serde_json::to_value(schema).unwrap_or(Value::Null);
                        let map = if let Value::Object(m) = value {
                            m
                        } else {
                            Map::new()
                        };
                        (key.clone(), map)
                    })
                    .collect::<HashMap<String, Map<String, Value>>>()
            });

            // Extract required fields
            let required = schema_obj
                .object
                .as_ref()
                .map(|obj| obj.required.iter().cloned().collect::<Vec<String>>())
                .unwrap_or_default();

            Ok(ToolInputSchema::new(required, properties))
        }
        _ => Err("Schema must be an object type".to_string()),
    }
}

// Main tools implementation
pub struct GmailMcpTools;

impl GmailMcpTools {
    pub fn list_all_tools() -> ListToolsResult {
        let mut generator = SchemaGenerator::default();

        ListToolsResult {
            tools: vec![
                // OAuth tools
                Tool {
                    name: "get_oauth_url".to_string(),
                    description: Some("🔐 START OAUTH FLOW: Generate Google OAuth authorization URL for Gmail, Calendar, and Contacts access. This is the first step in authentication - use this to get the URL to authorize the application.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        GetOAuthUrlTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: Some(serde_json::from_value(serde_json::json!({
                        "oauth_step": 1,
                        "oauth_flow": "authorization_code",
                        "required_for": "All Gmail, Calendar, and Contacts functionality",
                        "scopes": [
                            "https://mail.google.com/",
                            "https://www.googleapis.com/auth/calendar.readonly",
                            "https://www.googleapis.com/auth/calendar",
                            "https://www.googleapis.com/auth/contacts.readonly",
                            "https://www.googleapis.com/auth/directory.readonly"
                        ]
                    })).expect("Failed to convert annotations")),
                },
                Tool {
                    name: "complete_oauth".to_string(),
                    description: Some("🔑 COMPLETE OAUTH FLOW: Exchange authorization code for access tokens. Use this after visiting the OAuth URL from get_oauth_url and getting the authorization code from the callback.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        CompleteOAuthTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: Some(serde_json::from_value(serde_json::json!({
                        "oauth_step": 2,
                        "oauth_flow": "authorization_code",
                        "depends_on": "get_oauth_url",
                        "stores_tokens": true,
                        "enables": "All Gmail, Calendar, and Contacts tools"
                    })).expect("Failed to convert annotations")),
                },
                Tool {
                    name: "auth_status".to_string(),
                    description: Some("📊 CHECK AUTH STATUS: Verify current OAuth authentication status and token validity. Use this to check if you're authenticated and if tokens need refreshing.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        AuthStatusTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: Some(serde_json::from_value(serde_json::json!({
                        "oauth_step": "status",
                        "oauth_flow": "authorization_code",
                        "returns": ["authentication_status", "token_info", "email", "scopes"]
                    })).expect("Failed to convert annotations")),
                },
                // Gmail tools
                Tool {
                    name: "list_emails".to_string(),
                    description: Some("Get a list of emails from the inbox. Returns emails with subject, sender, recipient, date and snippet information.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        ListEmailsTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
                Tool {
                    name: "get_email".to_string(),
                    description: Some("Get details for a specific email. Returns the message with all metadata and content parsed.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        GetEmailTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
                Tool {
                    name: "search_emails".to_string(),
                    description: Some("Search for emails using a Gmail search query. Returns emails matching the query.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        SearchEmailsTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
                Tool {
                    name: "list_labels".to_string(),
                    description: Some("Get a list of email labels. Returns the raw JSON response from the Gmail API.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        ListLabelsTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
                Tool {
                    name: "check_connection".to_string(),
                    description: Some("Check connection status with Gmail API. Tests the connection by retrieving the user's profile.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        CheckConnectionTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
                Tool {
                    name: "analyze_email".to_string(),
                    description: Some("Analyze an email to extract key information. Takes an email ID and performs analysis.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        AnalyzeEmailTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
                Tool {
                    name: "batch_analyze_emails".to_string(),
                    description: Some("Batch analyze multiple emails. Takes a list of email IDs and performs quick analysis on each one.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        BatchAnalyzeEmailsTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
                Tool {
                    name: "create_draft_email".to_string(),
                    description: Some("Create a draft email in Gmail. Creates a new draft email with the specified content.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        CreateDraftEmailTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
                // People/Contacts tools
                Tool {
                    name: "list_contacts".to_string(),
                    description: Some("List contacts from Google Contacts. Retrieves a list of contacts with names, emails, and phone numbers.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        ListContactsTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
                Tool {
                    name: "search_contacts".to_string(),
                    description: Some("Search for contacts matching a query. Searches through contact names, emails, and other fields.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        SearchContactsTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
                Tool {
                    name: "get_contact".to_string(),
                    description: Some("Get a specific contact by resource name. Retrieves detailed contact information.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        GetContactTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
                // Calendar tools
                Tool {
                    name: "list_calendars".to_string(),
                    description: Some("List all available calendars. Retrieves a list of all calendars the user has access to.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        ListCalendarsTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
                Tool {
                    name: "list_events".to_string(),
                    description: Some("List events from a calendar. Retrieves events from a specified calendar with optional filtering.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        ListEventsTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
                Tool {
                    name: "get_event".to_string(),
                    description: Some("Get a single calendar event. Retrieves a specific event from a calendar.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        GetEventTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
                Tool {
                    name: "create_event".to_string(),
                    description: Some("Create a new calendar event. Creates a new event in the specified calendar.".to_string()),
                    input_schema: schema_to_tool_input_schema(
                        CreateEventTool::json_schema(&mut generator)
                    ).expect("Failed to convert schema"),
                    annotations: None,
                },
            ],
            meta: None,
            next_cursor: None,
        }
    }

    pub async fn call_tool(
        request: CallToolRequest,
        oauth_state: &OAuthState,
    ) -> Result<CallToolResult, CallToolError> {
        match request.params.name.as_str() {
            // OAuth tools
            "get_oauth_url" => {
                let tool: GetOAuthUrlTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            "complete_oauth" => {
                let tool: CompleteOAuthTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            "auth_status" => {
                let tool: AuthStatusTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            // Gmail tools
            "list_emails" => {
                let tool: ListEmailsTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            "get_email" => {
                let tool: GetEmailTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            "search_emails" => {
                let tool: SearchEmailsTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            "list_labels" => {
                let tool: ListLabelsTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            "check_connection" => {
                let tool: CheckConnectionTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            "analyze_email" => {
                let tool: AnalyzeEmailTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            "batch_analyze_emails" => {
                let tool: BatchAnalyzeEmailsTool = serde_json::from_value(
                    serde_json::Value::Object(request.params.arguments.unwrap_or_default()),
                )
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            "create_draft_email" => {
                let tool: CreateDraftEmailTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            // People/Contacts tools
            "list_contacts" => {
                let tool: ListContactsTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            "search_contacts" => {
                let tool: SearchContactsTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            "get_contact" => {
                let tool: GetContactTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            // Calendar tools
            "list_calendars" => {
                let tool: ListCalendarsTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            "list_events" => {
                let tool: ListEventsTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            "get_event" => {
                let tool: GetEventTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            "create_event" => {
                let tool: CreateEventTool = serde_json::from_value(serde_json::Value::Object(
                    request.params.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    CallToolError::new(Error::other(format!(
                        "Failed to parse tool arguments: {}",
                        e
                    )))
                })?;
                tool.call_tool(oauth_state).await
            }
            _ => Err(CallToolError::unknown_tool(request.params.name.clone())),
        }
    }
}
