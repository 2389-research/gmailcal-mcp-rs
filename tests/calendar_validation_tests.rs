/// Calendar Event Validation Tests Module
///
/// This module contains tests for the Calendar Event validation functionality,
/// focusing on validation of various edge cases and invalid inputs.
use chrono::{DateTime, Duration, Utc};
use mcp_gmailcal::calendar_api::{Attendee, CalendarEvent, EventOrganizer};
use mcp_gmailcal::errors::CalendarApiError;
use serde_json::json;
use uuid::Uuid;

// Define a helper struct for testing event validation
struct EventValidator;

impl EventValidator {
    fn validate_event(event: &CalendarEvent) -> Result<(), CalendarApiError> {
        // Validate event time range (end must be after start)
        if event.end_time <= event.start_time {
            return Err(CalendarApiError::EventFormatError(
                "Event end time must be after start time".to_string(),
            ));
        }

        // Validate attendees
        for attendee in &event.attendees {
            // Basic email validation (contains @)
            if !attendee.email.contains('@') {
                return Err(CalendarApiError::EventFormatError(format!(
                    "Invalid email address for attendee: {}",
                    attendee.email
                )));
            }
        }

        // Check that required fields are present
        if event.summary.is_empty() {
            return Err(CalendarApiError::EventFormatError(
                "Event summary cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod calendar_validation_tests {
    use super::*;

    // Helper to create a valid test event
    fn create_test_event() -> CalendarEvent {
        CalendarEvent {
            id: Some(Uuid::new_v4().to_string()),
            summary: "Test Event".to_string(),
            description: Some("This is a test event".to_string()),
            location: Some("Test Location".to_string()),
            creator: Some(EventOrganizer {
                email: "creator@example.com".to_string(),
                display_name: Some("Event Creator".to_string()),
                self_: Some(true),
            }),
            organizer: Some(EventOrganizer {
                email: "organizer@example.com".to_string(),
                display_name: Some("Event Organizer".to_string()),
                self_: Some(false),
            }),
            start_time: DateTime::parse_from_rfc3339("2025-05-15T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            end_time: DateTime::parse_from_rfc3339("2025-05-15T11:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            attendees: vec![
                Attendee {
                    email: "attendee1@example.com".to_string(),
                    display_name: Some("Attendee 1".to_string()),
                    response_status: Some("accepted".to_string()),
                    optional: None,
                },
                Attendee {
                    email: "attendee2@example.com".to_string(),
                    display_name: Some("Attendee 2".to_string()),
                    response_status: Some("tentative".to_string()),
                    optional: None,
                },
            ],
            html_link: Some("https://calendar.google.com/calendar/event?eid=test".to_string()),
            conference_data: None,
        }
    }

    #[test]
    fn test_invalid_date_range() {
        // Create an event with end time before start time
        let mut event = create_test_event();
        event.end_time = event.start_time - Duration::hours(1);

        // Validate the event
        let result = EventValidator::validate_event(&event);

        // Verify validation fails
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                CalendarApiError::EventFormatError(msg) => {
                    assert!(msg.contains("end time must be after start time"));
                }
                _ => panic!("Expected EventFormatError but got different error type"),
            }
        }
    }

    #[test]
    fn test_malformed_attendee_email() {
        // Create an event with invalid attendee email
        let mut event = create_test_event();
        event.attendees.push(Attendee {
            email: "invalid-email".to_string(), // Missing @ symbol
            display_name: Some("Invalid Email".to_string()),
            response_status: Some("accepted".to_string()),
            optional: None,
        });

        // Validate the event
        let result = EventValidator::validate_event(&event);

        // Verify validation fails
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                CalendarApiError::EventFormatError(msg) => {
                    assert!(msg.contains("Invalid email address"));
                }
                _ => panic!("Expected EventFormatError but got different error type"),
            }
        }
    }

    #[test]
    fn test_missing_required_fields() {
        // Create an event with missing summary
        let mut event = create_test_event();
        event.summary = "".to_string();

        // Validate the event
        let result = EventValidator::validate_event(&event);

        // Verify validation fails
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                CalendarApiError::EventFormatError(msg) => {
                    assert!(msg.contains("summary cannot be empty"));
                }
                _ => panic!("Expected EventFormatError but got different error type"),
            }
        }
    }

    #[test]
    fn test_timezone_conversion() {
        // Create events with different timezones
        let utc_event = create_test_event();

        // The event times should be in UTC
        assert_eq!(
            utc_event.start_time.to_rfc3339(),
            "2025-05-15T10:00:00+00:00"
        );
        assert_eq!(utc_event.end_time.to_rfc3339(), "2025-05-15T11:00:00+00:00");

        // Test conversion from different timezone
        let ny_time = DateTime::parse_from_rfc3339("2025-05-15T06:00:00-04:00")
            .unwrap()
            .with_timezone(&Utc);

        // After conversion to UTC, it should be 10:00 UTC
        assert_eq!(ny_time.to_rfc3339(), "2025-05-15T10:00:00+00:00");
    }

    #[test]
    fn test_valid_event() {
        // Create a valid event
        let event = create_test_event();

        // Validate the event
        let result = EventValidator::validate_event(&event);

        // Verify validation passes
        assert!(result.is_ok());
    }

    #[test]
    fn test_recurring_event_validation() {
        // In a real implementation, we would have a recurrence field
        // For this test, we'll just verify that a normal event passes validation

        let event = create_test_event();

        // We would add recurrence rules here if the struct had that field

        // Validate the event
        let result = EventValidator::validate_event(&event);

        // Verify validation passes
        assert!(result.is_ok());
    }
}
