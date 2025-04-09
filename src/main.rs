use clap::Parser;
use log::{debug, error, info, LevelFilter};
use mcp_attr::server::serve_stdio;
use mcp_gmailcal::{cli::{Cli, Commands}, oauth, setup_logging, GmailServer};
use std::env;

// Main function to start the MCP server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set environment variable to show all log levels
    env::set_var("RUST_LOG", "debug");

    // Parse command line arguments
    let cli = Cli::parse();

    // Check if we're in a read-only environment
    let is_read_only = std::env::var("CLAUDE_DESKTOP").is_ok()
        || std::env::var("CLAUDE_AI").is_ok()
        || cli.memory_only;
    if is_read_only {
        // Set a marker environment variable for read-only mode
        env::set_var("MCP_READ_ONLY", "1");
        println!("Running in read-only mode with in-memory logging");
    }

    // Determine which command to run
    match cli.command {
        Some(Commands::Auth) => {
            println!("Starting OAuth authentication flow...");
            if let Err(e) = oauth::run_oauth_flow().await {
                eprintln!("Authentication failed: {}", e);
                std::process::exit(1);
            }
            return Ok(());
        }
        Some(Commands::Test) => {
            println!("Testing Gmail credentials...");
            match oauth::test_credentials().await {
                Ok(result) => {
                    println!("{}\n", result);
                    println!("✅ Credentials are valid and working!");
                }
                Err(e) => {
                    eprintln!("❌ Credential test failed: {}", e);
                    eprintln!("\nRun 'cargo run -- auth' to refresh your credentials.");
                    std::process::exit(1);
                }
            }
            return Ok(());
        }
        Some(Commands::Server) | None => {
            // Continue with server startup
        }
    }

    // Initialize logging based on environment
    let log_file = if is_read_only {
        // Use in-memory logging for read-only environments
        setup_logging(LevelFilter::Debug, Some("memory"))?
    } else {
        // Use file logging for normal operation
        setup_logging(LevelFilter::Trace, None)?
    };

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
