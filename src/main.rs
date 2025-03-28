use std::sync::Mutex;

use mcp_attr::server::{mcp_server, McpServer, serve_stdio};
use mcp_attr::Result;

#[tokio::main]
async fn main() -> Result<()> {
    serve_stdio(ExampleServer(Mutex::new(ServerData { count: 0 }))).await?;
    Ok(())
}

struct ExampleServer(Mutex<ServerData>);

struct ServerData {
  /// Server state
  count: u32,
}

#[mcp_server]
impl McpServer for ExampleServer {
    /// Description sent to MCP client
    #[prompt]
    async fn example_prompt(&self) -> Result<&str> {
        Ok("Hello!")
    }

    #[resource("my_app://files/{name}.txt")]
    async fn read_file(&self, name: String) -> Result<String> {
        Ok(format!("Content of {name}.txt"))
    }

    #[tool]
    async fn add_count(&self, message: String) -> Result<String> {
        let mut state = self.0.lock().unwrap();
        state.count += 1;
        Ok(format!("Echo: {message} {}", state.count))
    }
}
