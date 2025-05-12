# Code Coverage Improvement Plan

Looking at the cobertura.xml file, I see the current code coverage is only 2.32% (lines-covered="42" out of lines-valid="1807"). Let's create a comprehensive plan to improve this coverage to 100%, breaking it down into manageable issues.

## Overview of Current Coverage

The most covered module is `auth.rs` at 47.22%, while many other modules have 0% coverage. Here's the plan to methodically improve coverage:

## Issue #1: Set Up Test Infrastructure

**Title:** Set up test infrastructure for improved code coverage tracking

**Description:**
- Configure CI to run coverage reports automatically
- Create a baseline report for tracking progress
- Set up mocking framework for external dependencies (Google APIs)
- Create utilities for test fixtures and common testing patterns

**Acceptance Criteria:**
- CI pipeline automatically generates coverage reports
- Coverage reports are available for each PR
- Basic mocking utilities are available for all tests

## Issue #2: Expand Auth Module Tests (Current: 47.22%)

**Title:** Increase test coverage for auth.rs to 100%

**Description:**
The auth.rs module already has 47.22% coverage, making it a good starting point. We need to complete testing:
- Test error scenarios for token refresh
- Cover edge cases for handling expired tokens
- Test concurrency patterns

**Acceptance Criteria:**
- All public methods in TokenManager have tests
- All error handling paths are tested
- 100% line coverage for auth.rs

## Issue #3: Implement Config Module Tests (Current: 15.79%)

**Title:** Increase test coverage for config.rs to 100%

**Description:**
The config.rs module needs expanded testing:
- Test Config::from_env with various environment configurations
- Test edge cases for missing environment variables
- Test handling of malformed configuration values
- Test token expiry logic

**Acceptance Criteria:**
- All public methods and functions have tests
- All error paths are covered
- 100% line coverage for config.rs

## Issue #4: Add Utils Module Tests (Current: 3.45%)

**Title:** Increase test coverage for utils.rs to 100%

**Description:**
The utils.rs module needs comprehensive testing:
- Test all error mapping functions
- Test base64 encoding/decoding functions with various inputs
- Test parsing functions with edge cases
- Test McpError creation and formatting

**Acceptance Criteria:**
- All utility functions have tests
- Edge cases and error paths are covered
- 100% line coverage for utils.rs

## Issue #5: Implement Calendar API Tests (Current: 0%)

**Title:** Add tests for calendar_api.rs module

**Description:**
Create comprehensive tests for the Calendar API module:
- Implement mocks for Google Calendar API responses
- Test event creation, retrieval, and listing functions
- Test date/time parsing and formatting
- Test error handling for API failures

**Acceptance Criteria:**
- All public methods in CalendarClient have tests
- All data structures can be properly serialized/deserialized
- Error handling paths are tested
- 100% line coverage for calendar_api.rs

## Issue #6: Implement Gmail API Tests (Current: 0%)

**Title:** Add tests for gmail_api.rs module

**Description:**
Create extensive tests for the Gmail API module:
- Implement mocks for Gmail API responses
- Test email listing, retrieval, and search functions
- Test email parsing and format conversion
- Test draft email creation
- Test MIME message generation
- Test error handling paths

**Acceptance Criteria:**
- All public methods in GmailService have tests
- Email parsing functions are thoroughly tested
- Error handling paths are covered
- 100% line coverage for gmail_api.rs

## Issue #7: Implement People API Tests (Current: 0%)

**Title:** Add tests for people_api.rs module

**Description:**
Create comprehensive tests for the People API module:
- Implement mocks for Google People API responses
- Test contact listing and search functions
- Test contact parsing and format conversion
- Test edge cases in contact data structures
- Test error handling for API failures

**Acceptance Criteria:**
- All public methods in PeopleClient have tests
- Contact parsing functions are thoroughly tested
- Error handling paths are covered
- 100% line coverage for people_api.rs

## Issue #8: Implement Logging Tests (Current: 0%)

**Title:** Add tests for logging.rs module

