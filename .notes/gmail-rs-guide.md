# Gmail-rs Implementation Guide

This guide provides a comprehensive overview of how to use the gmail-rs crate to interact with the Gmail API from Rust applications.

## Table of Contents

1. [Introduction](#introduction)
2. [Installation](#installation)
3. [Authentication](#authentication)
4. [Basic Usage](#basic-usage)
5. [Working with Messages](#working-with-messages)
6. [Working with Drafts](#working-with-drafts)
7. [Working with Threads](#working-with-threads)
8. [Working with Labels](#working-with-labels)
9. [Settings Management](#settings-management)
10. [Error Handling](#error-handling)
11. [Advanced Usage](#advanced-usage)

## Introduction

The gmail-rs crate is a Rust client library for the Gmail API. It provides a fluent interface for interacting with all Gmail API endpoints, allowing you to manage emails, drafts, threads, labels, and various Gmail settings.

The crate is built on top of the `httpclient` crate and uses OAuth2 for authentication.

## Installation

Add the gmail crate to your `Cargo.toml`:

```toml
[dependencies]
gmail = "0.17.0"
tokio = { version = "1.18.2", features = ["full"] }
```

## Authentication

The gmail-rs crate uses OAuth2 for authentication. You need to set up a project in the Google Cloud Console and enable the Gmail API. Once you have your OAuth2 credentials, you can authenticate with the API in several ways.

### Using Environment Variables

The simplest approach is to use environment variables:

```bash
export GMAIL_CLIENT_ID="your-client-id"
export GMAIL_CLIENT_SECRET="your-client-secret"
export GMAIL_REDIRECT_URI="your-redirect-uri"
export GMAIL_ACCESS_TOKEN="your-access-token"
export GMAIL_REFRESH_TOKEN="your-refresh-token"
```

Then in your code:

```rust
use gmail::GmailClient;

let client = GmailClient::from_env();
```

### Using Explicit Authentication

If you prefer to set the authentication details programmatically:

```rust
use gmail::{GmailClient, GmailAuth};
use std::sync::Arc;

// Initialize OAuth2 flow and middleware
let auth = GmailAuth::oauth2(
    "your-access-token",
    "your-refresh-token",
    None // Optional callback function for handling token refresh
);

let client = GmailClient::with_auth(auth);
```

## Basic Usage

The gmail-rs crate uses a fluent interface pattern. Here's a basic example of getting a user's profile:

```rust
use gmail::GmailClient;

#[tokio::main]
async fn main() {
    // Create a client using environment variables for authentication
    let client = GmailClient::from_env();
    
    // Get the user's profile (typically 'me' for the authenticated user)
    let user_id = "me";
    let profile = client.get_profile(user_id).await.unwrap();
    
    println!("Email: {}", profile.email_address.unwrap());
    println!("Messages total: {}", profile.messages_total.unwrap());
}
```

## Working with Messages

### Listing Messages

To list messages in the user's mailbox:

```rust
let response = client
    .messages_list("me")
    .max_results(10)  // Optional: limit number of results
    .q("is:unread")   // Optional: query parameter
    .await
    .unwrap();

for message in response.messages.unwrap() {
    println!("Message ID: {}", message.id);
}
```

### Getting a Message

To fetch a specific message:

```rust
let message = client
    .messages_get("message-id", "me")
    .format("full")  // Optional: message format
    .await
    .unwrap();

println!("Subject: {}", message.payload.headers.iter()
    .find(|h| h.name == "Subject")
    .map(|h| &h.value)
    .unwrap_or(&String::from("No subject")));
```

### Sending a Message

To send a new email:

```rust
use httpclient::InMemoryBody;

// Create a raw email message in RFC 2822 format
let email_content = "From: me@example.com\r\n\
                    To: recipient@example.com\r\n\
                    Subject: Hello from Rust\r\n\
                    \r\n\
                    This is a test email sent using the gmail-rs crate.";

let response = client
    .messages_send("me", InMemoryBody::Text(email_content.to_string()), None)
    .await
    .unwrap();

println!("Sent message ID: {}", response.id);
```

### Modifying a Message

To add or remove labels from a message:

```rust
let response = client
    .messages_modify("message-id", "me")
    .add_label_ids(&["INBOX", "UNREAD"])
    .remove_label_ids(&["SPAM"])
    .await
    .unwrap();

println!("Modified message: {}", response.id);
```

### Deleting a Message

To delete a message permanently:

```rust
client.messages_delete("message-id", "me").await.unwrap();
```

### Moving to Trash

To move a message to trash (safer than permanent deletion):

```rust
client.messages_trash("message-id", "me").await.unwrap();
```

## Working with Drafts

### Creating a Draft

```rust
// Create a draft using the same format as sending a message
let draft = client
    .drafts_create("me")
    // Additional parameters would go here
    .await
    .unwrap();

println!("Created draft ID: {}", draft.id.unwrap());
```

### Listing Drafts

```rust
let response = client
    .drafts_list("me")
    .max_results(10)
    .await
    .unwrap();

for draft in response.drafts.unwrap() {
    println!("Draft ID: {}", draft.id.unwrap());
}
```

### Updating a Draft

```rust
let updated_draft = client
    .drafts_update("draft-id", "me")
    // Update parameters would go here
    .await
    .unwrap();
```

### Sending a Draft

```rust
let message = client
    .drafts_send("me")
    // Additional parameters for the draft
    .await
    .unwrap();

println!("Sent message ID: {}", message.id);
```

## Working with Threads

### Listing Threads

```rust
let response = client
    .threads_list("me")
    .max_results(10)
    .q("is:important")
    .await
    .unwrap();

for thread in response.threads {
    println!("Thread ID: {}", thread.id);
}
```

### Getting a Thread

```rust
let thread = client
    .threads_get("thread-id", "me")
    .format("full")
    .await
    .unwrap();

println!("Thread messages count: {}", thread.messages.len());
```

### Modifying a Thread

```rust
let thread = client
    .threads_modify("thread-id", "me")
    .add_label_ids(&["IMPORTANT"])
    .remove_label_ids(&["CATEGORY_UPDATES"])
    .await
    .unwrap();
```

### Moving a Thread to Trash

```rust
client.threads_trash("thread-id", "me").await.unwrap();
```

## Working with Labels

### Listing Labels

```rust
let response = client.labels_list("me").await.unwrap();

if let Some(labels) = response.labels {
    for label in labels {
        println!("Label: {:?}", label);
    }
}
```

### Creating a Label

```rust
let label = client
    .labels_create("me")
    .name("MyNewLabel")
    .label_list_visibility("labelShow")
    .message_list_visibility("show")
    .await
    .unwrap();

println!("Created label ID: {}", label.id.unwrap());
```

### Updating a Label

```rust
let label = client
    .labels_update("label-id", "me")
    .name("UpdatedLabelName")
    .await
    .unwrap();
```

### Deleting a Label

```rust
client.labels_delete("label-id", "me").await.unwrap();
```

## Settings Management

The gmail-rs crate provides access to various Gmail settings:

### Get Profile

```rust
let profile = client.get_profile("me").await.unwrap();
println!("Email: {}", profile.email_address.unwrap());
```

### Auto-Forwarding Settings

```rust
let forwarding = client.settings_get_auto_forwarding("me").await.unwrap();
println!("Auto-forwarding enabled: {}", forwarding.enabled.unwrap_or(false));
```

### IMAP Settings

```rust
let imap = client.settings_get_imap("me").await.unwrap();
println!("IMAP enabled: {}", imap.enabled.unwrap_or(false));
```

### Vacation Responder

```rust
let vacation = client.settings_get_vacation("me").await.unwrap();
println!("Vacation auto-reply enabled: {}", vacation.enable_auto_reply.unwrap_or(false));
```

## Error Handling

The gmail-rs crate returns `Result` types from all API calls, allowing you to handle errors using Rust's standard error handling mechanisms:

```rust
match client.get_profile("me").await {
    Ok(profile) => {
        println!("Successfully retrieved profile for {}", profile.email_address.unwrap());
    },
    Err(err) => {
        eprintln!("Failed to retrieve profile: {}", err);
        // Handle specific error types
        match err {
            gmail::Error::Protocol(protocol_err) => {
                // Handle protocol errors
            },
            // Handle other error types
            _ => {}
        }
    }
}
```

## Advanced Usage

### Watching for Mailbox Changes

```rust
let response = client
    .watch("me")
    .topic_name("projects/myproject/topics/gmail-notifications")
    .label_ids(&["INBOX"])
    .await
    .unwrap();

println!("Watch expires at: {}", response.expiration.unwrap());
println!("History ID: {}", response.history_id.unwrap());
```

### Getting History Changes

```rust
let response = client
    .history_list("me", "start-history-id")
    .max_results(10)
    .history_types(&["messageAdded", "labelAdded"])
    .await
    .unwrap();

for history in response.history {
    println!("History ID: {}", history.id);
}
```

### Customizing HTTP Client

If you need to customize the underlying HTTP client:

```rust
use httpclient::Client;
use gmail::{GmailClient, GmailAuth, init_http_client, default_http_client};

// Initialize with custom middleware or settings
let custom_client = default_http_client()
    .with_middleware(/* custom middleware */);

init_http_client(custom_client);

// Now create the Gmail client
let client = GmailClient::from_env();
```

---

This guide covers the basic and advanced usage patterns for the gmail-rs crate. Refer to the official documentation or the source code for more detailed information about specific API endpoints and options.
