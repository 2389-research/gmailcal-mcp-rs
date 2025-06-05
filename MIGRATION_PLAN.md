# Migration Plan: mcp-attr → rust-mcp-sdk

## 📋 Overview
Migrate from the third-party `mcp-attr` crate to the official `rust-mcp-sdk` to ensure compatibility with the latest MCP protocol version (2025-03-26) and maintain long-term support.

## 🎯 Goals
- ✅ Support MCP protocol version 2025-03-26
- ✅ Maintain all existing functionality
- ✅ Improve long-term maintainability
- ✅ Ensure future protocol compatibility
- ✅ No breaking changes for end users

## Current State Analysis

### Current Architecture (mcp-attr)
- **Server Definition**: Uses `#[mcp_server]` attribute macro
- **Tools**: Defined with `#[tool]` attribute on async methods
- **Prompts**: Defined with `#[prompt]` attribute on async methods
- **Error Handling**: Custom `McpError` and `McpResult` types
- **Transport**: Automatic stdio transport via `serve_stdio()`
- **Protocol Version**: Likely supports 2024-11-05 (outdated)

### Target Architecture (rust-mcp-sdk)
- **Server Definition**: Implement `ServerHandler` trait manually
- **Tools**: Define tool structs with JSON schema, handle in `handle_call_tool_request`
- **Prompts**: Handle in `handle_list_prompts_request` and `handle_get_prompt_request`
- **Error Handling**: Use SDK's error types (`RpcError`, `CallToolError`, etc.)
- **Transport**: Explicit transport creation (`StdioTransport`, `SseTransport`)
- **Protocol Version**: Supports 2025-03-26 by default

## 🚀 Migration Strategy

### Phase 1: Research & Setup (1-2 days) 🔍
1. **API Discovery**
   - [ ] Create minimal test project with rust-mcp-sdk
   - [ ] Document exact import paths and type names
   - [ ] Test basic server creation and tool handling
   - [ ] Identify schema generation approach (manual vs macros)

2. **Dependencies Update**
   ```toml
   # Replace
   mcp-attr = "0.0.7"

   # With
   rust-mcp-sdk = "0.4.2"
   async-trait = "0.1"
   schemars = { version = "0.8", features = ["derive"] }
   ```

### Phase 2: Core Infrastructure (2-3 days) 🏗️
1. **Server Trait Implementation**
   - [ ] Replace `GmailServer` struct with `ServerHandler` implementation
   - [ ] Migrate OAuth token storage (should remain unchanged)
   - [ ] Update error handling to use SDK types

2. **Transport Layer**
   - [ ] Replace `serve_stdio()` with explicit `StdioTransport`
   - [ ] Update SSE transport if needed
   - [ ] Ensure protocol version is set to latest

3. **Schema Definitions**
   - [ ] Convert tool parameters to schema-compatible structs
   - [ ] Add `schemars::JsonSchema` derives
   - [ ] Test schema generation

### Phase 3: Tool Migration (3-4 days) 🔧
**Priority Order: OAuth tools → Core email tools → Extended features**

#### OAuth Tools (Day 1)
- [ ] `get_oauth_url` - Convert to struct + handler method
- [ ] `complete_oauth` - Convert to struct + handler method
- [ ] `auth_status` - Convert to struct + handler method

#### Core Email Tools (Day 2)
- [ ] `list_emails` - Convert to struct + handler method
- [ ] `get_email` - Convert to struct + handler method
- [ ] `search_emails` - Convert to struct + handler method

#### Gmail Management Tools (Day 3)
- [ ] `create_draft_email` - Convert to struct + handler method
- [ ] `analyze_email` - Convert to struct + handler method
- [ ] `batch_analyze_emails` - Convert to struct + handler method

#### Extended Tools (Day 4)
- [ ] `list_contacts` - Convert to struct + handler method
- [ ] `search_contacts` - Convert to struct + handler method
- [ ] `get_contact` - Convert to struct + handler method
- [ ] `list_calendars` - Convert to struct + handler method
- [ ] `list_events` - Convert to struct + handler method
- [ ] `get_event` - Convert to struct + handler method
- [ ] `create_event` - Convert to struct + handler method

### Phase 4: Prompts & Resources (1 day) 📝
1. **Prompts Migration**
   - [ ] `email_drafting_prompt` - Convert to prompt handler
   - [ ] `oauth_threading_prompt` - Convert to prompt handler

2. **Resources** (if applicable)
   - [ ] Assess if any resources need migration
   - [ ] Implement resource handlers if needed

### Phase 5: Testing & Validation (2-3 days) 🧪
1. **Unit Tests**
   - [ ] Update all tool tests for new API
   - [ ] Test schema generation and validation
   - [ ] Verify OAuth flow still works

2. **Integration Tests**
   - [ ] Test with MCP inspector (latest version)
   - [ ] Verify protocol version compatibility
   - [ ] Test SSE transport functionality

