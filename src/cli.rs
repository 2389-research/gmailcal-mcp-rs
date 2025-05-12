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
