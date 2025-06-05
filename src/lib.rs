pub mod auth;
pub mod config;
pub mod token_cache;
// ===== Module Declarations =====

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
// Core functionality
pub mod errors;
pub mod logging;

// API clients
pub mod calendar_api;
pub mod gmail_api;
pub mod people_api;

// Server implementation
pub mod cli;
pub mod oauth;
pub mod prompts;
pub mod sse_server;
pub mod sse_transport;

// rust-mcp-sdk server implementation
pub mod calendar_tools;
pub mod gmail_tools;
pub mod oauth_tools;
pub mod people_tools;
pub mod tools;

// ===== Re-exports =====

// Error handling and results
pub use crate::errors::{
    error_codes, CalendarApiError, CalendarResult, ConfigError, GmailApiError, GmailResult,
    PeopleApiError, PeopleResult,
};

// Configuration and constants
pub use crate::config::{get_token_expiry_seconds, Config, GMAIL_API_BASE_URL, OAUTH_TOKEN_URL};

// Logging setup
pub use crate::logging::setup_logging;

// Authentication
pub use crate::auth::TokenManager;
pub use crate::token_cache::{CachedToken, TokenCache, TokenCacheConfig};

// Gmail API types
pub use crate::gmail_api::{DraftEmail, EmailMessage, GmailService};

// People API types
pub use crate::people_api::{
    Contact, ContactList, EmailAddress, Organization, PeopleClient, PersonName, PhoneNumber, Photo,
};

// Calendar API types
pub use crate::calendar_api::{
    Attendee, CalendarClient, CalendarEvent, CalendarInfo, CalendarList, ConferenceData,
    ConferenceSolution, EntryPoint, EventOrganizer,
};

// Prompts
pub use crate::prompts::*;

// Server implementation
pub use crate::sse_server::SseServer;
