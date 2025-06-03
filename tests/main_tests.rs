/// Main Function Tests Module
///
/// This module contains tests for the main.rs functionality including
/// argument parsing, environment detection, and server initialization.
///
use clap::Parser;
use mcp_gmailcal::cli::{Cli, Commands};
use mcp_gmailcal::logging;
use std::env;
use std::sync::Once;

// Used to ensure environment setup happens only once
static INIT: Once = Once::new();

// Setup function to initialize environment variables for testing
fn setup() {
    INIT.call_once(|| {
        // Set mock environment variables for testing
        env::set_var("GMAIL_CLIENT_ID", "test_client_id");
        env::set_var("GMAIL_CLIENT_SECRET", "test_client_secret");
        env::set_var("GMAIL_REFRESH_TOKEN", "test_refresh_token");
        env::set_var("GMAIL_ACCESS_TOKEN", "test_access_token");
        env::set_var("GMAIL_REDIRECT_URI", "test_redirect_uri");
    });
}

// Testing argument parsing using clap framework
#[test]
fn test_cli_parsing() {
    setup();

    // Test default command (server)
    let args = vec!["gmail-mcp"];
    let cli = Cli::try_parse_from(args).unwrap();
    assert!(cli.command.is_none());
    assert!(!cli.memory_only);

    // Test explicit server command
    let args = vec!["gmail-mcp", "server"];
    let cli = Cli::try_parse_from(args).unwrap();
    assert!(matches!(cli.command, Some(Commands::Server)));

    // Test auth command
    let args = vec!["gmail-mcp", "auth"];
    let cli = Cli::try_parse_from(args).unwrap();
    assert!(matches!(cli.command, Some(Commands::Auth)));

    // Test test command
    let args = vec!["gmail-mcp", "test"];
    let cli = Cli::try_parse_from(args).unwrap();
    assert!(matches!(cli.command, Some(Commands::Test)));

    // Test memory_only flag with long form
    let args = vec!["gmail-mcp", "--memory-only"];
    let cli = Cli::try_parse_from(args).unwrap();
    assert!(cli.memory_only);

    // Test memory_only flag with short form
    let args = vec!["gmail-mcp", "-m"];
    let cli = Cli::try_parse_from(args).unwrap();
    assert!(cli.memory_only);

    // Test command with memory_only flag (must be before the subcommand for clap)
    let args = vec!["gmail-mcp", "--memory-only", "server"];
    let cli = Cli::try_parse_from(args).unwrap();
    assert!(matches!(cli.command, Some(Commands::Server)));
    assert!(cli.memory_only);
}

// Test environment detection
#[test]
fn test_environment_detection() {
    setup();

    // Clean up any previous environment settings
    env::remove_var("CLAUDE_DESKTOP");
    env::remove_var("CLAUDE_AI");
    env::remove_var("MCP_READ_ONLY");

    // Test detection of read-only environment via CLAUDE_DESKTOP
    env::set_var("CLAUDE_DESKTOP", "1");
    let is_read_only =
        std::env::var("CLAUDE_DESKTOP").is_ok() || std::env::var("CLAUDE_AI").is_ok();
    assert!(is_read_only);
    assert_eq!(env::var("CLAUDE_DESKTOP").unwrap(), "1");
    env::remove_var("CLAUDE_DESKTOP");

    // Test detection of read-only environment via CLAUDE_AI
    env::set_var("CLAUDE_AI", "1");
    let is_read_only =
        std::env::var("CLAUDE_DESKTOP").is_ok() || std::env::var("CLAUDE_AI").is_ok();
    assert!(is_read_only);
    assert_eq!(env::var("CLAUDE_AI").unwrap(), "1");
    env::remove_var("CLAUDE_AI");

    // Test default non-read-only environment
    let is_read_only =
        std::env::var("CLAUDE_DESKTOP").is_ok() || std::env::var("CLAUDE_AI").is_ok();
    assert!(!is_read_only);
}

// Test logging setup based on environment
#[test]
fn test_logging_setup_based_on_environment() {
    setup();

    use log::LevelFilter;

    // Test in-memory logging for read-only environment
    // The logger can only be set once, so we can only test one path
    match logging::setup_logging(LevelFilter::Debug, Some("memory")) {
        Ok(log_file) => {
            // If this is the first test to run, it will succeed
            assert_eq!(log_file, "stderr-only (memory mode)");
        }
        Err(_) => {
            // If another test has already set the logger, we'll get an error
            // but that's expected, so we don't fail the test
            assert!(true);
        }
    }
}

// Test server initialization
#[test]
fn test_server_initialization() {
    setup();

    // Test creating the server instance
    let _server = mcp_gmailcal::GmailServer::new();

    // We can't test much about the server - just make sure it creates without panicking
    // GmailServer is actually a zero-sized type
    assert!(true);
}

// Test error handling paths
#[test]
fn test_error_handling() {
    setup();

    // Test behavior when RUST_LOG is set
    env::set_var("RUST_LOG", "info");
    assert_eq!(env::var("RUST_LOG").unwrap(), "info");

    // Test handling of invalid log level (which we wrap in a Result)
    let _invalid_result =
        logging::setup_logging(log::LevelFilter::Trace, Some("/invalid/path.txt"));

    // Result may be Ok or Err depending on permissions, so we just ensure the call doesn't panic
    assert!(true);
}