**Description:**
Develop tests for the logging functionality:
- Test log initialization with various configurations
- Test log level filtering
- Test file logging vs. in-memory logging
- Test log message formatting
- Test error handling for logging setup failures

**Acceptance Criteria:**
- All logging setup paths are tested
- File access errors are properly handled and tested
- 100% line coverage for logging.rs

## Issue #9: Implement Error Handling Tests (Current: 0%)

**Title:** Expand tests for errors.rs module

**Description:**
Create comprehensive tests for error definitions and handling:
- Test creation of all error types
- Test error conversion between different types
- Test error formatting for user-friendly messages
- Test error code mapping functions

**Acceptance Criteria:**
- All error types and conversions are tested
- Error formatting is verified
- 100% line coverage for errors.rs

## Issue #10: Implement Server Tests (Current: 0.22%)

**Title:** Add tests for server.rs module

**Description:**
Create extensive tests for the MCP server implementation:
- Test command parsing and routing
- Test all MCP tools and commands
- Test error handling and response formatting
- Test prompt functionality
- Test server initialization and shutdown

**Acceptance Criteria:**
- All MCP commands have tests for success and failure cases
- Command routing logic is thoroughly tested
- Error handling and response formatting is verified
- 100% line coverage for server.rs

## Issue #11: Integration Tests for Complete Workflows

**Title:** Develop end-to-end integration tests for common workflows

**Description:**
Create comprehensive integration tests for common user workflows:
- Email search and retrieval workflows
- Calendar event creation and listing workflows
- Contact search and retrieval workflows
- Authentication workflows
- Error recovery workflows

**Acceptance Criteria:**
- Tests cover complete user workflows from start to finish
- All major integration points between modules are tested
- Error recovery paths are verified

## Issue #12: Main Function Tests

**Title:** Add tests for main.rs module

**Description:**
Create tests for the application entry point:
- Test command-line argument parsing
- Test environment detection logic
- Test server startup and initialization
- Test error handling for startup failures

**Acceptance Criteria:**
- All command-line options have tests
- Environment detection logic is verified
- Server initialization error handling is tested
- 100% line coverage for main.rs

## Issue #13: Improve Existing Test Files

**Title:** Enhance and expand existing test files

**Description:**
Review and enhance existing test files:
- Expand mock_client.rs to cover more API scenarios
- Enhance integration_tests.rs to cover more workflows
- Expand error_tests.rs to cover all error types
- Complete implementation of calendar_api_tests.rs and other partial tests

**Acceptance Criteria:**
- All existing test files provide comprehensive coverage
- Mock implementations are complete and reusable
- Test fixtures are well-organized and maintainable

## Issue #14: Set Up Property-Based Testing

**Title:** Implement property-based testing for critical functions

**Description:**
Add property-based testing for functions that should satisfy certain invariants:
- Base64 encoding/decoding should be reversible
- Date/time parsing and formatting should be reversible
- JSON serialization/deserialization should be reversible
- Email format conversion should maintain data integrity

**Acceptance Criteria:**
- Critical functions have property-based tests
- Tests verify invariants with randomly generated inputs
- Edge cases are discovered and tested

## Issue #15: Set Up Performance Benchmarks

**Title:** Create performance benchmarks for critical operations

**Description:**
Implement benchmarks to ensure code changes don't introduce performance regressions:
- Email parsing performance
- API request handling
- Token refresh operations
- Contact search operations
- Calendar event processing

**Acceptance Criteria:**
- Benchmarks are automated and run in CI
- Performance metrics are tracked over time
- Critical operations have performance baselines

## Summary

This plan divides the work into 15 manageable issues, focusing on one module at a time. By methodically working through these issues, we can achieve 100% code coverage while also improving the overall quality and reliability of the codebase.

The plan prioritizes:
1. Setting up proper test infrastructure
2. Building on existing partial coverage
3. Adding tests for core functionality
4. Creating integration tests for complete workflows
5. Implementing advanced testing techniques

Each issue is designed to be relatively self-contained, allowing for parallel work by multiple contributors if needed.
