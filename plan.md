# Gmail Calendar MCP Server - Realistic Test Coverage Plan

## Executive Summary

This test plan aims to systematically improve code coverage from the current 10.57% to 100%. The plan divides work into focused phases, addressing each module's specific testing needs and providing realistic milestones. We'll leverage Rust's testing ecosystem with mockall for mocking and tarpaulin for coverage tracking.

## Current Coverage Status (10.78%)

| Module | Coverage | Lines Covered | Total Lines |
|--------|----------|---------------|-------------|
| auth.rs | 47.22% | 34 | 72 |
| config.rs | 100.00% | 19 | 19 |
| utils.rs | 100.00% | 116 | 116 |
| logging.rs | 100.00% | 31 | 31 |
| server.rs | 0.67% | 3 | 450 |
| calendar_api.rs | 0.00% | 0 | 309 |
| gmail_api.rs | 0.00% | 0 | 288 |
| people_api.rs | 0.00% | 0 | 213 |
| oauth.rs | 0.00% | 0 | 260 |
| main.rs | 0.00% | 0 | 43 |
| errors.rs | 0.00% | 0 | 6 |

## Phase 1: Existing High-Coverage Modules Completion ✅

### 1.1 Config Module Testing ✅
- **Current Coverage:** 100.00% (19/19 lines)
- **Completed Actions:**
  - ✅ Tested environment variable handling edge cases
  - ✅ Added tests for dotenv integration
  - ✅ Verified API URL constants
- **Testing Strategies Used:**
  - ✅ Enhanced environment variable mocking
  - ✅ Added comprehensive tests for config permutations
- **Success Metrics:**
  - ✅ 100% line coverage for config.rs achieved

### 1.2 Utils Module Testing ✅
- **Current Coverage:** 100.00% (116/116 lines)
- **Completed Actions:**
  - ✅ Tested all utility functions
  - ✅ Verified error handling edge cases
  - ✅ Enhanced base64 encoding/decoding tests
- **Testing Strategies Used:**
  - ✅ Added table-driven tests for all functions
  - ✅ Implemented comprehensive error case testing
- **Success Metrics:**
  - ✅ 100% line coverage for utils.rs achieved

### 1.3 Logging Module Testing ✅
- **Current Coverage:** 100.00% (31/31 lines)
- **Completed Actions:**
  - ✅ Tested log level filtering
  - ✅ Verified file path handling edge cases
  - ✅ Tested custom formatting
- **Testing Strategies Used:**
  - ✅ Mocked filesystem operations
  - ✅ Tested environment variable interactions
- **Success Metrics:**
  - ✅ 100% line coverage for logging.rs achieved

## Phase 2: Moderate-Coverage Module Enhancement

### 2.1 Auth Module Enhancement
- **Current Coverage:** 47.22% (34/72 lines)
- **Action Items:**
  - Test token refresh error scenarios
  - Implement expired token edge cases
  - Test token creation with various parameters
  - Test secure token handling
- **Testing Strategies:**
  - Use parameterized tests for error conditions
  - Implement time-based tests with mock time
  - Test concurrent access patterns
- **Success Metrics:**
  - 100% line coverage for auth.rs
  - All error paths verified

## Phase 3: Zero-Coverage Critical API Modules

### 3.1 Gmail API Testing
- **Current Coverage:** 0% (0/288 lines)
- **Action Items:**
  - Create comprehensive mock responses for Gmail API
  - Test email parsing with various formats
  - Test MIME message generation
  - Test draft email creation
  - Test search functions
  - Test error handling paths
- **Testing Strategies:**
  - Create realistic mock data for email formats
  - Implement stateful mocks for API interactions
  - Test international character handling
- **Success Metrics:**
  - 95% line coverage for gmail_api.rs
  - All public methods have tests

### 3.2 Calendar API Testing
- **Current Coverage:** 0% (0/309 lines)
- **Action Items:**
  - Implement mocks for Calendar API responses
  - Test event creation and retrieval
  - Test date/time handling and timezones
  - Test recurring events
  - Test error handling for API failures
- **Testing Strategies:**
  - Create mock calendar data with various properties
  - Test timezone conversions
  - Test validation logic
