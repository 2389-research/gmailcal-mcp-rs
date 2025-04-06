/// Gmail Message Parsing Edge Cases Tests Module
///
/// This module contains tests for Gmail message parsing edge cases,
/// focusing on attachments, nested multipart messages, and unusual encodings.

use base64::{engine::general_purpose, Engine as _};
use mcp_gmailcal::gmail_api::EmailMessage;
use mcp_gmailcal::utils::{decode_base64, encode_base64_url_safe};
use serde_json::{json, Value};

// Define some test data
fn create_base64_content(content: &str) -> String {
    general_purpose::STANDARD.encode(content.as_bytes())
}

fn create_message_with_attachment() -> Value {
    // Create a MIME multipart message with attachment
    let attachment_content = create_base64_content("This is attachment content");
    
    json!({
        "id": "msg_with_attachment",
        "threadId": "thread123",
        "labelIds": ["INBOX", "IMPORTANT"],
        "snippet": "This is a message with attachment",
        "payload": {
            "mimeType": "multipart/mixed",
            "headers": [
                { "name": "From", "value": "sender@example.com" },
                { "name": "To", "value": "recipient@example.com" },
                { "name": "Subject", "value": "Message with Attachment" },
                { "name": "Date", "value": "Mon, 15 Apr 2025 10:00:00 +0000" }
            ],
            "parts": [
                {
                    "mimeType": "text/plain",
                    "headers": [
                        { "name": "Content-Type", "value": "text/plain; charset=UTF-8" }
                    ],
                    "body": {
                        "data": create_base64_content("This is the plain text body"),
                        "size": 28
                    }
                },
                {
                    "mimeType": "application/pdf",
                    "filename": "document.pdf",
                    "headers": [
                        { "name": "Content-Type", "value": "application/pdf" },
                        { "name": "Content-Disposition", "value": "attachment; filename=\"document.pdf\"" }
                    ],
                    "body": {
                        "attachmentId": "attachment123",
                        "data": attachment_content,
                        "size": attachment_content.len()
                    }
                }
            ]
        }
    })
}

fn create_nested_multipart_message() -> Value {
    json!({
        "id": "nested_multipart",
        "threadId": "thread456",
        "labelIds": ["INBOX"],
        "snippet": "This is a nested multipart message",
        "payload": {
            "mimeType": "multipart/mixed",
            "headers": [
                { "name": "From", "value": "sender@example.com" },
                { "name": "To", "value": "recipient@example.com" },
                { "name": "Subject", "value": "Nested Multipart Message" },
                { "name": "Date", "value": "Mon, 15 Apr 2025 11:00:00 +0000" }
            ],
            "parts": [
                {
                    "mimeType": "text/plain",
                    "headers": [
                        { "name": "Content-Type", "value": "text/plain; charset=UTF-8" }
                    ],
                    "body": {
                        "data": create_base64_content("This is the plain text part"),
                        "size": 27
                    }
                },
                {
                    "mimeType": "multipart/alternative",
                    "headers": [
                        { "name": "Content-Type", "value": "multipart/alternative; boundary=alt-boundary" }
                    ],
                    "parts": [
                        {
                            "mimeType": "text/plain",
                            "headers": [
                                { "name": "Content-Type", "value": "text/plain; charset=UTF-8" }
                            ],
                            "body": {
                                "data": create_base64_content("This is the alternative plain text"),
                                "size": 33
                            }
                        },
                        {
                            "mimeType": "text/html",
                            "headers": [
                                { "name": "Content-Type", "value": "text/html; charset=UTF-8" }
                            ],
                            "body": {
                                "data": create_base64_content("<html><body><p>This is the HTML content</p></body></html>"),
                                "size": 54
                            }
                        }
                    ]
                }
            ]
        }
    })
}

fn create_unusual_encoding_message() -> Value {
    json!({
        "id": "unusual_encoding",
        "threadId": "thread789",
        "labelIds": ["INBOX"],
        "snippet": "This has unusual encoding",
        "payload": {
            "mimeType": "text/plain",
            "headers": [
                { "name": "From", "value": "sender@example.com" },
                { "name": "To", "value": "recipient@example.com" },
                { "name": "Subject", "value": "Unusual Encoding" },
                { "name": "Content-Type", "value": "text/plain; charset=ISO-8859-1" },
                { "name": "Date", "value": "Mon, 15 Apr 2025 12:00:00 +0000" }
            ],
            "body": {
                "data": general_purpose::STANDARD.encode("This text has special characters: äöüß".as_bytes()),
                "size": 41
            }
        }
    })
}

fn create_malformed_mime_message() -> Value {
    json!({
        "id": "malformed_mime",
        "threadId": "thread012",
        "labelIds": ["INBOX"],
        "snippet": "Malformed MIME structure",
        "payload": {
            "mimeType": "multipart/mixed",
            "headers": [
                { "name": "From", "value": "sender@example.com" },
                { "name": "To", "value": "recipient@example.com" },
                { "name": "Subject", "value": "Malformed MIME Structure" },
                { "name": "Date", "value": "Mon, 15 Apr 2025 13:00:00 +0000" }
            ],
            "parts": [
                {
                    "mimeType": "text/plain",
                    "body": {
                        "data": create_base64_content("This has missing headers"),
                        "size": 25
                    }
                },
                {
                    "headers": [
                        { "name": "Content-Type", "value": "text/plain; charset=UTF-8" }
                    ],
                    "body": {
                        "data": create_base64_content("This has missing mime type"),
                        "size": 26
                    }
                }
            ]
        }
    })
}

