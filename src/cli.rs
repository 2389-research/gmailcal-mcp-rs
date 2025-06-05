use clap::{Parser, Subcommand};

#[derive(Parser, Debug, PartialEq)]
#[clap(name = "Gmail MCP Server")]
#[clap(author = "Gmail MCP Contributors")]
#[clap(version = "0.2.0")]
#[clap(about = "MCP server for Gmail access", long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Commands>,

    /// Force use of stderr-only logging (no file logging)
    #[clap(long, short, action)]
    pub memory_only: bool,

    /// Transport to use for the MCP server (stdio or sse)
    #[clap(long, default_value = "stdio")]
    pub transport: String,

    /// Port to listen on for SSE transport
    #[clap(long, default_value = "8080")]
    pub port: u16,

    /// Host to bind to for SSE transport
    #[clap(long, default_value = "127.0.0.1")]
    pub host: String,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Run the MCP server (default if no command specified)
    #[clap(name = "server")]
    Server,

    /// Run the OAuth authentication flow to get new credentials
    #[clap(name = "auth")]
    Auth,

    /// Test the current credentials
    #[clap(name = "test")]
    Test,
}
