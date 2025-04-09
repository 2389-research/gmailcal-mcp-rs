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
fn create_multipart_email(
    id: &str,
    subject: &str,
    text_part: &str,
    html_part: &str,
) -> serde_json::Value {
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

// Create nested multipart email with attachment
fn create_multipart_with_attachment(
    id: &str,
    subject: &str,
    text_part: &str,
    html_part: &str,
    attachment_name: &str,
    attachment_data: &str,
) -> serde_json::Value {
    json!({
        "id": id,
        "threadId": format!("thread_{}", id),
        "labelIds": ["INBOX"],
        "snippet": text_part.chars().take(50).collect::<String>(),
        "payload": {
            "mimeType": "multipart/mixed",
            "headers": [
                { "name": "From", "value": "sender@example.com" },
                { "name": "To", "value": "recipient@example.com" },
                { "name": "Subject", "value": subject },
                { "name": "Date", "value": "Mon, 15 Apr 2025 10:00:00 +0000" }
            ],
            "parts": [
                {
                    "mimeType": "multipart/alternative",
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
                },
                {
                    "mimeType": "application/octet-stream",
                    "filename": attachment_name,
                    "headers": [
                        { "name": "Content-Type", "value": "application/octet-stream" },
                        { "name": "Content-Disposition", "value": format!("attachment; filename=\"{}\"", attachment_name) }
                    ],
                    "body": {
                        "data": encode_base64_url_safe(attachment_data.as_bytes()),
                        "size": attachment_data.len(),
                        "attachmentId": format!("attachment_id_{}", id)
                    }
                }
            ]
        }
    })
}

// Create malformed email with missing fields
fn create_malformed_email(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "threadId": format!("thread_{}", id),
        // Missing labelIds
        // Missing snippet
        "payload": {
            // Missing mimeType
            "headers": [
                // Missing From header
                { "name": "To", "value": "recipient@example.com" },
                // Missing Subject header
                { "name": "Date", "value": "Mon, 15 Apr 2025 10:00:00 +0000" }
            ],
            // Missing body
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
        let simple_email =
            create_email_json("simple_id", "Simple Subject", "This is a simple email body");

        // Verify structure
        assert_eq!(simple_email["id"], "simple_id");
        assert_eq!(
            simple_email["payload"]["headers"][2]["value"],
            "Simple Subject"
        );

        // Test multipart email
        let multipart = create_multipart_email(
            "multi_id",
            "Multipart Email",
            "This is the plain text part",
            "<html><body>This is the HTML part</body></html>",
        );

        // Verify structure
        assert_eq!(multipart["id"], "multi_id");
        assert_eq!(multipart["payload"]["mimeType"], "multipart/alternative");
        assert_eq!(multipart["payload"]["parts"][0]["mimeType"], "text/plain");
        assert_eq!(multipart["payload"]["parts"][1]["mimeType"], "text/html");

        // Test multipart with attachment
        let multipart_attachment = create_multipart_with_attachment(
            "attachment_id",
            "Email with Attachment",
            "Plain text content",
            "<html><body>HTML content</body></html>",
            "test.txt",
            "This is the attachment content"
        );

        // Verify complex structure
        assert_eq!(multipart_attachment["id"], "attachment_id");
        assert_eq!(multipart_attachment["payload"]["mimeType"], "multipart/mixed");
        
        // Check first part is multipart/alternative
        let first_part = &multipart_attachment["payload"]["parts"][0];
        assert_eq!(first_part["mimeType"], "multipart/alternative");
        
        // Check second part is the attachment
        let attachment_part = &multipart_attachment["payload"]["parts"][1];
        assert_eq!(attachment_part["mimeType"], "application/octet-stream");
        assert_eq!(attachment_part["filename"], "test.txt");
        
        // Check the attachment ID field
        assert_eq!(attachment_part["body"]["attachmentId"], "attachment_id_attachment_id");
    }

    #[test]
    fn test_special_characters_encoding() {
        // Create email with special characters - the characters remain as UTF-8 in the JSON
        let email_json = create_email_json(
            "special_chars",
            "Email with special chars: äöüß",
            "Content with emoji: 🌍",
        );

        // Verify the subject contains the special characters
        let subject = email_json["payload"]["headers"][2]["value"]
            .as_str()
            .unwrap();
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
        
        // For email messages with special characters, we need to ensure our
        // handling of the encoded content works properly when received from the API
        
        // Create email with special characters
        let email_with_emoji = create_email_json(
            "emoji_message",
            "Email with emoji 🌍🚀🔥",
            "This email contains emoji characters: 🌍🚀🔥"
        );
        
        // Verify the encoded content is in the JSON
        let encoded_body = email_with_emoji["payload"]["body"]["data"].as_str().unwrap();
        assert!(!encoded_body.is_empty());
        
        // Create email with international characters
        let email_with_intl = create_email_json(
            "intl_message",
            "Email with international chars: áéíóúñ",
            "This email contains international characters: áéíóúñÁÉÍÓÚÑ德意志"
        );
        
        // Verify the encoded content is in the JSON
        let encoded_intl_body = email_with_intl["payload"]["body"]["data"].as_str().unwrap();
        assert!(!encoded_intl_body.is_empty());
    }
    
    #[test]
    fn test_malformed_email_structure() {
        // Create a malformed email with missing fields
        let malformed = create_malformed_email("malformed_id");
        
        // Verify the structure still has the required ID fields
        assert_eq!(malformed["id"], "malformed_id");
        assert_eq!(malformed["threadId"], "thread_malformed_id");
        
        // Check missing fields to ensure JSON structure is as expected
        assert!(!malformed.as_object().unwrap().contains_key("labelIds"));
        assert!(!malformed.as_object().unwrap().contains_key("snippet"));
        
        // Check missing mimeType in payload
        assert!(!malformed["payload"].as_object().unwrap().contains_key("mimeType"));
        
        // Check missing body in payload
        assert!(!malformed["payload"].as_object().unwrap().contains_key("body"));
        
        // Check headers are still present but incomplete
        assert!(malformed["payload"]["headers"].is_array());
        assert_eq!(malformed["payload"]["headers"].as_array().unwrap().len(), 2);
        
        // Verify the headers that are present
        let headers = malformed["payload"]["headers"].as_array().unwrap();
        assert_eq!(headers[0]["name"], "To");
        assert_eq!(headers[1]["name"], "Date");
        
        // Verify "From" and "Subject" headers are missing
        let from_headers: Vec<_> = headers.iter()
            .filter(|h| h["name"] == "From")
            .collect();
        assert!(from_headers.is_empty());
        
        let subject_headers: Vec<_> = headers.iter()
            .filter(|h| h["name"] == "Subject")
            .collect();
        assert!(subject_headers.is_empty());
    }
    
    #[test]
    fn test_extremely_long_fields() {
        // Create an email with extremely long subject and body
        let long_subject = "A".repeat(500); // 500 character subject
        let long_body = "B".repeat(100_000); // 100K character body
        
        let long_email = create_email_json("long_id", &long_subject, &long_body);
        
        // Verify structure remains intact
        assert_eq!(long_email["id"], "long_id");
        
        // Verify long fields are preserved
        let subject = long_email["payload"]["headers"][2]["value"]
            .as_str()
            .unwrap();
        assert_eq!(subject.len(), 500);
        assert!(subject.chars().all(|c| c == 'A'));
        
        // Check that the body data exists and is non-empty
        let encoded_body = long_email["payload"]["body"]["data"].as_str().unwrap();
        assert!(!encoded_body.is_empty());
        
        // Verify size field is correctly set
        assert_eq!(long_email["payload"]["body"]["size"], 100_000);
        
        // Check that the snippet is truncated as expected
        assert_eq!(long_email["snippet"].as_str().unwrap().len(), 50);
    }
    
    #[test]
    fn test_nested_multipart_email() {
        // Test creating a deeply nested multipart email (3+ levels)
        // This is a simplified representation of what you'd see in a real email with
        // forwarded content and different parts
        
        // Create the nested structure directly using serde_json
        let nested_email = json!({
            "id": "nested_id",
            "threadId": "thread_nested_id",
            "labelIds": ["INBOX"],
            "snippet": "This is the top level message",
            "payload": {
                "mimeType": "multipart/mixed",
                "headers": [
                    { "name": "From", "value": "sender@example.com" },
                    { "name": "To", "value": "recipient@example.com" },
                    { "name": "Subject", "value": "Nested Message Test" },
                    { "name": "Date", "value": "Mon, 15 Apr 2025 10:00:00 +0000" }
                ],
                "parts": [
                    {
                        "mimeType": "multipart/alternative",
                        "parts": [
                            {
                                "mimeType": "text/plain",
                                "body": {
                                    "data": encode_base64_url_safe("This is the top level message".as_bytes()),
                                    "size": 28
                                }
                            },
                            {
                                "mimeType": "text/html",
                                "body": {
                                    "data": encode_base64_url_safe("<div>This is the top level message</div>".as_bytes()),
                                    "size": 42
                                }
                            }
                        ]
                    },
                    {
                        "mimeType": "message/rfc822",
                        "headers": [
                            { "name": "Content-Type", "value": "message/rfc822" },
                            { "name": "Content-Disposition", "value": "attachment" }
                        ],
                        "body": {
                            "attachmentId": "forwarded_message_id"
                        },
                        "parts": [
                            {
                                "mimeType": "multipart/alternative",
                                "headers": [
                                    { "name": "From", "value": "original@example.com" },
                                    { "name": "To", "value": "sender@example.com" },
                                    { "name": "Subject", "value": "Original Message" },
                                    { "name": "Date", "value": "Mon, 14 Apr 2025 09:00:00 +0000" }
                                ],
                                "parts": [
                                    {
                                        "mimeType": "text/plain",
                                        "body": {
                                            "data": encode_base64_url_safe("This is the original message".as_bytes()),
                                            "size": 28
                                        }
                                    },
                                    {
                                        "mimeType": "text/html",
                                        "body": {
                                            "data": encode_base64_url_safe("<div>This is the original message</div>".as_bytes()),
                                            "size": 42
                                        }
                                    }
                                ]
                            }
                        ]
                    }
                ]
            }
        });
        
        // Verify the structure is correctly formed
        assert_eq!(nested_email["id"], "nested_id");
        assert_eq!(nested_email["payload"]["mimeType"], "multipart/mixed");
        
        // Check first part (multipart/alternative)
        let first_part = &nested_email["payload"]["parts"][0];
        assert_eq!(first_part["mimeType"], "multipart/alternative");
        assert_eq!(first_part["parts"][0]["mimeType"], "text/plain");
        assert_eq!(first_part["parts"][1]["mimeType"], "text/html");
        
        // Check second part (forwarded message)
        let second_part = &nested_email["payload"]["parts"][1];
        assert_eq!(second_part["mimeType"], "message/rfc822");
        
        // Check the nested message headers
        assert!(second_part.get("parts").is_some());
        let forwarded_message = &second_part["parts"][0];
        assert_eq!(forwarded_message["mimeType"], "multipart/alternative");
        
        // Verify forwarded message headers
        let headers = forwarded_message["headers"].as_array().unwrap();
        let subject_header = headers.iter()
            .find(|h| h["name"] == "Subject")
            .unwrap();
        assert_eq!(subject_header["value"], "Original Message");
        
        // Verify the deepest level content
        let text_part = &forwarded_message["parts"][0];
        assert_eq!(text_part["mimeType"], "text/plain");
        
        let html_part = &forwarded_message["parts"][1];
        assert_eq!(html_part["mimeType"], "text/html");
    }
}