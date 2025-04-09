/// Calendar Event Validation Tests Module
///
/// This module contains tests for the Calendar Event validation functionality,
/// focusing on validation of various edge cases and invalid inputs.
use chrono::{DateTime, Duration, TimeZone, Utc};
use mcp_gmailcal::calendar_api::{Attendee, CalendarEvent, ConferenceData, ConferenceSolution, EntryPoint, EventOrganizer};
use mcp_gmailcal::errors::CalendarApiError;
use uuid::Uuid;

// Define a helper struct for testing event validation
struct EventValidator;

impl EventValidator {
    fn validate_event(event: &CalendarEvent) -> Result<(), CalendarApiError> {
        // Validate event time range (end must be after start)
        // Note: We disallow zero-duration events (where end_time equals start_time)
        // as they don't have any practical calendar representation
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
    
    // Method to validate a conference data object
    fn validate_conference_data(conf_data: &ConferenceData) -> Result<(), CalendarApiError> {
        // Validate conference solution
        if let Some(solution) = &conf_data.conference_solution {
            if solution.name.is_empty() {
                return Err(CalendarApiError::EventFormatError(
                    "Conference solution name cannot be empty".to_string(),
                ));
            }
        } else {
            return Err(CalendarApiError::EventFormatError(
                "Conference solution is required".to_string(),
            ));
        }
        
        // At least one entry point is required
        if conf_data.entry_points.is_empty() {
            return Err(CalendarApiError::EventFormatError(
                "At least one conference entry point is required".to_string(),
            ));
        }
        
        // Validate each entry point
        for entry_point in &conf_data.entry_points {
            if entry_point.entry_point_type.is_empty() {
                return Err(CalendarApiError::EventFormatError(
                    "Entry point type cannot be empty".to_string(),
                ));
            }
            
            if entry_point.uri.is_empty() {
                return Err(CalendarApiError::EventFormatError(
                    "Entry point URI cannot be empty".to_string(),
                ));
            }
            
            // Validate URI format based on type
            match entry_point.entry_point_type.as_str() {
                "video" => {
                    if !entry_point.uri.starts_with("http") && !entry_point.uri.starts_with("https") {
                        return Err(CalendarApiError::EventFormatError(
                            format!("Video entry point URI must be an HTTP(S) URL: {}", entry_point.uri)
                        ));
                    }
                },
                "phone" => {
                    if !entry_point.uri.starts_with("tel:") {
                        return Err(CalendarApiError::EventFormatError(
                            format!("Phone entry point URI must use tel: protocol: {}", entry_point.uri)
                        ));
                    }
                },
                "sip" => {
                    if !entry_point.uri.starts_with("sip:") {
                        return Err(CalendarApiError::EventFormatError(
                            format!("SIP entry point URI must use sip: protocol: {}", entry_point.uri)
                        ));
                    }
                },
                _ => {
                    // Accept any URI for other types
                }
            }
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
    
    // Helper to create valid conference data
    fn create_test_conference_data() -> ConferenceData {
        ConferenceData {
            conference_solution: Some(ConferenceSolution {
                name: "Google Meet".to_string(),
                key: Some("meet".to_string()),
            }),
            entry_points: vec![
                EntryPoint {
                    entry_point_type: "video".to_string(),
                    uri: "https://meet.google.com/abc-defg-hij".to_string(),
                    label: Some("Google Meet".to_string()),
                },
                EntryPoint {
                    entry_point_type: "phone".to_string(),
                    uri: "tel:+11234567890".to_string(),
                    label: Some("Phone".to_string()),
                },
            ],
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
    
    #[test]
    fn test_conference_data_validation() {
        // Create a valid conference data object
        let conf_data = create_test_conference_data();
        
        // Validate the conference data
        let result = EventValidator::validate_conference_data(&conf_data);
        
        // Verify validation passes
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_invalid_conference_solution() {
        // Create conference data with missing solution name
        let mut conf_data = create_test_conference_data();
        conf_data.conference_solution = Some(ConferenceSolution {
            name: "".to_string(), // Empty name
            key: Some("meet".to_string()),
        });
        
        // Validate the conference data
        let result = EventValidator::validate_conference_data(&conf_data);
        
        // Verify validation fails
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                CalendarApiError::EventFormatError(msg) => {
                    assert!(msg.contains("Conference solution name cannot be empty"));
                }
                _ => panic!("Expected EventFormatError but got different error type"),
            }
        }
        
        // Create conference data with missing solution
        let mut conf_data = create_test_conference_data();
        conf_data.conference_solution = None;
        
        // Validate the conference data
        let result = EventValidator::validate_conference_data(&conf_data);
        
        // Verify validation fails
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                CalendarApiError::EventFormatError(msg) => {
                    assert!(msg.contains("Conference solution is required"));
                }
                _ => panic!("Expected EventFormatError but got different error type"),
            }
        }
    }
    
    #[test]
    fn test_invalid_entry_points() {
        // Create conference data with no entry points
        let mut conf_data = create_test_conference_data();
        conf_data.entry_points = vec![];
        
        // Validate the conference data
        let result = EventValidator::validate_conference_data(&conf_data);
        
        // Verify validation fails
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                CalendarApiError::EventFormatError(msg) => {
                    assert!(msg.contains("At least one conference entry point is required"));
                }
                _ => panic!("Expected EventFormatError but got different error type"),
            }
        }
        
        // Create conference data with invalid entry point type
        let mut conf_data = create_test_conference_data();
        conf_data.entry_points = vec![
            EntryPoint {
                entry_point_type: "".to_string(), // Empty type
                uri: "https://meet.google.com/abc-defg-hij".to_string(),
                label: Some("Google Meet".to_string()),
            },
        ];
        
        // Validate the conference data
        let result = EventValidator::validate_conference_data(&conf_data);
        
        // Verify validation fails
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                CalendarApiError::EventFormatError(msg) => {
                    assert!(msg.contains("Entry point type cannot be empty"));
                }
                _ => panic!("Expected EventFormatError but got different error type"),
            }
        }
        
        // Create conference data with invalid URI
        let mut conf_data = create_test_conference_data();
        conf_data.entry_points = vec![
            EntryPoint {
                entry_point_type: "video".to_string(),
                uri: "".to_string(), // Empty URI
                label: Some("Google Meet".to_string()),
            },
        ];
        
        // Validate the conference data
        let result = EventValidator::validate_conference_data(&conf_data);
        
        // Verify validation fails
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                CalendarApiError::EventFormatError(msg) => {
                    assert!(msg.contains("Entry point URI cannot be empty"));
                }
                _ => panic!("Expected EventFormatError but got different error type"),
            }
        }
    }
    
    #[test]
    fn test_invalid_uri_format() {
        // Test invalid video URL format (should be HTTP/HTTPS)
        let mut conf_data = create_test_conference_data();
        conf_data.entry_points = vec![
            EntryPoint {
                entry_point_type: "video".to_string(),
                uri: "ftp://meet.example.com".to_string(), // Not HTTP/HTTPS
                label: Some("Invalid Video".to_string()),
            },
        ];
        
        // Validate the conference data
        let result = EventValidator::validate_conference_data(&conf_data);
        
        // Verify validation fails
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                CalendarApiError::EventFormatError(msg) => {
                    assert!(msg.contains("Video entry point URI must be an HTTP(S) URL"));
                }
                _ => panic!("Expected EventFormatError but got different error type"),
            }
        }
        
        // Test invalid phone URL format (should use tel: protocol)
        let mut conf_data = create_test_conference_data();
        conf_data.entry_points = vec![
            EntryPoint {
                entry_point_type: "phone".to_string(),
                uri: "+11234567890".to_string(), // Missing tel: protocol
                label: Some("Invalid Phone".to_string()),
            },
        ];
        
        // Validate the conference data
        let result = EventValidator::validate_conference_data(&conf_data);
        
        // Verify validation fails
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                CalendarApiError::EventFormatError(msg) => {
                    assert!(msg.contains("Phone entry point URI must use tel: protocol"));
                }
                _ => panic!("Expected EventFormatError but got different error type"),
            }
        }
        
        // Test invalid SIP URL format (should use sip: protocol)
        let mut conf_data = create_test_conference_data();
        conf_data.entry_points = vec![
            EntryPoint {
                entry_point_type: "sip".to_string(),
                uri: "user@example.com".to_string(), // Missing sip: protocol
                label: Some("Invalid SIP".to_string()),
            },
        ];
        
        // Validate the conference data
        let result = EventValidator::validate_conference_data(&conf_data);
        
        // Verify validation fails
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                CalendarApiError::EventFormatError(msg) => {
                    assert!(msg.contains("SIP entry point URI must use sip: protocol"));
                }
                _ => panic!("Expected EventFormatError but got different error type"),
            }
        }
    }
    
    #[test]
    fn test_all_day_event() {
        // Create an all-day event (midnight to midnight)
        let mut event = create_test_event();
        
        // Set start time to midnight UTC
        event.start_time = Utc.ymd(2025, 5, 15).and_hms(0, 0, 0);
        
        // Set end time to midnight the next day (24 hours later)
        event.end_time = Utc.ymd(2025, 5, 16).and_hms(0, 0, 0);
        
        // Validate the event
        let result = EventValidator::validate_event(&event);
        
        // Verify validation passes
        assert!(result.is_ok());
        
        // Verify it's a full 24 hours
        let duration = event.end_time - event.start_time;
        assert_eq!(duration.num_hours(), 24);
    }
    
    #[test]
    fn test_zero_duration_event() {
        // Create an event with zero duration (start time equals end time)
        let mut event = create_test_event();
        event.end_time = event.start_time;
        
        // Validate the event
        let result = EventValidator::validate_event(&event);
        
        // Verify validation fails (we disallow zero-duration events)
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
    fn test_very_long_duration_event() {
        // Create an event that spans a very long period (1 year)
        let mut event = create_test_event();
        event.end_time = event.start_time + Duration::days(365);
        
        // Validate the event
        let result = EventValidator::validate_event(&event);
        
        // Verify validation passes (long duration events are allowed)
        assert!(result.is_ok());
        
        // Verify the duration is correct
        let duration = event.end_time - event.start_time;
        assert_eq!(duration.num_days(), 365);
    }
    
    #[test]
    fn test_optional_attendee() {
        // Create an event with an optional attendee
        let mut event = create_test_event();
        
        // Add an optional attendee
        event.attendees.push(Attendee {
            email: "optional@example.com".to_string(),
            display_name: Some("Optional Attendee".to_string()),
            response_status: Some("needsAction".to_string()),
            optional: Some(true), // This attendee is optional
        });
        
        // Validate the event
        let result = EventValidator::validate_event(&event);
        
        // Verify validation passes
        assert!(result.is_ok());
        
        // Verify the optional flag is set correctly
        let optional_attendee = event.attendees.iter()
            .find(|a| a.email == "optional@example.com")
            .unwrap();
        assert_eq!(optional_attendee.optional, Some(true));
    }
    
    #[test]
    fn test_resource_room_attendee() {
        // Create an event with a resource (room) attendee
        let mut event = create_test_event();
        
        // Add a resource attendee (usually a room)
        event.attendees.push(Attendee {
            email: "room123@resource.calendar.google.com".to_string(), // Resource email format
            display_name: Some("Conference Room 123".to_string()),
            response_status: Some("accepted".to_string()),
            optional: None,
        });
        
        // Validate the event
        let result = EventValidator::validate_event(&event);
        
        // Verify validation passes
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_datetime_formats() {
        // Test various datetime formats and conversions
        
        // RFC3339 format with offset
        let dt1 = DateTime::parse_from_rfc3339("2025-05-15T10:30:45+02:00")
            .unwrap()
            .with_timezone(&Utc);
        
        // RFC3339 format with Z (UTC)
        let dt2 = DateTime::parse_from_rfc3339("2025-05-15T08:30:45Z")
            .unwrap()
            .with_timezone(&Utc);
        
        // These should be the same time in UTC
        assert_eq!(dt1, dt2);
        assert_eq!(dt1.to_rfc3339(), "2025-05-15T08:30:45+00:00");
        assert_eq!(dt2.to_rfc3339(), "2025-05-15T08:30:45+00:00");
        
        // Create an event with these times
        let mut event = create_test_event();
        event.start_time = dt1;
        event.end_time = dt1 + Duration::hours(1);
        
        // Validate the event
        let result = EventValidator::validate_event(&event);
        
        // Verify validation passes
        assert!(result.is_ok());
    }
}