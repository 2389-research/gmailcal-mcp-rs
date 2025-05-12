/// Property-Based Testing Module
///
/// This module uses property-based testing to verify critical invariants
/// in the code, focusing on encoding/decoding, date/time operations, JSON
/// serialization/deserialization, and email format conversions.
use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use mcp_gmailcal::calendar_api::{Attendee, CalendarEvent};
use mcp_gmailcal::errors::GmailApiError;
use mcp_gmailcal::gmail_api::{DraftEmail, EmailMessage};
use mcp_gmailcal::utils::{
    decode_base64, encode_base64_url_safe,
    error_codes::{get_error_description, get_troubleshooting_steps},
    error_codes::{API_ERROR, AUTH_ERROR, CONFIG_ERROR, GENERAL_ERROR, MESSAGE_FORMAT_ERROR},
    map_gmail_error, parse_max_results, to_mcp_error,
};
use proptest::prelude::*;
use serde_json::{self, json, Value};
use std::collections::HashSet;

/// Strategies for generating test data

// Strategy for generating valid base64 content
fn base64_strategy() -> impl Strategy<Value = String> {
    // Generate bytes that will become valid base64 when encoded
    prop::collection::vec(any::<u8>(), 0..100).prop_map(|bytes| {
        // Convert to base64 and then to string
        base64::encode(&bytes)
    })
}

// Strategy for generating valid UTF-8 strings
fn utf8_string_strategy() -> impl Strategy<Value = String> {
    // Generate strings with common characters and some special ones
    prop::string::string_regex("[a-zA-Z0-9 !@#$%^&*()_+\\-=\\[\\]{};':\",./<>?\\\\|àéîøü]{1,100}")
        .unwrap()
}

// Strategy for generating realistic email messages
fn email_message_strategy() -> impl Strategy<Value = EmailMessage> {
    (
        // ID and thread ID are typically base64url-like strings
        "[a-zA-Z0-9_-]{6,22}".prop_map(String::from),
        "[a-zA-Z0-9_-]{6,22}".prop_map(String::from),
        // Subject, from, to could be none or some string
        proptest::option::weighted(0.9, utf8_string_strategy()),
        proptest::option::weighted(
            0.9,
            "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,6}".prop_map(String::from),
        ),
        proptest::option::weighted(
            0.9,
            "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,6}".prop_map(String::from),
        ),
        // Date in ISO format (optional)
        proptest::option::weighted(
            0.9,
            (
                1970i32..2030,
                1u32..13,
                1u32..28,
                0u32..24,
                0u32..60,
                0u32..60,
            )
                .prop_map(|(year, month, day, hour, min, sec)| {
                    Utc.with_ymd_and_hms(year, month, day, hour, min, sec)
                        .unwrap()
                        .to_rfc3339()
                }),
        ),
        // Snippet is a short preview of the email
        proptest::option::weighted(0.9, utf8_string_strategy()),
        // Body text and body HTML (optional)
        proptest::option::weighted(0.8, utf8_string_strategy()),
        proptest::option::weighted(
            0.7,
            utf8_string_strategy().prop_map(|s| format!("<div>{}</div>", s)),
        ),
    )
        .prop_map(
            |(id, thread_id, subject, from, to, date, snippet, body_text, body_html)| {
                EmailMessage {
                    id,
                    thread_id,
                    subject,
                    from,
                    to,
                    date,
                    snippet,
                    body_text,
                    body_html,
                }
            },
        )
}

