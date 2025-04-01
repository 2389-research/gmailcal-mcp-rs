# Gmail MCP Server - Test Suite Documentation

This document outlines the test suite for the Gmail MCP Server project, including implemented tests and potential future improvements.

## Test Structure

The test suite is organized into multiple test files:

1. **integration_tests.rs** - Basic integration tests for the MCP server
2. **unit_tests.rs** - Unit tests for individual components
3. **mock_client.rs** - Mock implementation of the Gmail API client
4. **token_gmail_tests.rs** - Tests for the token management functionality
5. **error_tests.rs** - Tests for error handling
6. **calendar_api_tests.rs** - Tests for Google Calendar API
7. **gmail_message_tests.rs** - Tests for email message parsing
8. **gmail_draft_tests.rs** - Tests for draft email functionality
9. **people_api_tests.rs** - Tests for Google People API (contacts)
10. **server_tests.rs** - Tests for MCP command handling

## Implemented Tests

### Integration Tests
- [x] **Server Creation** (`test_server_creation`)
  - Verifies the server can be created with test environment variables
- [x] **Configuration** (`test_configuration`)
  - Verifies the environment variables are correctly loaded
- [x] **Gmail Prompt** (`test_gmail_prompt`)
  - Tests the Gmail prompt functionality

### Unit Tests with Mock Client
- [x] **List Messages** (`test_mock_client_list_messages`)
  - Tests listing all messages
  - Tests filtering messages with a query
- [x] **Get Message** (`test_mock_client_get_message`)
  - Tests retrieving a specific message by ID
  - Tests error handling for non-existent messages
- [x] **List Labels** (`test_mock_client_list_labels`)
  - Tests retrieving all Gmail labels
- [x] **Get Profile** (`test_mock_client_get_profile`)
  - Tests retrieving the user profile
- [x] **Create Draft** (`test_mock_client_create_draft`)
  - Tests creating a draft email
- [x] **Email Conversion** (`test_email_conversion`)
  - Tests conversion between TestEmail and JSON

### Token Management Tests
- [x] **Token Manager Creation** (`test_token_manager_creation`)
  - Tests creating a new token manager
- [x] **Token Manager with Access Token** (`test_token_manager_with_access_token`)
  - Tests creating a token manager with an initial access token

### Error Tests
- [x] **Config Error** (`test_config_error`)
  - Tests error creation and formatting for configuration errors
- [x] **Gmail API Error** (`test_gmail_api_error`)
  - Tests error creation and formatting for Gmail API errors

### Calendar API Tests
- [x] **Calendar Client Creation** (`test_calendar_client_creation`)
  - Tests creating a Calendar API client
- [x] **List Calendars** (`test_list_calendars`)
  - Tests listing available calendars
- [x] **Get Event** (`test_get_event`)
  - Tests retrieving event details
- [x] **Create Event** (`test_create_event`)
  - Tests creating new events
- [x] **Date Handling** (`test_date_handling`)
  - Tests date/time handling

### Message Parsing Tests
- [x] **Parse Simple Message** (`test_parse_simple_message`)
  - Tests parsing plain text messages
- [x] **Parse Multipart Message** (`test_parse_multipart_message`)
  - Tests parsing multipart messages
- [x] **Parse Malformed Message** (`test_parse_malformed_message`)
  - Tests handling of malformed messages

### Draft Email Tests
- [x] **Draft Creation** (`test_draft_creation`)
  - Tests creating a draft with minimal fields
- [x] **Draft to API Format** (`test_draft_to_api_format`)
  - Tests conversion to API format
- [x] **Draft Validation** (`test_draft_validation`)
  - Tests draft validation rules

### People API Tests
- [x] **People Client Creation** (`test_people_client_creation`)
  - Tests creating a People API client
- [x] **Search Contacts** (`test_search_contacts`)
  - Tests searching for contacts
- [x] **Get Contact** (`test_get_contact`)
  - Tests retrieving contact details
- [x] **Contact Parsing** (`test_contact_parsing`)
  - Tests parsing contact data

### MCP Command Tests
- [x] **Server Creation** (`test_server_creation`)
  - Tests creating an MCP server instance
- [x] **Command Parsing** (`test_command_parsing`)
  - Tests parsing MCP commands
- [x] **Response Formatting** (`test_response_formatting`)
  - Tests formatting MCP responses

## Mock Implementation

The mock implementation provides:

1. **MockGmailClient** - A mock Gmail API client that:
   - Returns predefined responses for API calls
   - Simulates filtering and searching
   - Handles error conditions
   - Can be customized for specific test scenarios

2. **TestEmail** - A test email structure that:
   - Provides common email attributes
   - Has methods to convert to JSON
   - Can be easily created with factory methods

3. **Utility Functions**:
   - `create_test_emails()` - Creates a set of sample emails
   - `create_test_labels()` - Creates a set of sample labels
   - `labels_to_json()` - Converts labels to JSON format
   - `create_mock_client()` - Creates a pre-configured mock client

## Future Test Improvements

The following improvements are still needed for the test suite:

1. **Complete Mocking Implementation**
   - [ ] Implement proper HTTP client mocking with mockall
   - [ ] Create comprehensive fixture data for all API tests
   - [ ] Build integration test helpers for common scenarios

2. **Token Management Improvements**
   - [ ] Implement full token refresh test with mock HTTP client
   - [ ] Test concurrent refresh with simulated race conditions
   - [ ] Test persistent token storage

3. **API Client Tests**
   - [ ] Add tests for real API client behavior with mocked responses
   - [ ] Test error handling and retries
   - [ ] Test rate limiting and backoff strategies

4. **Edge Case Testing**
   - [ ] Test with pathological inputs (very large or malformed)
   - [ ] Test with unusual character sets and encodings
   - [ ] Test error recovery scenarios

5. **Performance Testing**
   - [ ] Test pagination with large result sets
   - [ ] Test concurrent request handling
   - [ ] Test memory usage under load

6. **Transport Layer Testing**
   - [ ] Test the STDIN/STDOUT transport layer
   - [ ] Test message framing and serialization
   - [ ] Test graceful handling of connection issues

7. **Automated Testing Improvements**
   - [ ] Set up continuous integration with GitHub Actions
   - [ ] Add code coverage reporting
   - [ ] Implement property-based testing
   - [ ] Add benchmarking to track performance

## Running Tests

To run all tests:
```bash
cargo test
```

To run a specific test file:
```bash
cargo test --test file_name
```

For example:
```bash
cargo test --test token_gmail_tests
```

## Test Coverage Verification

To check test coverage, you can use tools like:

- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin)
- [grcov](https://github.com/mozilla/grcov)

Example:
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Xml
```

This will generate a coverage report showing which parts of the code need additional test coverage.