3. **Manual Testing**
   - [ ] Full OAuth flow
   - [ ] All email operations
   - [ ] Calendar and contacts functionality
   - [ ] Error scenarios

### Phase 6: Cleanup & Documentation (1 day) 🧹
1. **Code Cleanup**
   - [ ] Remove `mcp-attr` dependencies
   - [ ] Clean up unused imports and functions
   - [ ] Update lib.rs exports

2. **Documentation Updates**
   - [ ] Update CLAUDE.md with any API changes
   - [ ] Update README if needed
   - [ ] Document any breaking changes

## 📊 Current Tools Inventory

### OAuth & Authentication Tools
| Tool Name | Parameters | Status | Priority |
|-----------|------------|--------|----------|
| `get_oauth_url` | `client_id: String` | ✅ Implemented | High |
| `complete_oauth` | `auth_code: String, client_id: String, client_secret: String` | ✅ Implemented | High |
| `auth_status` | None | ✅ Implemented | High |

### Email Management Tools
| Tool Name | Parameters | Status | Priority |
|-----------|------------|--------|----------|
| `list_emails` | `query: Option<String>, max_results: Option<u32>` | ✅ Implemented | High |
| `get_email` | `message_id: String` | ✅ Implemented | High |
| `search_emails` | `query: String, max_results: Option<u32>` | ✅ Implemented | High |
| `analyze_email` | `message_id: String` | ✅ Implemented | Medium |
| `batch_analyze_emails` | `message_ids: Vec<String>` | ✅ Implemented | Medium |
| `create_draft_email` | `to: String, subject: String, body: String, cc: Option<String>, bcc: Option<String>` | ✅ Implemented | High |

### Contacts Tools
| Tool Name | Parameters | Status | Priority |
|-----------|------------|--------|----------|
| `list_contacts` | `max_results: Option<u32>` | ✅ Implemented | Medium |
| `search_contacts` | `query: String, max_results: Option<u32>` | ✅ Implemented | Medium |
| `get_contact` | `person_id: String` | ✅ Implemented | Low |

### Calendar Tools
| Tool Name | Parameters | Status | Priority |
|-----------|------------|--------|----------|
| `list_calendars` | None | ✅ Implemented | Medium |
| `list_events` | `calendar_id: String, max_results: Option<u32>` | ✅ Implemented | Medium |
| `get_event` | `calendar_id: String, event_id: String` | ✅ Implemented | Low |
| `create_event` | `calendar_id: String, title: String, start_time: String, end_time: String, description: Option<String>` | ✅ Implemented | Medium |

### System Tools
| Tool Name | Parameters | Status | Priority |
|-----------|------------|--------|----------|
| `check_connection` | None | ✅ Implemented | Low |

### Prompts
| Prompt Name | Status | Priority |
|-------------|--------|----------|
| `email_drafting_prompt` | ✅ Implemented | Medium |
| `oauth_threading_prompt` | ✅ Implemented | Medium |

## 💻 Implementation Details

### Tool Definition Pattern
```rust
// Current (mcp-attr)
#[tool]
async fn list_emails(&self, query: Option<String>, max_results: Option<u32>) -> McpResult<String> {
    // implementation
}

// Target (rust-mcp-sdk)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListEmailsTool {
    /// Optional search query to filter emails
    pub query: Option<String>,
    /// Maximum number of emails to return (default: 10, max: 100)
    pub max_results: Option<u32>,
}

// In ServerHandler implementation
async fn handle_call_tool_request(&self, request: CallToolRequest, _runtime: &dyn McpServer) -> Result<CallToolResult, CallToolError> {
    match request.tool_name() {
        "list_emails" => {
            let params: ListEmailsTool = serde_json::from_value(request.arguments().clone())?;
            self.handle_list_emails(params).await
        }
        // ... other tools
    }
}
```

### Error Handling Migration
```rust
// Current
Err(self.to_mcp_error("Error message", error_codes::API_ERROR))

// Target
Err(CallToolError::InternalError("Error message".to_string()))
```

### Server Creation Migration
```rust
// Current
let server = GmailServer::new();
serve_stdio(server).await

// Target
let server_handler = GmailServer::new();
let server_info = InitializeResult {
    server_info: Implementation { name: "Gmail MCP Server".to_string(), version: "0.10.0".to_string() },
    capabilities: ServerCapabilities { tools: Some(ServerCapabilitiesTools { list_changed: Some(false) }), /* ... */ },
    protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
    // ...
};
let server = McpServerBuilder::new(server_info).handler(server_handler).build();
let transport = StdioTransport::new();
server.serve(transport).await
```

## Risk Mitigation

### Backup Strategy
- [ ] Create backup branch before starting migration
- [ ] Keep old implementation files as `*_old.rs` during migration
- [ ] Maintain parallel implementation until full validation

