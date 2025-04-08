# Fixing Runtime Conflicts Between Tokio and Mockito

This document provides guidance on how to fix the runtime conflicts between tokio and mockito in our test suite.

## The Problem

We're currently encountering runtime conflicts in tests that use both tokio and mockito. The error message is:

```
Cannot start a runtime from within a runtime. This happens because a function (like `block_on`) attempted to block the current thread while the thread is being used to drive asynchronous tasks.
```

This happens because:
1. The `#[tokio::test]` attribute creates a tokio runtime for async testing
2. The mockito server also creates its own runtime
3. When both run in the same test, they conflict

## Current Workaround

Our current workaround is to add `#[ignore]` attributes to the affected tests. This allows the rest of the test suite to run but means these tests are never executed, reducing our test coverage.

## Proper Solution

There are several ways to fix this issue:

### Option 1: Use Regular Tests with Manual Runtime Creation

Instead of using `#[tokio::test]`, use regular tests and create a tokio runtime manually inside the test:

```rust
#[test]
fn test_parse_simple_message() {
    // Create a mock server first (outside tokio runtime)
    let mut server = mockito::Server::new();
    
    // Mock the Gmail API message endpoint
    let mock = server.mock("GET", "/gmail/v1/users/me/messages/12345?format=full")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(message_json.to_string())
        .create();
    
    // Create an isolated tokio runtime for the async portion
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    // Run the async code inside the runtime
    rt.block_on(async {
        // Create service and make API calls here
        let result = gmail_service.get_message_details("12345").await;
        
        // Assertions inside the runtime
        assert!(result.is_ok());
    });
    
    // Mock assertions outside the runtime
    mock.assert();
}
```

### Option 2: Use Async Test Runtime Features

Use tokio test runtime features to control runtime behavior:

```rust
#[tokio::test(flavor = "current_thread")]
async fn test_parse_simple_message() {
    // Test code here
}
```

This creates a more limited runtime that may not conflict with mockito.

### Option 3: Refactor to Mock at a Different Level

Instead of using mockito to create a mock HTTP server, consider:

1. Mocking at the HTTP client level using a crate like `mockall` or `wiremock`
2. Creating a trait abstraction for HTTP requests that can be mocked independently

## Implementation Plan

For the most reliable approach, we recommend Option 1:

1. Convert all `#[tokio::test]` attributes to regular `#[test]` functions
2. Create the mockito server outside any tokio runtime
3. Create a manual runtime with `tokio::runtime::Runtime::new()`
4. Run async code with `rt.block_on(async { ... })`
5. Keep mock assertions outside the tokio runtime

## Example Implementation

Here's a complete example implementation for `test_parse_simple_message`:

```rust
#[test]
fn test_parse_simple_message() {
    // Create a mock server first (outside tokio runtime)
    let mut server = mockito::Server::new();
    
    // Create a simple test message JSON
    let message_id = "12345";
    let message_json = create_email_json(
        message_id,
        "thread123",
        "Test Subject",
        "sender@example.com",
        "recipient@example.com",
        "This is a test email body",
        None, // No HTML part
    );
    
    // Mock the Gmail API message endpoint
    let mock = server.mock("GET", format!("/gmail/v1/users/me/messages/{}?format=full", message_id).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(message_json.to_string())
        .create();
    
    // Override Gmail API URL
    env::set_var("GMAIL_API_BASE_URL", server.url());
    env::set_var("OAUTH_TOKEN_URL", format!("{}/token", server.url()));
    
    // Create a runtime manually
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    // Use the runtime to run our async code
    rt.block_on(async {
        // Create GmailService with mock config
        let mut gmail_service = GmailService::new(&create_mock_config()).unwrap();
        
        // Get message details
        let result = gmail_service.get_message_details(message_id).await;
        
        // Check the result
        assert!(result.is_ok());
        let message = result.unwrap();
        
        // Verify fields
        assert_eq!(message.id, "12345");
        assert_eq!(message.thread_id, "thread123");
        assert_eq!(message.subject.unwrap(), "Test Subject");
        assert_eq!(message.from.unwrap(), "sender@example.com");
        assert_eq!(message.to.unwrap(), "recipient@example.com");
        assert!(message.body_text.is_some());
        assert_eq!(message.body_text.unwrap(), "This is a test email body");
        assert!(message.body_html.is_none()); // No HTML body in this test
    });
    
    // Verify the mock was called (outside the runtime)
    mock.assert();
}
```

Implementing this approach across all affected test files will allow us to run all tests and achieve better code coverage.