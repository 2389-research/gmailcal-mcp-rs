# Gmail MCP Server - Development Guidelines

## Build/Test/Run Commands
- **Build**: `cargo build`
- **Run (stdio)**: `cargo run` or `cargo run -- --transport stdio`
- **Run (SSE)**: `cargo run -- --transport sse --port 8080 --host 127.0.0.1`
- **Test all**: `cargo test`
- **Test single**: `cargo test test_name`
- **Integration tests**: `cargo test --test integration_tests`
- **Lint**: `cargo clippy`
- **Format**: `cargo fmt`
- **Documentation**: `cargo doc --no-deps --open`
- **Security audit**: `cargo audit`
- **Benchmarking**: `cargo bench`
- **Code coverage**: `cargo tarpaulin`
- **Run with MCP inspector (stdio)**: `npx @modelcontextprotocol/inspector cargo run`
- **Run with MCP inspector (SSE)**: `npx @modelcontextprotocol/inspector http://localhost:8080/sse`

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
