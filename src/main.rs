// ABOUTME: Gmail MCP server using rust-mcp-sdk
// ABOUTME: Supports CLI arguments for transport, OAuth, and configuration

mod calendar_tools;
mod gmail_tools;
mod oauth_tools;
mod people_tools;
mod tools;

use async_trait::async_trait;
use clap::Parser;
use mcp_gmailcal::cli::{Cli, Commands};
use oauth_tools::OAuthState;
use rust_mcp_sdk::{
    error::SdkResult,
    mcp_server::{server_runtime, ServerHandler, ServerRuntime},
    schema::{
        CallToolRequest, CallToolResult, GetPromptRequest, GetPromptResult, Implementation,
        InitializeResult, ListPromptsRequest, ListPromptsResult, ListResourcesRequest,
        ListResourcesResult, ListToolsRequest, ListToolsResult, Prompt, ReadResourceRequest,
        ReadResourceResult, Resource, RpcError, ServerCapabilities, ServerCapabilitiesTools,
        TextResourceContents, LATEST_PROTOCOL_VERSION,
    },
    McpServer, StdioTransport, TransportOptions,
};
use tools::GmailMcpTools;

#[tokio::main]
async fn main() -> SdkResult<()> {
    // Parse command line arguments
    let args = Cli::parse();

    // Initialize logging - use stderr to avoid interfering with JSON-RPC on stdout
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Stderr)
        .init();

    // Handle commands
    match args.command {
        Some(Commands::Auth) => {
            eprintln!("🔑 Starting OAuth authentication flow...");
            run_oauth_flow().await?;
            return Ok(());
        }
        Some(Commands::Test) => {
            eprintln!("🧪 Testing current credentials...");
            test_credentials().await?;
            return Ok(());
        }
        Some(Commands::Server) | None => {
            // Default behavior - run the server
        }
    }

    // Determine transport type
    match args.transport.as_str() {
        "sse" => {
            eprintln!(
                "🚀 Starting Gmail MCP Server (rust-mcp-sdk) with SSE transport on {}:{}...",
                args.host, args.port
            );
            run_sse_server(args.host, args.port).await?;
        }
        "stdio" => {
            eprintln!("🚀 Starting Gmail MCP Server (rust-mcp-sdk) with stdio transport...");
            run_stdio_server().await?;
        }
        _ => {
            eprintln!("❌ Unknown transport type: {}", args.transport);
            eprintln!("💡 Supported transports: stdio, sse");
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn run_stdio_server() -> SdkResult<()> {
    // Define server details and capabilities
    let server_details = InitializeResult {
        server_info: Implementation {
            name: "Gmail MCP Server".to_string(),
            version: "0.2.0".to_string(),
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            resources: Some(rust_mcp_sdk::schema::ServerCapabilitiesResources {
                subscribe: Some(false),
                list_changed: Some(false)
            }),
            prompts: Some(rust_mcp_sdk::schema::ServerCapabilitiesPrompts {
                list_changed: Some(false)
            }),
            ..Default::default()
        },
        meta: Some(serde_json::from_value(serde_json::json!({
            "oauth": {
                "provider": "google",
                "auth_url": "https://accounts.google.com/o/oauth2/auth",
                "token_url": "https://oauth2.googleapis.com/token",
                "scopes": [
                    "https://mail.google.com/",
                    "https://www.googleapis.com/auth/calendar.readonly",
                    "https://www.googleapis.com/auth/calendar",
                    "https://www.googleapis.com/auth/contacts.readonly",
                    "https://www.googleapis.com/auth/directory.readonly"
                ],
                "redirect_uri": "http://localhost:8080/oauth/callback",
                "flow": "authorization_code",
                "tools": {
                    "get_oauth_url": "Start OAuth flow and get authorization URL",
                    "complete_oauth": "Complete OAuth flow with authorization code",
                    "auth_status": "Check current authentication status"
                }
            },
            "apis": {
                "gmail": "Full Gmail API access for emails, labels, and drafts",
                "calendar": "Google Calendar API for events and calendars",
                "contacts": "Google People API for contacts and directory access"
            }
        })).expect("Failed to convert meta")),
        instructions: Some("Gmail MCP Server provides direct access to Gmail API using OAuth authentication. Use get_oauth_url to start authentication, complete_oauth to finish the flow, and auth_status to check your authentication status. Supports email management, contacts, and calendar operations.".to_string()),
        protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
    };

    // Create a stdio transport with default options
    let transport = StdioTransport::new(TransportOptions::default())?;

    // Create shared OAuth state
    let oauth_state = OAuthState::new();

    // Instantiate custom handler for MCP messages
    let handler = GmailServerHandler { oauth_state };

    eprintln!("✅ Server configuration created");

    // Create a MCP server using the correct pattern
    let server: ServerRuntime = server_runtime::create_server(server_details, transport, handler);

    eprintln!("🔧 Starting MCP server runtime...");

    // Start the server
    server.start().await
}

async fn run_sse_server(_host: String, _port: u16) -> SdkResult<()> {
    eprintln!("❌ SSE transport not yet implemented for rust-mcp-sdk server");
    eprintln!("💡 Use stdio transport instead: cargo run -- --transport stdio");

    // TODO: Implement SSE transport for rust-mcp-sdk when available
    Ok(())
}

async fn run_oauth_flow() -> SdkResult<()> {
    eprintln!("📋 OAuth Authentication Flow");
    eprintln!("This will help you set up Gmail API credentials.");
    eprintln!("Please follow these steps:");
    eprintln!("1. Go to the Google Cloud Console (https://console.cloud.google.com/)");
    eprintln!("2. Create or select a project");
    eprintln!("3. Enable the Gmail API");
    eprintln!("4. Create OAuth 2.0 credentials");
    eprintln!("5. Use the OAuth tools in the MCP server to complete authentication:");
    eprintln!("   - get_oauth_url: Generate authorization URL");
    eprintln!("   - complete_oauth: Exchange authorization code for tokens");
    eprintln!("   - auth_status: Check authentication status");
    eprintln!("For detailed instructions, see the README.md file.");

    Ok(())
}

async fn test_credentials() -> SdkResult<()> {
    use mcp_gmailcal::config::Config;

    eprintln!("🔍 Testing current credentials...");

    match Config::from_env() {
        Ok(config) => {
            eprintln!("✅ Configuration loaded successfully");
            eprintln!("   Client ID: {}", config.client_id);
            eprintln!("   Has refresh token: {}", !config.refresh_token.is_empty());
            eprintln!("   Has access token: {}", config.access_token.is_some());

            eprintln!("💡 Use the MCP tools to test actual API connectivity");
        }
        Err(e) => {
            eprintln!("❌ Configuration error: {}", e);
            eprintln!("💡 Run 'cargo run -- auth' to set up OAuth credentials");
        }
    }

    Ok(())
}

// Custom handler implementation
pub struct GmailServerHandler {
    oauth_state: OAuthState,
}

#[async_trait]
impl ServerHandler for GmailServerHandler {
    async fn handle_list_tools_request(
        &self,
        _request: ListToolsRequest,
        _runtime: &dyn rust_mcp_sdk::McpServer,
    ) -> Result<ListToolsResult, rust_mcp_sdk::schema::RpcError> {
        eprintln!("📋 Handling list_tools_request");

        // Use the tools module to get available tools
        Ok(GmailMcpTools::list_all_tools())
    }

    async fn handle_call_tool_request(
        &self,
        request: CallToolRequest,
        _runtime: &dyn rust_mcp_sdk::McpServer,
    ) -> Result<CallToolResult, rust_mcp_sdk::schema::schema_utils::CallToolError> {
        eprintln!(
            "🔧 Handling call_tool_request for tool: {}",
            request.params.name
        );

        // Use the tools module to handle the request
        GmailMcpTools::call_tool(request, &self.oauth_state).await
    }

    async fn handle_list_resources_request(
        &self,
        _request: ListResourcesRequest,
        _runtime: &dyn rust_mcp_sdk::McpServer,
    ) -> Result<ListResourcesResult, RpcError> {
        eprintln!("📁 Handling list_resources_request");

        Ok(ListResourcesResult {
            resources: vec![Resource {
                uri: "oauth://metadata".to_string(),
                name: "OAuth Configuration".to_string(),
                description: Some(
                    "OAuth configuration metadata for Google API authentication".to_string(),
                ),
                mime_type: Some("application/json".to_string()),
                size: None,
                annotations: None,
            }],
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_read_resource_request(
        &self,
        request: ReadResourceRequest,
        _runtime: &dyn rust_mcp_sdk::McpServer,
    ) -> Result<ReadResourceResult, RpcError> {
        eprintln!(
            "📄 Handling read_resource_request for: {}",
            request.params.uri
        );

        match request.params.uri.as_str() {
            "oauth://metadata" => {
                let oauth_metadata = serde_json::json!({
                    "provider": "google",
                    "auth_url": "https://accounts.google.com/o/oauth2/auth",
                    "token_url": "https://oauth2.googleapis.com/token",
                    "client_id_required": true,
                    "client_secret_required": true,
                    "scopes": [
                        "https://mail.google.com/",
                        "https://www.googleapis.com/auth/calendar.readonly",
                        "https://www.googleapis.com/auth/calendar",
                        "https://www.googleapis.com/auth/contacts.readonly",
                        "https://www.googleapis.com/auth/directory.readonly"
                    ],
                    "redirect_uri": "http://localhost:8080/oauth/callback",
                    "response_type": "code",
                    "access_type": "offline",
                    "prompt": "consent",
                    "flow_type": "authorization_code",
                    "tools": {
                        "start": "get_oauth_url",
                        "complete": "complete_oauth",
                        "status": "auth_status"
                    }
                });

                Ok(ReadResourceResult {
                    contents: vec![
                        rust_mcp_sdk::schema::ReadResourceResultContentsItem::TextResourceContents(
                            TextResourceContents {
                                uri: request.params.uri,
                                mime_type: Some("application/json".to_string()),
                                text: oauth_metadata.to_string(),
                            },
                        ),
                    ],
                    meta: None,
                })
            }
            _ => Err(RpcError {
                code: -32602,
                message: format!("Unknown resource: {}", request.params.uri),
                data: None,
            }),
        }
    }

    async fn handle_list_prompts_request(
        &self,
        _request: ListPromptsRequest,
        _runtime: &dyn rust_mcp_sdk::McpServer,
    ) -> Result<ListPromptsResult, RpcError> {
        eprintln!("📝 Handling list_prompts_request");

        Ok(ListPromptsResult {
            prompts: vec![Prompt {
                name: "oauth_setup".to_string(),
                description: Some(
                    "Step-by-step OAuth setup guide for Google API authentication".to_string(),
                ),
                arguments: vec![],
            }],
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_get_prompt_request(
        &self,
        request: GetPromptRequest,
        _runtime: &dyn rust_mcp_sdk::McpServer,
    ) -> Result<GetPromptResult, RpcError> {
        eprintln!(
            "💬 Handling get_prompt_request for: {}",
            request.params.name
        );

        match request.params.name.as_str() {
            "oauth_setup" => {
                let oauth_guide = r#"
# OAuth Setup Guide for Gmail MCP Server

## Prerequisites
1. Google Cloud Console account
2. Project with Gmail, Calendar, and People APIs enabled
3. OAuth 2.0 credentials (Client ID and Secret)

## Step-by-Step Setup

### 1. Get OAuth URL
Call the `get_oauth_url` tool with your Google OAuth Client ID:
```json
{
  "client_id": "your-google-client-id.googleusercontent.com"
}
```

### 2. Authorize in Browser
- Open the returned authorization URL in your browser
- Sign in to your Google account
- Grant permissions for Gmail, Calendar, and Contacts access
- Copy the authorization code from the callback URL

### 3. Complete OAuth
Call the `complete_oauth` tool with the authorization code:
```json
{
  "auth_code": "4/0AX4XfWh...",
  "client_id": "your-google-client-id.googleusercontent.com",
  "client_secret": "your-google-client-secret"
}
```

### 4. Verify Status
Call the `auth_status` tool to verify authentication:
```json
{}
```

## Required Scopes
- `https://mail.google.com/` - Full Gmail access
- `https://www.googleapis.com/auth/calendar` - Calendar read/write
- `https://www.googleapis.com/auth/calendar.readonly` - Calendar read
- `https://www.googleapis.com/auth/contacts.readonly` - Contacts read
- `https://www.googleapis.com/auth/directory.readonly` - Directory read

## Security Notes
- Client secret should be kept secure
- Tokens are stored in memory only
- Re-authentication required on server restart
"#;

                Ok(GetPromptResult {
                    description: Some(
                        "Complete OAuth setup guide for Google API authentication".to_string(),
                    ),
                    messages: vec![rust_mcp_sdk::schema::PromptMessage {
                        role: rust_mcp_sdk::schema::Role::User,
                        content: rust_mcp_sdk::schema::PromptMessageContent::TextContent(
                            rust_mcp_sdk::schema::TextContent::new(oauth_guide.to_string(), None),
                        ),
                    }],
                    meta: None,
                })
            }
            _ => Err(RpcError {
                code: -32602,
                message: format!("Unknown prompt: {}", request.params.name),
                data: None,
            }),
        }
    }
}
