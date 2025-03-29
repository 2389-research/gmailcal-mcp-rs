use log::{debug, error, info, warn, LevelFilter};
use mcp_attr::server::serve_stdio;
use mcp_gmailcal::{logging::write_direct_to_log, setup_logging, GmailServer};
use std::env;
use std::sync::Arc;
use std::sync::Mutex;

// We'll use this to store the log file path globally
lazy_static::lazy_static! {
    static ref LOG_FILE_PATH: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
}

// Direct log function that doesn't depend on the logging system
fn direct_log(message: &str) {
    if let Ok(path) = LOG_FILE_PATH.lock() {
        if !path.is_empty() {
            let _ = write_direct_to_log(&path, message);
        }
        println!("DIRECT LOG: {}", message);
    }
}

// Main function to start the MCP server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set environment variable to show all log levels
    env::set_var("RUST_LOG", "debug");

    println!("Starting Gmail MCP Server...");

    // Initialize logging with maximum verbosity
    let log_file = setup_logging(LevelFilter::Trace, None)?;

    // Store the log file path for direct logging
    if let Ok(mut path) = LOG_FILE_PATH.lock() {
        *path = log_file.clone();
    }

    // Log startup information
    println!("Initialized logging to: {}", log_file);
    direct_log(&format!("Direct logging test - starting application"));

    info!("Gmail MCP Server starting...");
    info!("Logs will be saved to {}", log_file);
    debug!("Debug logging enabled");

    // Log some system information
    direct_log(&format!(
        "Current directory: {:?}",
        std::env::current_dir().unwrap_or_default()
    ));
    debug!(
        "Environment variables: RUST_LOG={:?}",
        env::var("RUST_LOG").unwrap_or_default()
    );

    // Start the MCP server
    debug!("Creating GmailServer instance");
    let server = GmailServer::new();

    // Log right before starting the server
    info!("Starting MCP server with stdio interface");
    direct_log("About to start MCP server with serve_stdio()");

    // Run the server
    let result = serve_stdio(server).await;

    // Log the result
    if let Err(ref e) = result {
        let error_msg = format!("Error running MCP server: {}", e);
        error!("{}", error_msg);
        direct_log(&error_msg);
    } else {
        info!("MCP server completed successfully");
        direct_log("MCP server completed successfully");
    }

    debug!("Exiting application");
    direct_log("Application exit");

    result.map_err(|e| e.into())
}
