use log::{debug, error, info, LevelFilter};
use mcp_attr::server::serve_stdio;
use mcp_gmailcal::{setup_logging, GmailServer};
use std::env;

// Main function to start the MCP server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set environment variable to show all log levels
    env::set_var("RUST_LOG", "debug");

    // Initialize logging with maximum verbosity
    let log_file = setup_logging(LevelFilter::Trace, None)?;

    info!("Gmail MCP Server starting...");
    info!("Logs will be saved to {}", log_file);
    debug!("Debug logging enabled");

    // Start the MCP server
    debug!("Creating GmailServer instance");
    let server = GmailServer::new();

    // Run the server
    info!("Starting MCP server with stdio interface");
    let result = serve_stdio(server).await;

    // Log the result
    if let Err(ref e) = result {
        error!("Error running MCP server: {}", e);
    } else {
        info!("MCP server completed successfully");
    }

    debug!("Exiting application");
    result.map_err(|e| e.into())
}
