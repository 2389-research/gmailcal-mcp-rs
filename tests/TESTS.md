# Gmail MCP Server - Test Suite Documentation

This document outlines the test suite for the Gmail MCP Server project, including implemented tests and potential future improvements.

## Test Structure

The test suite is organized into multiple test files:

1. **integration_tests.rs** - Basic integration tests for the MCP server
2. **unit_tests.rs** - Unit tests for individual components
3. **mock_client.rs** - Mock implementation of the Gmail API client

## Implemented Tests

### Integration Tests
- [x] **Server Creation** (`test_server_creation`)
  - Verifies the server can be created with test environment variables
- [x] **Configuration** (`test_configuration`)
  - Verifies the environment variables are correctly loaded

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

The following improvements can be made to the test suite:

1. **Expanded MCP Command Testing**
   - [ ] Test all MCP commands with the mock client
   - [ ] Test request parameter validation
   - [ ] Test response format and structure

2. **Error Handling**
   - [ ] Test OAuth error handling
   - [ ] Test network error handling
   - [ ] Test rate limiting error handling

3. **Authentication Testing**
   - [ ] Test token refresh mechanism
   - [ ] Test token expiration handling
   - [ ] Test invalid credentials

4. **Edge Cases**
   - [ ] Test with very large emails
   - [ ] Test with unusual character encodings
   - [ ] Test with various email formats (HTML, plain text, mixed)
   - [ ] Test with attachments of different types

5. **Performance Testing**
   - [ ] Test handling of large result sets
   - [ ] Test response time for different operations
   - [ ] Test memory usage

## Running Tests

To run all tests:

```bash
cargo test
```

To run only integration tests:

```bash
cargo test --test integration_tests
```

To run only unit tests:

```bash
cargo test --test unit_tests
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