use chrono::Local;
use log::LevelFilter;
use simplelog::{self, CombinedLogger, TermLogger, WriteLogger};
use std::fs::OpenOptions;
use std::io::Write;

/// Sets up logging to file and stderr
///
/// # Arguments
///
/// * `log_level` - The level of log messages to capture
/// * `log_file` - Optional path to log file. If None, creates a timestamped file
///
/// # Returns
///
/// Sets up the logging system
///
/// # Arguments
///
/// * `log_level` - The level of logging to use
/// * `log_file` - Optional log file name or "memory" to use in-memory logging
///
/// # Returns
///
/// The path to the log file or a description of the logging destination
pub fn setup_logging(log_level: LevelFilter, log_file: Option<&str>) -> std::io::Result<String> {
    // Use the default config for simplicity - explicitly use simplelog::Config to avoid ambiguity
    let log_config = simplelog::Config::default();

    // Check if we should use memory-only logging
    if log_file == Some("memory") {
        // For memory-only logging, just use stderr
        TermLogger::init(
            log_level,
            log_config,
            simplelog::TerminalMode::Stderr,
            simplelog::ColorChoice::Auto,
        )
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        log::info!("Logging initialized to stderr only (memory mode)");
        log::debug!("Debug logging enabled");

        return Ok(String::from("stderr-only (memory mode)"));
    }

    // Create a timestamp for the log file
    let timestamp = Local::now().format("%Y%m%d_%H").to_string();

    // Determine log file path
    let log_path = match log_file {
        Some(path) => path.to_string(),
        None => format!("gmail_mcp_{}.log", timestamp),
    };

    // Create the log file with append mode and write header in one operation
    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    writeln!(
        log_file,
        "====== GMAIL MCP SERVER LOG - Started at {} ======",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    )?;

    // Setup loggers to write to both file and stderr
    CombinedLogger::init(vec![
        // File logger
        WriteLogger::new(log_level, log_config.clone(), log_file),
        // Terminal logger for stderr
        TermLogger::new(
            log_level,
            log_config,
            simplelog::TerminalMode::Stderr,
            simplelog::ColorChoice::Auto,
        ),
    ])
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    log::info!("Logging initialized to file: {} and stderr", log_path);
    log::debug!("Debug logging enabled");

    Ok(log_path)
}