// Strategy for generating draft emails
fn draft_email_strategy() -> impl Strategy<Value = DraftEmail> {
    (
        // Required fields: to, subject, body
        "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,6}".prop_map(String::from),
        utf8_string_strategy(),
        utf8_string_strategy(),
        // Optional fields
        proptest::option::weighted(
            0.7,
            "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,6}".prop_map(String::from),
        ),
        proptest::option::weighted(
            0.5,
            "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,6}".prop_map(String::from),
        ),
        // Thread ID is base64url-like
        proptest::option::weighted(0.5, "[a-zA-Z0-9_-]{6,22}".prop_map(String::from)),
        // Message IDs for replies look like <...@...>
        proptest::option::weighted(
            0.4,
            (
                "[a-zA-Z0-9]{10,20}".prop_map(String::from),
                "[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,6}".prop_map(String::from),
            )
                .prop_map(|(id, domain)| format!("<{}@{}>", id, domain)),
        ),
        // References are a list of message IDs
        proptest::option::weighted(
            0.3,
            prop::collection::vec(
                (
                    "[a-zA-Z0-9]{10,20}".prop_map(String::from),
                    "[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,6}".prop_map(String::from),
                )
                    .prop_map(|(id, domain)| format!("<{}@{}>", id, domain)),
                1..5,
            )
            .prop_map(|refs| refs.join(" ")),
        ),
    )
        .prop_map(
            |(to, subject, body, cc, bcc, thread_id, in_reply_to, references)| DraftEmail {
                to,
                subject,
                body,
                cc,
                bcc,
                thread_id,
                in_reply_to,
                references,
            },
        )
}

// Strategy for dates in a reasonable range
fn date_strategy() -> impl Strategy<Value = DateTime<Utc>> {
    (
        1970i32..2030,
        1u32..13,
        1u32..28,
        0u32..24,
        0u32..60,
        0u32..60,
    )
        .prop_map(|(year, month, day, hour, min, sec)| {
            Utc.with_ymd_and_hms(year, month, day, hour, min, sec)
                .unwrap()
        })
}

// Strategy for generating calendar events
fn calendar_event_strategy() -> impl Strategy<Value = CalendarEvent> {
    (
        // ID is optional
        proptest::option::weighted(0.8, "[a-zA-Z0-9_-]{6,22}".prop_map(String::from)),
        // Summary is required
        utf8_string_strategy(),
        // Description and location are optional
        proptest::option::weighted(0.7, utf8_string_strategy()),
        proptest::option::weighted(0.7, utf8_string_strategy()),
        // Start and end times (ensure end is after start)
        date_strategy(),
        prop::collection::vec(
            (
                "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,6}".prop_map(String::from),
                proptest::option::weighted(0.7, utf8_string_strategy()),
                proptest::option::weighted(
                    0.7,
                    proptest::string::string_regex("(needsAction|declined|tentative|accepted)")
                        .unwrap(),
                ),
                proptest::option::weighted(0.3, prop::bool::ANY),
            )
                .prop_map(|(email, display_name, response_status, optional)| {
                    Attendee {
                        email,
                        display_name,
                        response_status,
                        optional,
                    }
                }),
            0..5,
        ),
    )
        .prop_flat_map(
            |(id, summary, description, location, start_time, attendees)| {
                // Generate an end_time that is always after start_time
                (
                    Just(id),
                    Just(summary),
                    Just(description),
                    Just(location),
                    Just(start_time),
                    (1i64..100).prop_map(move |hours_to_add| {
                        start_time + chrono::Duration::hours(hours_to_add)
                    }),
                    Just(attendees),
                )
            },
        )
        .prop_map(
            |(id, summary, description, location, start_time, end_time, attendees)| {
                CalendarEvent {
                    id,
                    summary,
                    description,
                    location,
                    start_time,
                    end_time,
                    attendees,
                    conference_data: None, // Simplify by not testing conference data
                    html_link: None,
                    creator: None,
                    organizer: None,
                }
            },
        )
}

// Strategy for JSON values for testing parse_max_results
fn max_results_values_strategy() -> impl Strategy<Value = Value> {
    prop_oneof![
        // Number as a JSON number
        (1u32..100).prop_map(|n| json!(n)),
        // Number as a JSON string
        (1u32..100).prop_map(|n| json!(n.to_string())),
        // Invalid types
        Just(json!(null)),
        Just(json!(true)),
        Just(json!(false)),
        Just(json!([])),
        Just(json!({}))
    ]
}

