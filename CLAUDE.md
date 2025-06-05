# Gmail MCP Server - Development Guidelines

## Gmail MCP Server (rust-mcp-sdk)
- **Build**: `cargo build --bin mcp-gmailcal`
- **Run (stdio)**: `cargo run --bin mcp-gmailcal`
- **Test with MCP inspector**: `npx @modelcontextprotocol/inspector cargo run --bin mcp-gmailcal`
- **Tools Available**: 18 tools (Complete Gmail, Calendar, and Contacts functionality)
  - OAuth (3): `get_oauth_url`, `complete_oauth`, `auth_status`
  - Gmail (8): `list_emails`, `get_email`, `search_emails`, `list_labels`, `check_connection`, `analyze_email`, `batch_analyze_emails`, `create_draft_email`
  - People/Contacts (3): `list_contacts`, `search_contacts`, `get_contact`
  - Calendar (4): `list_calendars`, `list_events`, `get_event`, `create_event`

## Standard Commands
- **Test all**: `cargo test`
- **Test single**: `cargo test test_name`
- **Integration tests**: `cargo test --test integration_tests`
- **Lint**: `cargo clippy`
- **Format**: `cargo fmt`
- **Documentation**: `cargo doc --no-deps --open`
- **Security audit**: `cargo audit`
- **Benchmarking**: `cargo bench`
- **Code coverage**: `cargo tarpaulin`

## OAuth Authentication
The server supports OAuth authentication through MCP tools:
- **Check auth status**: Use `auth_status` tool
- **Start OAuth flow**: Use `get_oauth_url` tool with your Google Client ID
- **Complete OAuth**: Use `complete_oauth` tool with auth code and client secret

## Code Style Guidelines
- **Formatting**: Follow Rust standard formatting (rustfmt)
- **Error handling**: Use `thiserror` for custom errors, return Result types
- **Logging**: Use `log` crate with appropriate levels (debug, info, error)
- **Naming**:
  - Use snake_case for functions, variables, modules
  - Use CamelCase for types, traits, enums
- **File organization**: Group related functionality in modules
- **Comments**: Use doc comments `///` for public API, regular comments `//` for implementation details
- **Types**: Use strong typing, leverage Rust's type system
- **Async**: Use `tokio` for async operations, with proper error propagation
- **JSON handling**: Use `serde` for serialization/deserialization
