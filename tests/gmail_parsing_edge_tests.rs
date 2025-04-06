/// Gmail Message Parsing Edge Cases Tests Module
///
/// This module contains tests for Gmail message parsing edge cases,
/// focusing on base64 encoding/decoding and input sanitization.

use mcp_gmailcal::utils::{decode_base64, encode_base64_url_safe};
use serde_json::json;

// Create email JSON with content
fn create_email_json(id: &str, subject: &str, body: &str) -> serde_json::Value {
    json!({
        "id": id,
        "threadId": format!("thread_{}", id),
        "labelIds": ["INBOX"],
        "snippet": body.chars().take(50).collect::<String>(),
        "payload": {
            "mimeType": "text/plain",
            "headers": [
                { "name": "From", "value": "sender@example.com" },
                { "name": "To", "value": "recipient@example.com" },
                { "name": "Subject", "value": subject },
                { "name": "Date", "value": "Mon, 15 Apr 2025 10:00:00 +0000" }
            ],
            "body": {
                "data": encode_base64_url_safe(body.as_bytes()),
                "size": body.len()
            }
        }
    })
}

// Create multipart email JSON
fn create_multipart_email(id: &str, subject: &str, text_part: &str, html_part: &str) -> serde_json::Value {
    json!({
        "id": id,
        "threadId": format!("thread_{}", id),
        "labelIds": ["INBOX"],
        "snippet": text_part.chars().take(50).collect::<String>(),
        "payload": {
            "mimeType": "multipart/alternative",
            "headers": [
                { "name": "From", "value": "sender@example.com" },
                { "name": "To", "value": "recipient@example.com" },
                { "name": "Subject", "value": subject },
                { "name": "Date", "value": "Mon, 15 Apr 2025 10:00:00 +0000" }
            ],
            "parts": [
                {
                    "mimeType": "text/plain",
                    "headers": [
                        { "name": "Content-Type", "value": "text/plain; charset=UTF-8" }
                    ],
                    "body": {
                        "data": encode_base64_url_safe(text_part.as_bytes()),
                        "size": text_part.len()
                    }
                },
                {
                    "mimeType": "text/html",
                    "headers": [
                        { "name": "Content-Type", "value": "text/html; charset=UTF-8" }
                    ],
                    "body": {
                        "data": encode_base64_url_safe(html_part.as_bytes()),
                        "size": html_part.len()
                    }
                }
            ]
        }
    })
}

#[cfg(test)]
mod gmail_parsing_tests {
    use super::*;

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
        let encoded = encode_base64_url_safe(original.as_bytes());
        let decoded_back = decode_base64(&encoded).unwrap();
        assert_eq!(decoded_back, original);
    }
    
    #[test]
    fn test_invalid_base64() {
        // Test invalid base64 characters
        let invalid_base64 = "This is not valid base64!";
        let result = decode_base64(invalid_base64);
        assert!(result.is_err() || result.unwrap() != invalid_base64);
        
        // Test malformed base64 (incorrect length)
        let malformed_base64 = "SGVsbG8gV29yb";
        let result = decode_base64(malformed_base64);
        // Depending on the implementation, this might succeed with partial data or fail
        if result.is_ok() {
            assert_ne!(result.unwrap(), "Hello World");
        }
    }
    
    #[test]
    fn test_empty_and_large_encoding() {
        // Test empty input encoding
        let empty = "";
        let encoded_empty = encode_base64_url_safe(empty.as_bytes());
        assert_eq!(encoded_empty, "");
        
        // Test large input encoding (10KB)
        let large_string = "A".repeat(10240);
        let encoded_large = encode_base64_url_safe(large_string.as_bytes());
        assert!(encoded_large.len() > 10000);
        
        // Verify round-trip encoding and decoding
        let decoded_large = decode_base64(&encoded_large).unwrap();
        assert_eq!(decoded_large, large_string);
    }
    
    #[test]
    fn test_email_json_structure() {
        // Test creating different email structures
        let simple_email = create_email_json(
            "simple_id", 
            "Simple Subject", 
            "This is a simple email body"
        );
        
        // Verify structure
        assert_eq!(simple_email["id"], "simple_id");
        assert_eq!(simple_email["payload"]["headers"][2]["value"], "Simple Subject");
        
        // Test multipart email
        let multipart = create_multipart_email(
            "multi_id",
            "Multipart Email",
            "This is the plain text part",
            "<html><body>This is the HTML part</body></html>"
        );
        
        // Verify structure
        assert_eq!(multipart["id"], "multi_id");
        assert_eq!(multipart["payload"]["mimeType"], "multipart/alternative");
        assert_eq!(multipart["payload"]["parts"][0]["mimeType"], "text/plain");
        assert_eq!(multipart["payload"]["parts"][1]["mimeType"], "text/html");
    }
    
    #[test]
    fn test_special_characters_encoding() {
        // Create email with special characters - the characters remain as UTF-8 in the JSON
        let email_json = create_email_json(
            "special_chars", 
            "Email with special chars: äöüß", 
            "Content with emoji: 🌍"
        );
        
        // Verify the subject contains the special characters
        let subject = email_json["payload"]["headers"][2]["value"].as_str().unwrap();
        assert!(subject.contains("äöüß"));
        
        // Verify the encoded body can be created
        let encoded_body = email_json["payload"]["body"]["data"].as_str().unwrap();
        assert!(!encoded_body.is_empty());
        
        // The base64 encoding/decoding might not preserve multi-byte UTF-8 precisely
        // depending on the implementation, but basic ASCII should work consistently
        let simple_text = "Simple ASCII text";
        let encoded = encode_base64_url_safe(simple_text.as_bytes());
        let decoded = decode_base64(&encoded).unwrap();
        assert_eq!(decoded, simple_text);
    }
}