// Property tests
proptest! {
    // Test that encoding and then decoding base64 preserves the original data
    #[test]
    fn test_base64_roundtrip(data: Vec<u8>) {
        // Convert data to URL-safe base64
        let encoded = encode_base64_url_safe(&data);

        // The URL-safe encoding uses different characters, so we need to use
        // the appropriate decode function (URL_SAFE config)
        let decoded = match base64::decode_config(&encoded, base64::URL_SAFE) {
            Ok(d) => d,
            Err(e) => {
                // If there's an error, we'll print debug info and fail gracefully
                prop_assert!(
                    false,
                    "Failed to decode base64 string '{}': {}",
                    encoded, e
                );
                return Ok(());
            }
        };

        // Verify the roundtrip preserves the original data
        prop_assert_eq!(&data, &decoded);
    }

    // Test that decode_base64 properly decodes valid standard base64 strings
    // (not URL safe, since decode_base64 uses standard base64)
    #[test]
    fn test_decode_base64_with_valid_input(s in "A{0,20}") {
        // Create a valid standard base64 string
        let valid_base64 = base64::encode(s.as_bytes());

        // Test that our function can decode it
        let result = decode_base64(&valid_base64);
        prop_assert!(result.is_ok());
    }

    // Test parse_max_results with various input types
    #[test]
    fn test_parse_max_results_property(value in max_results_values_strategy(), default in 1u32..100) {
        // The function should never panic regardless of input
        let result = parse_max_results(Some(value.clone()), default);

        // If input is a valid number or string representation of a number, result should match that number
        match value {
            Value::Number(n) if n.is_u64() && n.as_u64().unwrap() <= u32::MAX as u64 => {
                prop_assert_eq!(result, n.as_u64().unwrap() as u32);
            },
            Value::String(s) if s.parse::<u32>().is_ok() => {
                prop_assert_eq!(result, s.parse::<u32>().unwrap());
            },
            // Otherwise should use the default
            _ => prop_assert_eq!(result, default),
        }
    }

    // Test that JSON serialization and deserialization of EmailMessage works correctly
    #[test]
    fn test_email_message_serialization(email in email_message_strategy()) {
        // Serialize to JSON
        let json = serde_json::to_string(&email).unwrap();

        // Deserialize back to EmailMessage
        let deserialized: EmailMessage = serde_json::from_str(&json).unwrap();

        // Verify properties are preserved
        prop_assert_eq!(email.id, deserialized.id);
        prop_assert_eq!(email.thread_id, deserialized.thread_id);
        prop_assert_eq!(email.subject, deserialized.subject);
        prop_assert_eq!(email.from, deserialized.from);
        prop_assert_eq!(email.to, deserialized.to);
        prop_assert_eq!(email.date, deserialized.date);
        prop_assert_eq!(email.snippet, deserialized.snippet);
        prop_assert_eq!(email.body_text, deserialized.body_text);
        prop_assert_eq!(email.body_html, deserialized.body_html);
    }

    // Test that JSON serialization and deserialization of DraftEmail works correctly
    #[test]
    fn test_draft_email_serialization(draft in draft_email_strategy()) {
        // Serialize to JSON
        let json = serde_json::to_string(&draft).unwrap();

        // Deserialize back to DraftEmail
        let deserialized: DraftEmail = serde_json::from_str(&json).unwrap();

        // Verify properties are preserved
        prop_assert_eq!(draft.to, deserialized.to);
        prop_assert_eq!(draft.subject, deserialized.subject);
        prop_assert_eq!(draft.body, deserialized.body);
        prop_assert_eq!(draft.cc, deserialized.cc);
        prop_assert_eq!(draft.bcc, deserialized.bcc);
        prop_assert_eq!(draft.thread_id, deserialized.thread_id);
        prop_assert_eq!(draft.in_reply_to, deserialized.in_reply_to);
        prop_assert_eq!(draft.references, deserialized.references);
    }

    // Test that DateTimes serialize and deserialize correctly
    #[test]
    fn test_datetime_serialization(dt in date_strategy()) {
        // Serialize to string (RFC3339 format)
        let date_str = dt.to_rfc3339();

        // Parse back to DateTime
        let parsed = DateTime::parse_from_rfc3339(&date_str).unwrap().with_timezone(&Utc);

        // Verify they match
        prop_assert_eq!(dt, parsed);
    }

    // Test that CalendarEvent serialization works correctly
    #[test]
    fn test_calendar_event_serialization(event in calendar_event_strategy()) {
        // Serialize to JSON
        let json = serde_json::to_string(&event).unwrap();

        // Deserialize back to CalendarEvent
        let deserialized: CalendarEvent = serde_json::from_str(&json).unwrap();

        // Verify core properties are preserved
        prop_assert_eq!(event.id, deserialized.id);
        prop_assert_eq!(event.summary, deserialized.summary);
        prop_assert_eq!(event.description, deserialized.description);
        prop_assert_eq!(event.location, deserialized.location);
        prop_assert_eq!(event.start_time, deserialized.start_time);
        prop_assert_eq!(event.end_time, deserialized.end_time);

        // Verify attendees count matches
        prop_assert_eq!(event.attendees.len(), deserialized.attendees.len());

        // Verify each attendee
        for (original, deserialized_attendee) in event.attendees.iter().zip(deserialized.attendees.iter()) {
            prop_assert_eq!(&original.email, &deserialized_attendee.email);
            prop_assert_eq!(&original.display_name, &deserialized_attendee.display_name);
            prop_assert_eq!(&original.response_status, &deserialized_attendee.response_status);
            prop_assert_eq!(&original.optional, &deserialized_attendee.optional);
        }
    }

    // Test invariants for calendar events
    #[test]
    fn test_calendar_event_invariants(event in calendar_event_strategy()) {
        // End time should always be after start time
        prop_assert!(event.end_time > event.start_time,
            "End time {:?} should be after start time {:?}", event.end_time, event.start_time);

        // Each attendee email should be unique
        let mut emails = HashSet::new();
        for attendee in &event.attendees {
            prop_assert!(emails.insert(attendee.email.clone()),
                "Duplicate attendee email: {}", attendee.email);
        }
    }
}

