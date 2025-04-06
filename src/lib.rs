/**
 * Gmail MCP Server Implementation
 *
 * This crate provides an MCP (Model Completion Protocol) server for Gmail,
 * allowing Claude to read emails from a Gmail account, search for contacts,
 * and manage calendar events.
 *
 * # Features
 *
 * - List emails from inbox
 * - Search emails using Gmail search queries
 * - Get details for a specific email
 * - List email labels
 * - Search for contacts
 * - List calendar events
 * - Create calendar events
 * - Check connection status
 *
 * # Testing
 *
 * The crate includes unit tests for internal functions and integration tests
 * for testing the MCP commands. Future improvements could include more
 * sophisticated mocking of the API endpoints and more comprehensive tests.
 */

// ===== Module Declarations =====

// Core functionality
pub mod errors;
pub mod config;
pub mod logging;
pub mod utils;
pub mod auth;

// API clients
pub mod gmail_api;
pub mod people_api;
pub mod calendar_api;

// Server implementation
pub mod server;
pub mod prompts;
pub mod oauth;

// ===== Re-exports =====

// Error handling and results
pub use crate::errors::{
    ConfigError, GmailApiError, PeopleApiError, CalendarApiError,
    GmailResult, PeopleResult, CalendarResult, error_codes
};

// Configuration and constants
pub use crate::config::{Config, GMAIL_API_BASE_URL, OAUTH_TOKEN_URL, get_token_expiry_seconds};

// Logging setup
pub use crate::logging::setup_logging;

// Authentication
pub use crate::auth::TokenManager;

// Gmail API types
pub use crate::gmail_api::{EmailMessage, DraftEmail, GmailService};

// People API types
pub use crate::people_api::{
    Contact, EmailAddress, Organization, PeopleClient, 
    PersonName, PhoneNumber, Photo, ContactList
};

// Calendar API types
pub use crate::calendar_api::{
    CalendarClient, CalendarEvent, CalendarList, CalendarInfo,
    Attendee, EventOrganizer, ConferenceData, ConferenceSolution, EntryPoint
};

// Utils and prompts
pub use crate::prompts::*;
pub use crate::utils::{
    parse_max_results, decode_base64, encode_base64_url_safe, 
    to_mcp_error, map_gmail_error, error_codes as utils_error_codes
};

// Server implementation
pub use crate::server::GmailServer;