use clap::Parser;
use log::{debug, error, LevelFilter};
use mcp_attr::server::serve_stdio;
use mcp_gmailcal::{
    cli::{Cli, Commands},
    oauth, setup_logging, GmailServer, SseServer,
};
use std::{env, net::SocketAddr};

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
        eprintln!("Running in read-only mode with in-memory logging");
    }

    // Determine which command to run
    match cli.command {
        Some(Commands::Auth) => {
            eprintln!("Starting OAuth authentication flow...");
            if let Err(e) = oauth::run_oauth_flow().await {
                eprintln!("Authentication failed: {}", e);
                std::process::exit(1);
            }
            return Ok(());
        }
        Some(Commands::Test) => {
            eprintln!("Testing Gmail credentials...");
            match oauth::test_credentials().await {
                Ok(result) => {
                    eprintln!("{}\n", result);
                    eprintln!("✅ Credentials are valid and working!");
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

    debug!("Gmail MCP Server starting...");
    debug!("Logs will be saved to {}", log_file);
    debug!("Debug logging enabled");

    // Choose transport based on CLI argument
    match cli.transport.as_str() {
        "stdio" => {
            // Start the MCP server with stdio transport
            debug!("Creating GmailServer instance for stdio transport");
            let server = GmailServer::new();

            // Run the server
            debug!("Starting MCP server with stdio interface");
            let result = serve_stdio(server).await;

            // Log the result
            if let Err(ref e) = result {
                error!("Error running MCP server: {}", e);
            } else {
                debug!("MCP server completed successfully");
            }

            debug!("Exiting application");
            result.map_err(|e| e.into())
        }
        "sse" => {
            // Start the MCP server with SSE transport
            debug!("Creating SseServer instance for SSE transport");
            let server = SseServer::new();

            // Build the socket address
            let addr: SocketAddr = format!("{}:{}", cli.host, cli.port).parse()?;

            debug!("Starting MCP server with SSE transport on {}", addr);

            // Run the SSE server
            server.serve(addr).await?;

            Ok(())
        }
        _ => {
            error!("Unknown transport: {}", cli.transport);
            eprintln!(
                "Unknown transport: {}. Use 'stdio' or 'sse'.",
                cli.transport
            );
            std::process::exit(1);
        }
    }
}