// Use macro to generate a test for each JSON value type
macro_rules! test_json_value_parsing {
    ($name:ident, $value:expr, $expected:expr, $default:expr) => {
        #[test]
        fn $name() {
            let value = $value;
            let result = parse_max_results(Some(value), $default);
            assert_eq!(result, $expected);
        }
    };
}

// Test specific edge cases with fixed values
test_json_value_parsing!(test_parse_max_results_with_number, json!(10), 10, 5);
test_json_value_parsing!(test_parse_max_results_with_string, json!("20"), 20, 5);
test_json_value_parsing!(test_parse_max_results_with_null, json!(null), 5, 5);
test_json_value_parsing!(test_parse_max_results_with_boolean, json!(true), 5, 5);
test_json_value_parsing!(test_parse_max_results_with_array, json!([1, 2, 3]), 5, 5);
test_json_value_parsing!(
    test_parse_max_results_with_object,
    json!({"value": 30}),
    5,
    5
);
test_json_value_parsing!(
    test_parse_max_results_with_invalid_string,
    json!("not a number"),
    5,
    5
);
test_json_value_parsing!(test_parse_max_results_with_float, json!(15.7), 5, 5); // Floats don't convert to u32
test_json_value_parsing!(
    test_parse_max_results_with_large_number,
    json!(4_294_967_295_i64 + 1),
    5,
    5
); // > u32::MAX
test_json_value_parsing!(test_parse_max_results_with_negative, json!(-5), 5, 5);

// Additional focused tests for base64 encoding/decoding
#[test]
fn test_base64_encoding_special_cases() {
    // Test empty data
    let empty: Vec<u8> = vec![];
    let encoded = encode_base64_url_safe(&empty);
    assert_eq!(encoded, "");

    // Test decoding empty string
    let result = decode_base64("");
    assert_eq!(result.unwrap(), "");

    // Test data with special URL characters (+, /)
    let special_chars = "Hello+World/Special_Chars".as_bytes().to_vec();
    let encoded = encode_base64_url_safe(&special_chars);
    assert!(!encoded.contains('+'));
    assert!(!encoded.contains('/'));

    // Test decoding with URL-safe characters
    let decoded = decode_base64(&encoded).unwrap();
    assert_eq!(decoded.as_bytes(), special_chars);

    // Test invalid base64 input (not a multiple of 4)
    let invalid = "abc";
    let result = decode_base64(invalid);
    assert!(result.is_err());

    // Test invalid base64 input (invalid characters)
    let invalid = "a$c=";
    let result = decode_base64(invalid);
    assert!(result.is_err());
}

