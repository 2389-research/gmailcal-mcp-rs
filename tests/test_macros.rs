/// Test Macros Module
///
/// This module provides helper macros for common test patterns
/// in the Gmail MCP Server codebase.
///

/// Macro for testing that a function returns an Ok result
#[macro_export]
macro_rules! assert_ok {
    ($expression:expr) => {
        match $expression {
            Ok(value) => value,
            Err(err) => panic!("Expected Ok but got Err: {:?}", err),
        }
    };
    ($expression:expr, $message:expr) => {
        match $expression {
            Ok(value) => value,
            Err(err) => panic!("{}: {:?}", $message, err),
        }
    };
}

/// Macro for testing that a function returns an Err result
#[macro_export]
macro_rules! assert_err {
    ($expression:expr) => {
        match $expression {
            Ok(value) => panic!("Expected Err but got Ok: {:?}", value),
            Err(err) => err,
        }
    };
    ($expression:expr, $message:expr) => {
        match $expression {
            Ok(value) => panic!("{}: {:?}", $message, value),
            Err(err) => err,
        }
    };
}

/// Macro for testing that an error matches a specific error variant
#[macro_export]
macro_rules! assert_err_variant {
    ($expression:expr, $pattern:pat) => {
        match $expression {
            Ok(value) => panic!("Expected Err but got Ok: {:?}", value),
            Err($pattern) => (),
            Err(err) => panic!("Error didn't match expected pattern: {:?}", err),
        }
    };
    ($expression:expr, $pattern:pat, $message:expr) => {
        match $expression {
            Ok(value) => panic!("{}: {:?}", $message, value),
            Err($pattern) => (),
            Err(err) => panic!("{}: {:?}", $message, err),
        }
    };
}

/// Macro for testing async functions that should return Ok
#[macro_export]
macro_rules! assert_async_ok {
    ($expression:expr) => {
        match tokio_test::block_on($expression) {
            Ok(value) => value,
            Err(err) => panic!("Expected Ok but got Err: {:?}", err),
        }
    };
    ($expression:expr, $message:expr) => {
        match tokio_test::block_on($expression) {
            Ok(value) => value,
            Err(err) => panic!("{}: {:?}", $message, err),
        }
    };
}

/// Macro for testing async functions that should return Err
#[macro_export]
macro_rules! assert_async_err {
    ($expression:expr) => {
        match tokio_test::block_on($expression) {
            Ok(value) => panic!("Expected Err but got Ok: {:?}", value),
            Err(err) => err,
        }
    };
    ($expression:expr, $message:expr) => {
        match tokio_test::block_on($expression) {
            Ok(value) => panic!("{}: {:?}", $message, value),
            Err(err) => err,
        }
    };
}

/// Macro for setting up temporary environment variables for a test
#[macro_export]
macro_rules! with_env_vars {
    ($(($key:expr, $value:expr)),* => $body:expr) => {{
        let mut guard = crate::helper::EnvVarGuard::new();
        $(
            guard.set($key, $value);
        )*
        let result = $body;
        result
    }};
}

/// Macro for testing a function with multiple input/output pairs
#[macro_export]
macro_rules! test_cases {
    ($test_fn:expr, $(($name:expr, $input:expr, $expected:expr)),* $(,)?) => {
        $(
            let result = $test_fn($input);
            assert_eq!(
                result, $expected,
                "Test case '{}' failed: expected {:?}, got {:?}",
                $name, $expected, result
            );
        )*
    };
}

/// Macro for temporarily mocking the system time for tests
#[macro_export]
macro_rules! with_mock_time {
    ($epoch_seconds:expr => $body:expr) => {{
        let time_provider = crate::helper::MockTimeProvider::new($epoch_seconds);
        let result = $body(&time_provider);
        result
    }};
    ($epoch_seconds:expr, $advance_by:expr => $body:expr) => {{
        let mut time_provider = crate::helper::MockTimeProvider::new($epoch_seconds);
        let result = $body(&time_provider);
        time_provider.advance($advance_by);
        let result2 = $body(&time_provider);
        (result, result2)
    }};
}

/// Macro for testing functions that should timeout
#[macro_export]
macro_rules! assert_timeout {
    ($timeout_ms:expr, $expression:expr) => {{
        let result = tokio::time::timeout(
            std::time::Duration::from_millis($timeout_ms),
            $expression
        ).await;
        assert!(result.is_err(), "Expected timeout but expression completed");
    }};
}

/// Macro for checking if a collection contains an item matching a predicate
#[macro_export]
macro_rules! assert_contains {
    ($collection:expr, $predicate:expr) => {
        let found = $collection.iter().any($predicate);
        assert!(found, "Collection does not contain expected item");
    };
    ($collection:expr, $predicate:expr, $message:expr) => {
        let found = $collection.iter().any($predicate);
        assert!(found, "{}", $message);
    };
}

// End of macro definitions