- **Success Metrics:**
  - 95% line coverage for calendar_api.rs
  - All public methods have tests

### 3.3 People API Testing
- **Current Coverage:** 0% (0/213 lines)
- **Action Items:**
  - Create mock responses for contact operations
  - Test contact fetching and formatting
  - Test search operations
  - Test error handling
- **Testing Strategies:**
  - Create diverse contact records for testing
  - Test international name handling
  - Test error paths
- **Success Metrics:**
  - 95% line coverage for people_api.rs
  - All public methods have tests

## Phase 4: Infrastructure and Complex Modules

### 4.1 OAuth Module Testing
- **Current Coverage:** 0% (0/260 lines)
- **Action Items:**
  - Test OAuth flow initialization
  - Test token exchange
  - Test OAuth URL generation
  - Test error handling in OAuth flows
- **Testing Strategies:**
  - Mock HTTP responses for OAuth endpoints
  - Test authorization code flow
  - Test refresh token flow
- **Success Metrics:**
  - 95% line coverage for oauth.rs
  - All authentication flows verified

### 4.2 Error Handling Testing
- **Current Coverage:** 0% (0/6 lines)
- **Action Items:**
  - Test all error types
  - Test error code constants
  - Test error conversions
  - Test error formatting
- **Testing Strategies:**
  - Create tests for all error variants
  - Test error message generation
- **Success Metrics:**
  - 100% line coverage for errors.rs
  - All error types have tests

## Phase 5: Server and Integration

### 5.1 Server Module Testing
- **Current Coverage:** 0.67% (3/450 lines)
- **Action Items:**
  - Test command parsing and routing
  - Test all MCP commands
  - Test server initialization
  - Test error handling in responses
  - Test prompt handling
- **Testing Strategies:**
  - Create realistic MCP command mocks
  - Test request/response pairs
  - Test error propagation
- **Success Metrics:**
  - 90% line coverage for server.rs
  - All public endpoints tested

### 5.2 Main Function Testing
- **Current Coverage:** 0% (0/43 lines)
- **Action Items:**
  - Test command line argument parsing
  - Test environment detection
  - Test server startup and initialization
  - Test error handling for startup
- **Testing Strategies:**
  - Mock command line arguments
  - Test environment variable interactions
- **Success Metrics:**
  - 95% line coverage for main.rs
  - All startup paths verified

## Phase 6: Advanced Testing Techniques

### 6.1 Property-Based Testing
- **Action Items:**
  - Implement property-based tests for encoding/decoding
  - Test date/time operations
  - Test JSON serialization/deserialization
  - Test email format conversion
- **Testing Strategies:**
  - Use proptest crate for diverse test cases
  - Test roundtrip properties
- **Success Metrics:**
  - Critical invariants verified
  - Edge cases discovered and fixed

### 6.2 Performance Benchmarking ✅
- **Completed Actions:**
  - ✅ Benchmarked email parsing
  - ✅ Benchmarked API request handling
  - ✅ Benchmarked token operations
  - ✅ Benchmarked search operations
- **Testing Strategies Used:**
  - ✅ Used Criterion.rs for benchmarks
  - ✅ Established performance baselines
- **Success Metrics:**
  - ✅ Benchmarks integrated into CI
  - ✅ Performance metrics documented

## Implementation Timeline

| Phase | Focus Area | Est. Duration | Target Coverage | Status |
|-------|------------|---------------|----------------|--------|
| 1 | High-Coverage Modules | 1 week | 20% | ✅ Completed |
| 2 | Auth Module | 1 week | 35% | ⏳ In Progress |
| 3 | API Modules | 3 weeks | 70% | 📅 Planned |
| 4 | Infrastructure | 2 weeks | 85% | 📅 Planned |
| 5 | Server & Integration | 2 weeks | 95% | 📅 Planned |
| 6 | Advanced Techniques | 1 week | 100% | ⏳ Partial (6.2 Complete) |

## Success Criteria

- Overall code coverage reaches 95%+ across all modules
- All critical paths have tests
- Error handling is thoroughly tested
- Integration tests verify complete workflows
- Performance benchmarks establish baselines ✅