// Test email format conversion invariants
#[test]
fn test_email_message_invariants() {
    // Create a valid email message
    let email = EmailMessage {
        id: "msg123".to_string(),
        thread_id: "thread456".to_string(),
        subject: Some("Test Email".to_string()),
        from: Some("sender@example.com".to_string()),
        to: Some("recipient@example.com".to_string()),
        date: Some("2023-05-15T10:00:00Z".to_string()),
        snippet: Some("This is a test email...".to_string()),
        body_text: Some("This is the plain text body.".to_string()),
        body_html: Some("<div>This is the HTML body.</div>".to_string()),
    };

    // Serialize to JSON
    let json = serde_json::to_string(&email).unwrap();

    // Deserialize back
    let deserialized: EmailMessage = serde_json::from_str(&json).unwrap();

    // Required fields should never be empty
    assert!(!deserialized.id.is_empty());
    assert!(!deserialized.thread_id.is_empty());

    // Optional fields should match exactly
    assert_eq!(email.subject, deserialized.subject);
    assert_eq!(email.from, deserialized.from);
    assert_eq!(email.to, deserialized.to);
    assert_eq!(email.date, deserialized.date);
    assert_eq!(email.snippet, deserialized.snippet);
    assert_eq!(email.body_text, deserialized.body_text);
    assert_eq!(email.body_html, deserialized.body_html);
}

// Test date/time parsing edge cases
#[test]
fn test_datetime_parsing_edge_cases() {
    // Test parsing various date formats
    let formats = [
        "2023-05-15T10:00:00Z",
        "2023-05-15T10:00:00+00:00",
        "2023-05-15T10:00:00.123Z",
        "2023-05-15T10:00:00.123456Z",
    ];

    for format in formats {
        let parsed = DateTime::parse_from_rfc3339(format).unwrap();
        let utc = parsed.with_timezone(&Utc);
        assert_eq!(utc.year(), 2023);
        assert_eq!(utc.month(), 5);
        assert_eq!(utc.day(), 15);
        assert_eq!(utc.hour(), 10);
        assert_eq!(utc.minute(), 0);
        assert_eq!(utc.second(), 0);
    }

    // Test with different time zones
    let with_timezone = "2023-05-15T10:00:00+05:00"; // UTC+5
    let parsed = DateTime::parse_from_rfc3339(with_timezone).unwrap();
    let utc = parsed.with_timezone(&Utc);

    // This should be 5 hours earlier in UTC
    assert_eq!(utc.hour(), 5);
}

/// Generate arbitrary error messages
fn error_message_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9 ]{0,50}").unwrap()
}

