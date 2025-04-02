# MCP-Attr Comprehensive Guide

This document outlines a robust plan for implementing the `mcp-attr` crate, along with a practical how-to guide for creating an MCP server. It covers everything from the project’s overall structure and macro usage to installation, basic server setup, testing, and best practices.

## Table of Contents

1. [Project Layout](#project-layout)
2. [Installation](#installation)
3. [Overview of Attribute Macros](#overview-of-attribute-macros)
4. [Core Library (Traits and Runtime)](#core-library-traits-and-runtime)
5. [Schema & Code Generation](#schema--code-generation)
6. [Basic Server Usage](#basic-server-usage)
    - [Server Structure](#server-structure)
    - [Prompts](#prompts)
    - [Resources](#resources)
    - [Tools](#tools)
    - [Error Handling](#error-handling)
    - [State Management](#state-management)
7. [Advanced Features](#advanced-features)
8. [Testing Strategy](#testing-strategy)
9. [Best Practices](#best-practices)
10. [Future Enhancements](#future-enhancements)
11. [Troubleshooting](#troubleshooting)
12. [Step-by-Step Summary](#step-by-step-summary)

---

## Project Layout

A recommended structure for your `mcp-attr` workspace includes two primary crates and an optional code generation step:

1. **Macro Crate: `mcp-attr-macros`**

    - Houses all procedural macros (`mcp_server`, `prompt`, `resource`, `tool`, `route`).
    - Uses [`syn`](https://docs.rs/syn/latest/syn/) to parse Rust code, transform the AST, and generate the resulting implementations.
    - Likely organized with separate modules for each macro (`prompts.rs`, `resources.rs`, `tools.rs`, etc.) plus shared utilities (`utils.rs`).
    - A root `lib.rs` providing the `#[proc_macro_attribute]` definitions.

2. **Library Crate: `mcp-attr`**

    - Exposes the main user-facing traits (`McpServer`, `McpClient`, etc.), data structures, error types, and runtime logic.
    - Typical structure:
        - `server/` for the `McpServer` trait, request contexts, and utility methods (e.g., `serve_stdio`).
        - `client/` for `McpClient` logic, builders, and typed calls.
        - `schema/` for protocol data structures.
        - A `tests/` folder for both normal and compile-fail tests.

3. **Code Generation (Optional)**
    - A small `codegen` crate or build script can auto-generate internal schemas or conversions (`transitivity.rs`, `schema.rs`) based on external specs.
    - This helps reduce code duplication and ensure consistent protocol definitions.

---

## Installation

In your project’s `Cargo.toml`, add:

```toml
[dependencies]
mcp-attr = "0.0.6"
tokio = "1.43.0"
serde = { version = "1.0", features = ["derive"] }
schemars = "0.8"
```

Use any other dependencies as needed, such as `syn` and `quote` if you’re working on macros directly in a separate crate.

---

## Overview of Attribute Macros

`mcp-attr` uses attribute macros to annotate server methods and generate MCP-compatible code. Below are the primary macros:

- **`#[mcp_server]`**  
  Placed on an `impl McpServer for SomeType` block. It transforms the methods inside that block into the MCP server’s routes.

- **`#[prompt]`**  
  Designates an async function that serves static or dynamically generated text for the AI.

- **`#[resource]`**  
  Declares functions returning data (e.g., strings, binary buffers) accessed via URI templates.

- **`#[tool]`**  
  Exposes a method that can be called with JSON-serializable arguments, returning structured data or text.

- **`#[route]`** (internal)  
  Provides low-level plumbing to turn annotated methods into routing tables. Typically used behind the scenes by the other macros.

Each of these macros inspects method signatures, parameters, doc comments, and optional meta-attributes (`#[arg("name")]`, `mime_type`, `uri_template`, etc.) to generate the final server logic.

---

## Core Library (Traits and Runtime)

1. **`McpServer` Trait**

    - Defines core server methods:
        - `prompts_list`, `prompts_get`
        - `resources_list`, `resources_read`
        - `tools_list`, `tools_call`
        - Additional optional overrides like `server_info` or `completion_complete`.
    - The `#[mcp_server]` macro implements these automatically when it processes your method annotations.

2. **`McpClient`**

    - Provides a typed client interface. It wraps a JSON-RPC session and calls the server’s methods, returning strongly typed responses.
    - Built via `McpClientBuilder`.

3. **`RequestContext`**

    - Passed to server methods that need callbacks into the client (e.g., `roots_list` or logging).
    - Avoid storing it if you only need ephemeral calls.

4. **`serve_stdio`**

    - A convenience function for running the server over standard input/output using `jsoncall::Session` under the hood.
    - You can create your own transport if needed (WebSockets, SSE, etc.).

5. **Error Handling**
    - Includes an `mcp_attr::Error` type and macros like `bail!` and `bail_public!`.
    - Distinguishes private (internal) errors from public ones surfaced to the AI client.

---

## Schema & Code Generation

- **Schemars Integration**  
  Tools can generate JSON schemas for argument types, helping AI clients understand each parameter.  
  The `#[tool]` macro uses Schemars under the hood.

- **Build Scripts**  
  If you have an external spec or format, a `codegen` or build script can generate additional files (e.g., `schema.rs`, `transitivity.rs`) for consistent type definitions.

---

## Basic Server Usage

### Server Structure

Below is a minimal example showing how you might define and run an MCP server:

```rust
/// main.rs
use std::sync::Mutex;
use mcp_attr::server::{mcp_server, McpServer, serve_stdio};
use mcp_attr::Result;

struct MyServer(Mutex<ServerState>);

struct ServerState {
    counter: u32,
}

#[mcp_server]
impl McpServer for MyServer {
    // Place your #[prompt], #[resource], #[tool] methods here
}

#[tokio::main]
async fn main() -> Result<()> {
    let server = MyServer(Mutex::new(ServerState { counter: 0 }));
    serve_stdio(server).await?;
    Ok(())
}
```

### Prompts

Use `#[prompt]` to provide static or semi-static textual content:

```rust
#[mcp_server]
impl McpServer for MyServer {
    /// This doc comment is passed to the AI
    #[prompt]
    async fn example_prompt(&self) -> Result<&str> {
        Ok("This is a prompt from the server")
    }

    #[prompt("custom_prompt_name")]
    async fn prompt_with_args(&self,
        /// An argument for user identification
        user_name: String
    ) -> Result<String> {
        Ok(format!("Hello, {}!", user_name))
    }
}
```

### Resources

Resources behave like files or URLs, with optional URI templates:

```rust
#[mcp_server]
impl McpServer for MyServer {
    #[resource("app://files/{filename}.txt")]
    async fn text_file(&self, filename: String) -> Result<String> {
        Ok(format!("Content of {}.txt", filename))
    }

    // A resource returning binary data (image)
    #[resource("app://images/{name}.{format}", mime_type = "image/png")]
    async fn image_file(&self, name: String, format: String) -> Result<Vec<u8>> {
        Ok(vec![0, 1, 2, 3]) // Example placeholder
    }

    // A fallback resource
    #[resource]
    async fn fallback_resource(&self, url: String) -> Result<String> {
        Ok(format!("Resource at {}", url))
    }
}
```

### Tools

`#[tool]` allows JSON-RPC calls with complex arguments, returning structured data:

```rust
#[mcp_server]
impl McpServer for MyServer {
    #[tool]
    async fn add_numbers(&self,
        a: i32,
        b: i32
    ) -> Result<String> {
        Ok(format!("Result: {}", a + b))
    }

    #[tool("renamed_tool")]
    async fn complex_tool(&self,
        /// Example optional param
        optional_arg: Option<String>,

        #[arg("arg_values")]
        values: Vec<String>
    ) -> Result<String> {
        let mut state = self.0.lock().unwrap();
        state.counter += 1;

        let opt = optional_arg.unwrap_or_else(|| "default".to_string());
        let count = state.counter;

        Ok(format!("Processed {} items with {}, count: {}",
            values.len(), opt, count))
    }
}
```

### Error Handling

Use the macros `bail!` (private) and `bail_public!` (client-facing):

```rust
use mcp_attr::{bail, bail_public, ErrorCode, Result};

#[mcp_server]
impl McpServer for MyServer {
    #[tool]
    async fn error_demo(&self, input: String) -> Result<String> {
        if input.is_empty() {
            bail_public!(ErrorCode::INVALID_PARAMS, "Input cannot be empty");
        }

        if input.len() > 100 {
            bail!("Input too long: {}", input.len());
        }

        let number = input.parse::<i32>()?;
        Ok(format!("Parsed: {}", number))
    }
}
```

### State Management

If methods share data, ensure safe concurrency:

```rust
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

struct MyServer {
    state: Arc<Mutex<ServerState>>,
}

struct ServerState {
    data: HashMap<String, String>,
}

#[mcp_server]
impl McpServer for MyServer {
    #[tool]
    async fn store_data(&self, key: String, value: String) -> Result<String> {
        let mut state = self.state.lock().unwrap();
        state.data.insert(key.clone(), value);
        Ok(format!("Stored data with key: {}", key))
    }

    #[tool]
    async fn get_data(&self, key: String) -> Result<String> {
        let state = self.state.lock().unwrap();
        match state.data.get(&key) {
            Some(value) => Ok(value.clone()),
            None => bail_public!(ErrorCode::NOT_FOUND, "Key not found: {}", key),
        }
    }
}
```

---

## Advanced Features

- **Client Feature Calls**: Inside a server method, you can access MCP client features via `RequestContext`:

    ```rust
    use mcp_attr::server::{RequestContext, mcp_server, McpServer};

    #[mcp_server]
    impl McpServer for MyServer {
        #[tool]
        async fn list_roots(&self, context: &RequestContext) -> Result<String> {
            let roots = context.roots_list().await?;
            Ok(format!("Available roots: {:?}", roots))
        }
    }
    ```

- **Custom Return Types**: Implement `From<T>` for the appropriate return type (e.g., `CallToolResult`) if you need special serialization:

    ```rust
    use mcp_attr::schema::{CallToolResult};
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize)]
    struct CustomResponse {
        status: String,
        data: Vec<i32>,
    }

    impl From<CustomResponse> for CallToolResult {
        fn from(resp: CustomResponse) -> Self {
            let json = serde_json::to_string(&resp).unwrap();
            CallToolResult::from(json)
        }
    }

    #[mcp_server]
    impl McpServer for MyServer {
        #[tool]
        async fn custom_response(&self) -> Result<CustomResponse> {
            Ok(CustomResponse {
                status: "success".to_string(),
                data: vec![1, 2, 3],
            })
        }
    }
    ```

---

## Testing Strategy

1. **Unit & Integration Tests**

    - Test individual macros, AST parsing, and expansions as separate units.
    - Write integration tests that call the server in-process via `McpClient`.

2. **Compile-Fail Tests**

    - Ensure incorrect usage of macros fails with helpful compiler messages.
    - Use a dedicated `tests/compile_fail` folder with `.rs` files that are expected to fail.

3. **Example**:

    ```rust
    /// tests/integration_test.rs
    use mcp_attr::client::McpClient;
    use mcp_attr::schema::{GetPromptRequestParams, CallToolRequestParams};
    use mcp_attr::Result;

    #[tokio::test]
    async fn test_my_server() -> Result<()> {
        let server = MyServer(Mutex::new(ServerState { counter: 0 }));
        let client = McpClient::with_server(server).await?;

        // Test prompt
        let prompt = client
            .prompts_get(GetPromptRequestParams::new("example_prompt"))
            .await?;
        assert_eq!(prompt.text, "This is a prompt from the server");

        // Test tool
        let params = serde_json::json!({ "a": 5, "b": 7 });
        let result = client
            .tools_call(CallToolRequestParams::new("add_numbers", params))
            .await?;
        assert_eq!(result.result, "Result: 12");

        Ok(())
    }
    ```

---

## Best Practices

1. **Documentation**: Thoroughly document each macro-annotated method and parameter.
2. **Error Handling**: Use `bail_public!` for client-facing messages, `bail!` for private ones.
3. **Type Safety**: Prefer typed arguments over raw JSON.
4. **Concurrency**: Wrap shared data structures with thread-safe containers (`Mutex`, `RwLock`, etc.).
5. **Async**: Keep methods async for concurrency.
6. **Testing**: Use both integration and compile-fail tests.
7. **Security**: Avoid exposing sensitive data in public error messages.

---

## Future Enhancements

1. **Alternate Transports**: WebSockets, SSE, or custom streams for real-time interactions.
2. **Extended Error Handling**: Additional macros or logic for custom error codes.
3. **Workflow Integration**: Potential cargo subcommands or build scripts to run code generation.

---

## Troubleshooting

- **Method Not Found**: Ensure the method name or custom macro name matches the client’s request.
- **Type Conversion Errors**: Check that all parameters and return types implement `Serialize`/`Deserialize` when necessary.
- **Locking Issues**: Confirm your concurrency logic.
- **Protocol Errors**: Verify you’re using a compatible MCP version.
- **Test Failures**: Run in debug mode for detailed logging.

---

## Step-by-Step Summary

1. **Set up Crates**: Create two crates if necessary—one for macros, one for the main library (or keep them unified if simpler).
2. **Implement Macros**: In `mcp-attr-macros`, define each procedural macro, using `syn` to parse function signatures and generate AST expansions.
3. **Core Library**: In `mcp-attr`, implement `McpServer` trait, `McpClient`, and shared schema structures.
4. **Add Codegen (Optional)**: Write a build script or separate crate to generate additional schema files.
5. **Write Server**: Create your server struct, annotate it with `#[mcp_server]`, and fill it with `#[prompt]`, `#[resource]`, and `#[tool]` methods.
6. **Test Thoroughly**: Use normal integration tests plus compile-fail tests.
7. **Iterate and Publish**: Document methods, ensure robust error handling, and ship your crates.

With this plan and guide combined, you can implement, organize, and use the `mcp-attr` framework confidently—building robust MCP-compatible servers and cleanly exposing prompts, resources, and tools to client applications.
