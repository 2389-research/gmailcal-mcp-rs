# OAuth Inside MCP Server - Implementation Plan

## Current State Analysis

### What We Have
- **External OAuth flow** via `oauth.rs` that runs a separate web server
- OAuth credentials stored in `.env` files and passed as environment variables
- MCP server reads credentials from environment at startup
- Separate `cargo run -- --oauth` command for initial authentication

### What MCP Specification Supports
Based on analysis of https://modelcontextprotocol.io/llms-full.txt:
- **No explicit OAuth framework** in MCP spec
- **Security considerations** mentioned but implementation-agnostic
- **Client-specific OAuth** (e.g., Klavis AI with Slack/Discord) exists
- **Server authentication** left to individual implementations

## OAuth Inside MCP Server - Technical Feasibility

### ✅ Possible Approaches

#### 1. **MCP Tool-Based OAuth** (Recommended)
```rust
// New MCP tools for OAuth management
impl McpServer for GmailServer {
    #[tool(description = "Start OAuth flow and return authorization URL")]
    async fn start_oauth_flow(&self) -> McpResult<String> {
        // Generate auth URL, return to client
        // Client opens URL in browser
    }

    #[tool(description = "Complete OAuth flow with authorization code")]
    async fn complete_oauth_flow(&self, auth_code: String) -> McpResult<String> {
        // Exchange code for tokens
        // Store securely in server memory/cache
        // Return success message
    }

    #[tool(description = "Check OAuth status")]
    async fn oauth_status(&self) -> McpResult<String> {
        // Return current authentication status
    }
}
```

#### 2. **Resource-Based OAuth Status**
```rust
#[resource(uri = "oauth://status", description = "Current OAuth status")]
async fn oauth_status_resource(&self) -> McpResult<String> {
    // Return JSON with auth status, expiry, scopes
}

#[resource(uri = "oauth://config", description = "OAuth configuration")]
async fn oauth_config_resource(&self) -> McpResult<String> {
    // Return client ID, scopes, redirect URI
}
```

#### 3. **Prompt-Based OAuth Guidance**
```rust
#[prompt(name = "oauth_setup", description = "Guide user through OAuth setup")]
async fn oauth_setup_prompt(&self) -> McpResult<String> {
    // Return step-by-step OAuth instructions
    // Include links, commands, troubleshooting
}
```

### ❌ Not Possible with Current MCP Spec
- **Direct browser control** from MCP server
- **HTTP server endpoints** exposed to external traffic
- **Real-time callback handling** during OAuth flow
- **Persistent credential storage** across MCP sessions

## Implementation Strategy

### Phase 1: Enhanced OAuth Tools
```rust
// Add to server.rs
#[tool(description = "Generate OAuth authorization URL")]
async fn get_oauth_url(&self, client_id: Option<String>) -> McpResult<String> {
    let config = self.get_or_prompt_oauth_config(client_id).await?;
    let auth_url = generate_auth_url(&config)?;
    Ok(json!({
        "authorization_url": auth_url,
        "instructions": "1. Open this URL in your browser\n2. Complete Google OAuth\n3. Copy the 'code' parameter from redirect URL\n4. Use complete_oauth tool with the code",
        "redirect_uri": config.redirect_uri
    }).to_string())
}

#[tool(description = "Complete OAuth with authorization code")]
async fn complete_oauth(&self, auth_code: String, client_secret: String) -> McpResult<String> {
    // Exchange code for tokens
    // Store in server memory for session
    // Update internal client configurations
    Ok("OAuth completed successfully".to_string())
}

#[tool(description = "Check current authentication status")]
async fn auth_status(&self) -> McpResult<String> {
    // Check if we have valid tokens
    // Return expiry times, scopes, email
}
```

### Phase 2: Memory-Based Token Management
```rust
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct GmailServer {
    oauth_tokens: Arc<RwLock<Option<OAuthTokens>>>,
}

struct OAuthTokens {
    access_token: String,
    refresh_token: String,
    expires_at: chrono::DateTime<chrono::Utc>,
    email: String,
}
```

### Phase 3: Seamless Tool Integration
```rust
// Modify existing tools to handle OAuth state
#[tool(description = "List Gmail messages")]
async fn list_messages(&self, query: Option<String>) -> McpResult<String> {
    // Check if authenticated
    if !self.is_authenticated().await {
        return Ok(json!({
            "error": "Not authenticated",
            "action_required": "Use get_oauth_url tool to start authentication"
        }).to_string());
    }

    // Continue with normal Gmail operations
    let service = self.init_gmail_service().await?;
    // ...
}
```

## Security Considerations

### ✅ Secure Practices
- **In-memory token storage** (no disk persistence)
- **Token expiry handling** with automatic refresh
- **Scope validation** before tool execution
- **Error sanitization** to prevent token leakage

### ⚠️ Limitations
- **Session-based auth** - tokens lost when MCP server restarts
- **Client secret handling** - must be provided by user each time
- **No persistent storage** - user must re-auth each session

## User Experience Flow

### First-Time Setup
1. User calls `get_oauth_url` tool with client ID
2. MCP returns authorization URL and instructions
3. User opens URL, completes OAuth in browser
4. User copies authorization code from redirect
5. User calls `complete_oauth` with code and client secret
6. MCP server stores tokens in memory
7. All Gmail tools now work normally

### Subsequent Sessions
1. User must repeat OAuth flow each time (no persistence)
2. Or: Load tokens from external source via tool

## Implementation Timeline

### Week 1: Core OAuth Tools
- [ ] Add `get_oauth_url` tool
- [ ] Add `complete_oauth` tool
- [ ] Add `auth_status` tool
- [ ] Update existing tools to check auth state

### Week 2: Integration & Polish
- [ ] Add OAuth prompts and resources
- [ ] Enhance error messages with OAuth guidance
- [ ] Add automatic token refresh logic
- [ ] Write comprehensive documentation

### Week 3: Advanced Features
- [ ] Add OAuth configuration management
- [ ] Support multiple OAuth profiles
- [ ] Add OAuth troubleshooting tools
- [ ] Performance optimization

## Conclusion

**YES, we can implement OAuth inside the MCP server** using tools and resources. The approach provides:

✅ **Better UX** - OAuth integrated into Claude conversations
✅ **No external dependencies** - No separate OAuth command needed
✅ **Session management** - Tokens managed by MCP server
✅ **Clear error handling** - OAuth status visible to Claude

❌ **Trade-offs** - Session-based (not persistent), requires manual code copy

This approach transforms OAuth from an external setup step into a natural part of the MCP conversation flow.