### Rollback Plan
- [ ] If migration fails, revert to `mcp-attr` with version pinning
- [ ] Document any blockers encountered for future attempts

### Testing Strategy
- [ ] Implement tests incrementally as tools are migrated
- [ ] Use feature flags if needed to enable/disable new implementation
- [ ] Test against real Gmail API to ensure no regressions

## ✅ Success Criteria

### Functional Requirements
- [ ] All existing tools work identically to current implementation
- [ ] OAuth authentication flow remains unchanged from user perspective
- [ ] MCP inspector connects successfully with latest protocol version
- [ ] No performance regressions
- [ ] All tests pass
- [ ] Documentation is updated and accurate

### Quality Requirements
- [ ] Code coverage remains at current levels or improves
- [ ] Error messages are as helpful as current implementation
- [ ] Logging output is equivalent or better
- [ ] Memory usage is comparable
- [ ] Startup time is not significantly increased

### Protocol Requirements
- [ ] Supports MCP protocol 2025-03-26
- [ ] JSON schema validation works correctly
- [ ] All tool parameters are properly validated
- [ ] Error responses follow MCP protocol format

## Timeline Estimate

**Total: 10-13 days**

- Phase 1 (Research): 1-2 days
- Phase 2 (Infrastructure): 2-3 days
- Phase 3 (Tools): 3-4 days
- Phase 4 (Prompts): 1 day
- Phase 5 (Testing): 2-3 days
- Phase 6 (Cleanup): 1 day

## Dependencies & Blockers

### External Dependencies
- Availability of rust-mcp-sdk documentation and examples
- MCP inspector version compatibility
- Gmail API stability during testing

### Internal Dependencies
- Current server must remain functional during migration
- No breaking changes to OAuth implementation
- Maintain backward compatibility with existing .env configurations

## 🚨 Decision Points & Go/No-Go Criteria

### Phase 1 Decision Point (End of Research)
**Go Criteria:**
- [ ] rust-mcp-sdk API is well-documented and stable
- [ ] Basic server + tool example works
- [ ] Schema generation is straightforward
- [ ] Performance is acceptable

**No-Go Criteria:**
- [ ] API is unstable or poorly documented
- [ ] Requires significant workarounds
- [ ] Performance is significantly worse
- [ ] Missing critical features

### Phase 2 Decision Point (End of Infrastructure)
**Go Criteria:**
- [ ] ServerHandler trait implemented successfully
- [ ] Transport layer works correctly
- [ ] Error handling pattern established
- [ ] OAuth token storage migrated

**No-Go Criteria:**
- [ ] Major architectural issues discovered
- [ ] OAuth integration broken
- [ ] Transport layer problems

## 🔧 Troubleshooting Guide

### Common Issues & Solutions

#### Import/Type Resolution Issues
```rust
// If imports fail, check these patterns:
use rust_mcp_sdk::{
    server::{ServerHandler, McpServer},
    types::{Tool, CallToolRequest, CallToolResult},
    transport::stdio::StdioTransport,
};
```

#### Schema Generation Problems
```rust
// Ensure all tool structs have proper derives:
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MyTool {
    #[schemars(description = "Parameter description")]
    pub param: String,
}
```

#### Runtime Errors
- Check protocol version compatibility
- Verify JSON schema validation
- Ensure async trait implementations are correct
- Check transport initialization order

### Rollback Triggers
- Unable to resolve critical API issues within 2 days of Phase 1
- Major performance regressions discovered
- Essential features missing from rust-mcp-sdk
- OAuth integration cannot be maintained

## 📋 Pre-Migration Checklist

### Environment Setup
- [ ] Backup current working state to branch
- [ ] Document current performance benchmarks
- [ ] Set up test environment with MCP inspector
- [ ] Ensure all current tests pass

### Research Preparation
- [ ] Review rust-mcp-sdk documentation thoroughly
- [ ] Check GitHub issues for known problems
- [ ] Look for example implementations
- [ ] Set up minimal test project structure

## 📝 Notes

- This migration primarily changes the MCP protocol implementation layer
- Core Gmail/Calendar/Contacts API logic should remain largely unchanged
- OAuth token management and storage can be preserved as-is
- The migration provides future-proofing for MCP protocol updates
- Consider this migration as technical debt reduction and future-proofing investment

## 📚 Resources

### Documentation
- [rust-mcp-sdk Documentation](https://docs.rs/rust-mcp-sdk)
- [MCP Protocol Specification 2025-03-26](https://modelcontextprotocol.io/specification/2025-03-26)
- [MCP Official GitHub](https://github.com/modelcontextprotocol)

### Testing Tools
- MCP Inspector: `npm install -g @modelcontextprotocol/inspector`
- Rust MCP SDK Examples: Check rust-mcp-sdk repository
