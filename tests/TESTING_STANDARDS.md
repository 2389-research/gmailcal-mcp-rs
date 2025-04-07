# Testing Standards for Gmail MCP Server

This document outlines the standard patterns and best practices for writing tests in the Gmail MCP Server codebase. Following these standards ensures consistency, maintainability, and effective test coverage.

## Test Organization

### File Structure

- **Unit tests**: Located beside the code they test, in the same file, using Rust's `#[cfg(test)]` module
- **Integration tests**: Located in the `/tests` directory
- **Mock objects and fixtures**: Centralized in the `/tests/mock_client.rs` and `/tests/helper.rs` files
- **Test utilities**: Shared code in `/tests/helper.rs`

### Naming Conventions

- Test modules: `mod test_<feature>` or `mod <feature>_tests`
- Test functions: `test_<scenario_being_tested>`
- Test cases in parameterized tests: Descriptive of the specific variation

## Test Types and Patterns

### Unit Tests

Unit tests should focus on testing a single component in isolation.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_specific_function_behavior() {
        // Arrange
        let input = 5;
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected_output);
    }
}
```

### Integration Tests

Integration tests should verify how components work together.

```rust
// In /tests/integration_tests.rs
#[test]
fn test_end_to_end_workflow() {
    // Set up environment
    // Execute workflow
    // Verify results
}
```

### Parameterized Tests

For testing multiple scenarios with the same test logic, use the test case pattern:

```rust
#[test]
fn test_with_multiple_inputs() {
    let test_cases = [
        TestCase::new("valid_input", "input1", expected1),
        TestCase::new("edge_case", "input2", expected2),
        TestCase::new("error_case", "bad_input", expected_error),
    ];
    
    run_test_cases(&test_cases, |input| {
        // Test logic that converts input to output
        function_under_test(input)
    });
}
```

Or use the parameterized_test macro:

```rust
fn test_function(param: &str) {
    // Test logic using param
}

parameterized_test!(test_function, case_a, case_b, case_c);
```

### Mocking External Dependencies

Use `mockall` to create mock implementations of external dependencies:

```rust
use mockall::mock;

mock! {
    pub ApiClient {
        pub fn fetch_data(&self) -> Result<String, Error>;
    }
}

#[test]
fn test_with_mock() {
    let mut mock = MockApiClient::new();
    mock.expect_fetch_data()
        .returning(|| Ok("mocked data".to_string()));
        
    // Use mock in test
}
```

### Environment Variables

Use `EnvVarGuard` for tests that need to set environment variables:

```rust
#[test]
fn test_with_env_vars() {
    let mut env = EnvVarGuard::new();
    env.set("API_KEY", "test_key");
    
    // Test logic that uses environment variables
    
    // EnvVarGuard will restore original env vars when dropped
}
```

## Best Practices

### Error Testing

Every function that returns a `Result` type should have tests for both success and error cases:

```rust
#[test]
fn test_success_case() {
    assert!(function_under_test().is_ok());
}

#[test]
fn test_error_case() {
    assert!(function_under_test_with_bad_input().is_err());
}
```

### API Testing

When testing code that interacts with external APIs:

1. Use mock clients from `/tests/mock_client.rs`
2. Test both normal and error responses
3. Test handling of malformed responses
4. Verify request formation

```rust
#[test]
fn test_api_interaction() {
    let mock = create_mock_client();
    // Configure mock expectations
    
    let api = ApiWithClient::new(mock);
    let result = api.perform_operation();
    
    // Verify result
}
```

### Time-Based Testing

For functions that depend on time, use `MockTimeProvider`:

```rust
#[test]
fn test_time_dependent_function() {
    let mut time_provider = MockTimeProvider::new(1617235200); // April 1, 2021
    
    // Test with initial time
    let result1 = function_with_time_provider(&time_provider);
    
    // Advance time and test again
    time_provider.advance(3600); // Advance 1 hour
    let result2 = function_with_time_provider(&time_provider);
    
    // Verify time-dependent behavior
}
```

### JSON Testing

Use the provided assertion macro for JSON comparisons:

```rust
#[test]
fn test_json_response_handling() {
    let actual = get_json_response();
    let expected = json!({
        "key": "value",
        "nested": {
            "important_field": "expected_value"
        }
    });
    
    assert_json_contains!(actual, expected);
}
```

## Test Coverage Expectations

- All public API functions should have at least one test
- All error conditions should be tested
- Edge cases and boundary conditions should be tested
- Critical paths should have integration tests

## Adding New Tests

When adding new functionality:

1. Add unit tests for new functions
2. Update or add integration tests for the feature
3. Ensure tests cover both success and error paths
4. Consider edge cases and add specific tests
5. Run the full test suite with `cargo test` before submitting

## Code Coverage

Use `cargo tarpaulin` to check code coverage:

```bash
cargo tarpaulin
```

We aim for 100% line coverage for critical components and at least 80% overall.