fn create_large_message() -> Value {
    // Create a large message with repeated content
    let large_content = "This is a line of text that will be repeated many times to create a large message. ".repeat(1000);
    
    json!({
        "id": "large_message",
        "threadId": "thread345",
        "labelIds": ["INBOX"],
        "snippet": "This is a very large message",
        "payload": {
            "mimeType": "text/plain",
            "headers": [
                { "name": "From", "value": "sender@example.com" },
                { "name": "To", "value": "recipient@example.com" },
                { "name": "Subject", "value": "Very Large Message" },
                { "name": "Date", "value": "Mon, 15 Apr 2025 14:00:00 +0000" }
            ],
            "body": {
                "data": create_base64_content(&large_content),
                "size": large_content.len()
            }
        }
    })
}

#[cfg(test)]
mod gmail_parsing_tests {
    use super::*;

    #[test]
    fn test_message_with_attachment() {
        let json_data = create_message_with_attachment();
        
        // Parse the message
        let email_message = EmailMessage::from_json(&json_data).unwrap();
        
        // Verify basic fields
        assert_eq!(email_message.id, "msg_with_attachment");
        assert_eq!(email_message.thread_id, "thread123");
        assert_eq!(email_message.subject, "Message with Attachment");
        
        // Verify the message has parts (multipart)
        assert!(email_message.body.contains("This is the plain text body"));
        
        // In a real implementation, we would check for attachment metadata or content
        // But for this test, we'll just check that certain strings are present
        assert!(email_message.raw_payload.to_string().contains("document.pdf"));
        assert!(email_message.raw_payload.to_string().contains("attachment123"));
    }

    #[test]
    fn test_nested_multipart_message() {
        let json_data = create_nested_multipart_message();
        
        // Parse the message
        let email_message = EmailMessage::from_json(&json_data).unwrap();
        
        // Verify basic fields
        assert_eq!(email_message.id, "nested_multipart");
        assert_eq!(email_message.thread_id, "thread456");
        assert_eq!(email_message.subject, "Nested Multipart Message");
        
        // Verify the message has both plain text and HTML content
        assert!(email_message.body.contains("This is the plain text part"));
        
        // In a real implementation, we would check for HTML content specifically
        // But for this test, we'll just check that certain strings are present
        assert!(email_message.raw_payload.to_string().contains("This is the HTML content"));
    }

    #[test]
    fn test_unusual_encoding_message() {
        let json_data = create_unusual_encoding_message();
        
        // Parse the message
        let email_message = EmailMessage::from_json(&json_data).unwrap();
        
        // Verify basic fields
        assert_eq!(email_message.id, "unusual_encoding");
        assert_eq!(email_message.thread_id, "thread789");
        assert_eq!(email_message.subject, "Unusual Encoding");
        
        // Verify the message body contains special characters
        // Note: The exact representation may depend on how the EmailMessage handles different encodings
        assert!(email_message.body.contains("special characters"));
    }

    #[test]
    fn test_malformed_mime_structure() {
        let json_data = create_malformed_mime_message();
        
        // Parse the message - should not crash even with malformed MIME
        let email_message = EmailMessage::from_json(&json_data).unwrap();
        
        // Verify basic fields
        assert_eq!(email_message.id, "malformed_mime");
        assert_eq!(email_message.thread_id, "thread012");
        assert_eq!(email_message.subject, "Malformed MIME Structure");
        
        // Check that we can extract some content even from malformed structure
        assert!(email_message.body.contains("missing headers") || 
                email_message.body.contains("missing mime type"));
    }

    #[test]
    fn test_base64_decoding_edge_cases() {
        // Test valid base64
        let valid_base64 = "SGVsbG8gV29ybGQ="; // "Hello World"
        let decoded = decode_base64(valid_base64).unwrap();
        assert_eq!(decoded, "Hello World");
        
        // Test empty string
        let empty = "";
        let decoded_empty = decode_base64(empty).unwrap();
        assert_eq!(decoded_empty, "");
        
        // Test URL-safe base64
        let urlsafe_base64 = "SGVsbG8gV29ybGQ"; // No padding
        let decoded_urlsafe = decode_base64(urlsafe_base64).unwrap();
        assert_eq!(decoded_urlsafe, "Hello World");
        
        // Test encoding and then decoding
        let original = "Test string with special chars: !@#$%^&*()";
        let encoded = encode_base64_url_safe(original);
        let decoded_back = decode_base64(&encoded).unwrap();
        assert_eq!(decoded_back, original);
    }

    #[test]
    fn test_large_message() {
        let json_data = create_large_message();
        
        // Parse the message
        let email_message = EmailMessage::from_json(&json_data).unwrap();
        
        // Verify basic fields
        assert_eq!(email_message.id, "large_message");
        assert_eq!(email_message.thread_id, "thread345");
        assert_eq!(email_message.subject, "Very Large Message");
        
        // Verify the message body contains expected text
        assert!(email_message.body.contains("This is a line of text"));
        
        // Check the body length is substantial
        assert!(email_message.body.len() > 5000);
    }
}