// Add additional property tests for utils module
proptest! {
    // Test that error codes always have descriptions and troubleshooting steps
    #[test]
    fn test_error_descriptions_for_all_codes(code in 0u32..10000u32) {
        // Every code should return a description
        let desc = get_error_description(code);
        prop_assert!(!desc.is_empty());

        // Every code should return troubleshooting steps
        let steps = get_troubleshooting_steps(code);
        prop_assert!(!steps.is_empty());
    }

    // Test to_mcp_error with arbitrary inputs
    #[test]
    fn test_to_mcp_error_with_arbitrary_inputs(message in error_message_strategy(), code in 1000u32..1010u32) {
        // The to_mcp_error function should work with any message and code
        let error = to_mcp_error(&message, code);
        // Check that the error was created (we can't check much else since we can't access the internals)
        let debug_str = format!("{:?}", error);
        prop_assert!(debug_str.contains(&code.to_string()));
    }

    // Test map_gmail_error with arbitrary inputs
    #[test]
    fn test_map_gmail_error_with_arbitrary_inputs(message in error_message_strategy()) {
        // Test all enum variants with arbitrary messages
        let api_error = map_gmail_error(GmailApiError::ApiError(message.clone()));
        let debug_str = format!("{:?}", api_error);

        let auth_error = map_gmail_error(GmailApiError::AuthError(message.clone()));
        let debug_str2 = format!("{:?}", auth_error);

        let retrieval_error = map_gmail_error(GmailApiError::MessageRetrievalError(message.clone()));
        let debug_str3 = format!("{:?}", retrieval_error);

        let format_error = map_gmail_error(GmailApiError::MessageFormatError(message.clone()));
        let debug_str4 = format!("{:?}", format_error);

        let network_error = map_gmail_error(GmailApiError::NetworkError(message.clone()));
        let debug_str5 = format!("{:?}", network_error);

        let rate_limit_error = map_gmail_error(GmailApiError::RateLimitError(message.clone()));
        let debug_str6 = format!("{:?}", rate_limit_error);

        // Make sure all errors are mapped to some error code
        prop_assert!(debug_str.contains(&API_ERROR.to_string())
            || debug_str.contains(&AUTH_ERROR.to_string())
            || debug_str.contains(&MESSAGE_FORMAT_ERROR.to_string()));

        prop_assert!(debug_str2.contains(&AUTH_ERROR.to_string()));
        prop_assert!(debug_str3.contains(&API_ERROR.to_string()));
        prop_assert!(debug_str4.contains(&MESSAGE_FORMAT_ERROR.to_string()));
        prop_assert!(debug_str5.contains(&API_ERROR.to_string()));
        prop_assert!(debug_str6.contains(&API_ERROR.to_string()));
    }

    // Test the ApiError message classification
    #[test]
    fn test_api_error_message_classification(message in error_message_strategy()) {
        // Test different keywords in the message to ensure classification works
        let error1 = map_gmail_error(GmailApiError::ApiError(format!("quota {}", message)));
        let debug_str1 = format!("{:?}", error1);

        let error2 = map_gmail_error(GmailApiError::ApiError(format!("rate {}", message)));
        let debug_str2 = format!("{:?}", error2);

        let error3 = map_gmail_error(GmailApiError::ApiError(format!("limit {}", message)));
        let debug_str3 = format!("{:?}", error3);

        let error4 = map_gmail_error(GmailApiError::ApiError(format!("network {}", message)));
        let debug_str4 = format!("{:?}", error4);

        let error5 = map_gmail_error(GmailApiError::ApiError(format!("connection {}", message)));
        let debug_str5 = format!("{:?}", error5);

        let error6 = map_gmail_error(GmailApiError::ApiError(format!("timeout {}", message)));
        let debug_str6 = format!("{:?}", error6);

        let error7 = map_gmail_error(GmailApiError::ApiError(format!("authentication {}", message)));
        let debug_str7 = format!("{:?}", error7);

        let error8 = map_gmail_error(GmailApiError::ApiError(format!("auth {}", message)));
        let debug_str8 = format!("{:?}", error8);

        let error9 = map_gmail_error(GmailApiError::ApiError(format!("token {}", message)));
        let debug_str9 = format!("{:?}", error9);

        let error10 = map_gmail_error(GmailApiError::ApiError(format!("format {}", message)));
        let debug_str10 = format!("{:?}", error10);

        let error11 = map_gmail_error(GmailApiError::ApiError(format!("missing field {}", message)));
        let debug_str11 = format!("{:?}", error11);

        let error12 = map_gmail_error(GmailApiError::ApiError(format!("parse {}", message)));
        let debug_str12 = format!("{:?}", error12);

        let error13 = map_gmail_error(GmailApiError::ApiError(format!("not found {}", message)));
        let debug_str13 = format!("{:?}", error13);

        let error14 = map_gmail_error(GmailApiError::ApiError(format!("404 {}", message)));
        let debug_str14 = format!("{:?}", error14);

        // Verify the error code mappings
        prop_assert!(debug_str1.contains(&API_ERROR.to_string()));
        prop_assert!(debug_str2.contains(&API_ERROR.to_string()));
        prop_assert!(debug_str3.contains(&API_ERROR.to_string()));
        prop_assert!(debug_str4.contains(&API_ERROR.to_string()));
        prop_assert!(debug_str5.contains(&API_ERROR.to_string()));
        prop_assert!(debug_str6.contains(&API_ERROR.to_string()));
        prop_assert!(debug_str7.contains(&AUTH_ERROR.to_string()));
        prop_assert!(debug_str8.contains(&AUTH_ERROR.to_string()));
        prop_assert!(debug_str9.contains(&AUTH_ERROR.to_string()));
        prop_assert!(debug_str10.contains(&MESSAGE_FORMAT_ERROR.to_string()));
        prop_assert!(debug_str11.contains(&MESSAGE_FORMAT_ERROR.to_string()));
        prop_assert!(debug_str12.contains(&MESSAGE_FORMAT_ERROR.to_string()));
        prop_assert!(debug_str13.contains(&API_ERROR.to_string()));
        prop_assert!(debug_str14.contains(&API_ERROR.to_string()));
    }
}
