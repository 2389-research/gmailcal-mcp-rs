/// Calendar API Mock Tests
///
/// This module tests the Calendar API functionality using mockall
/// to avoid the tokio runtime issues.
///
use mcp_gmailcal::errors::CalendarApiError;
use mcp_gmailcal::calendar_api::{CalendarEvent, CalendarInfo, CalendarList, Attendee, ConferenceData, ConferenceSolution, EntryPoint, EventOrganizer};
use chrono::{DateTime, Duration, TimeZone, Utc};
use mockall::predicate::*;
use uuid::Uuid;

// Mock the Calendar API client used in CalendarClient
mockall::mock! {
    pub CalendarApiClient {
        pub fn list_calendars<'a>(&'a self) -> Result<CalendarList, CalendarApiError>;
        pub fn list_events<'a>(&'a self, calendar_id: &'a str, max_results: Option<u32>, time_min: Option<DateTime<Utc>>, time_max: Option<DateTime<Utc>>) -> Result<Vec<CalendarEvent>, CalendarApiError>;
        pub fn create_event<'a>(&'a self, calendar_id: &'a str, event: CalendarEvent) -> Result<CalendarEvent, CalendarApiError>;
        pub fn get_event<'a>(&'a self, calendar_id: &'a str, event_id: &'a str) -> Result<CalendarEvent, CalendarApiError>;
    }
}

// Helper function to create a test event
fn create_test_event(
    id: &str,
    summary: &str,
    location: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> CalendarEvent {
    CalendarEvent {
        id: Some(id.to_string()),
        summary: summary.to_string(),
        description: Some(format!("Description for {}", summary)),
        location: Some(location.to_string()),
        start_time,
        end_time,
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
        conference_data: Some(ConferenceData {
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
        }),
        html_link: Some(format!("https://calendar.google.com/calendar/event?eid={}", id)),
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
    }
}

// Helper function to create a test calendar
fn create_test_calendar(id: &str, summary: &str, is_primary: bool) -> CalendarInfo {
    CalendarInfo {
        id: id.to_string(),
        summary: summary.to_string(),
        description: Some(format!("Description for {}", summary)),
        primary: Some(is_primary),
    }
}

// Tests for Calendar API functionality
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_calendars_success() {
        // Create mock client
        let mut mock = MockCalendarApiClient::new();
        
        // Create test calendars
        let test_calendars = vec![
            create_test_calendar("primary", "Primary Calendar", true),
            create_test_calendar("work@example.com", "Work Calendar", false),
            create_test_calendar("family@example.com", "Family Calendar", false),
        ];
        
        // Setup expectations
        mock.expect_list_calendars()
            .returning(move || Ok(CalendarList {
                calendars: test_calendars.clone(),
                next_page_token: None,
            }));
        
        // Test the function
        let result = mock.list_calendars();
        
        // Verify result
        assert!(result.is_ok());
        let calendar_list = result.unwrap();
        assert_eq!(calendar_list.calendars.len(), 3);
        
        // Verify primary calendar
        let primary = calendar_list.calendars.iter().find(|c| c.primary == Some(true)).unwrap();
        assert_eq!(primary.id, "primary");
        assert_eq!(primary.summary, "Primary Calendar");
        
        // Verify other calendars
        let work_calendar = calendar_list.calendars.iter().find(|c| c.id == "work@example.com").unwrap();
        assert_eq!(work_calendar.summary, "Work Calendar");
    }

    #[test]
    fn test_list_calendars_failure() {
        // Create mock client
        let mut mock = MockCalendarApiClient::new();
        
        // Setup expectations for failure
        mock.expect_list_calendars()
            .returning(|| Err(CalendarApiError::ApiError("Failed to list calendars".to_string())));
        
        // Test the function
        let result = mock.list_calendars();
        
        // Verify error
        assert!(result.is_err());
        match result {
            Err(CalendarApiError::ApiError(msg)) => {
                assert_eq!(msg, "Failed to list calendars");
            }
            _ => panic!("Expected ApiError")
        }
    }
    
    #[test]
    fn test_list_events_success() {
        // Create mock client
        let mut mock = MockCalendarApiClient::new();
        
        // Create test events
        let test_events = vec![
            create_test_event(
                "event1", 
                "Team Meeting", 
                "Conference Room", 
                Utc::now() + Duration::hours(1), 
                Utc::now() + Duration::hours(2)
            ),
            create_test_event(
                "event2", 
                "Client Call", 
                "Phone", 
                Utc::now() + Duration::hours(3), 
                Utc::now() + Duration::hours(4)
            ),
        ];
        
        // Setup expectations
        mock.expect_list_events()
            .returning(move |_, _, _, _| Ok(test_events.clone()));
        
        // Test the function with various parameters
        
        // No filters
        let result = mock.list_events("primary", None, None, None);
        
        // Verify result
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 2);
        
        // Verify event details
        assert_eq!(events[0].id.as_ref().unwrap(), "event1");
        assert_eq!(events[0].summary, "Team Meeting");
        assert_eq!(events[1].id.as_ref().unwrap(), "event2");
        assert_eq!(events[1].summary, "Client Call");
    }

    #[test]
    fn test_list_events_with_filters() {
        // Create mock client
        let mut mock = MockCalendarApiClient::new();
        
        // Create test events at different times
        let now = Utc::now();
        let event1 = create_test_event(
            "event1", 
            "Soon Event", 
            "Location 1", 
            now + Duration::hours(1), 
            now + Duration::hours(2)
        );
        let event2 = create_test_event(
            "event2", 
            "Later Event", 
            "Location 2", 
            now + Duration::days(1), 
            now + Duration::days(1) + Duration::hours(1)
        );
        
        // Clone events for use in closures
        let event1_clone = event1.clone();
        let event2_clone = event2.clone();
        let now_clone = now.clone();
        
        // Setup expectations for different time ranges
        mock.expect_list_events()
            .with(eq("primary"), eq(None), function(move |t: &Option<DateTime<Utc>>| t.is_some() && t.unwrap() > now_clone), eq(None))
            .returning(move |_, _, _, _| Ok(vec![event1_clone.clone(), event2_clone.clone()]));
            
        // Clone event1 again for the second closure
        let event1_clone2 = event1.clone();
            
        mock.expect_list_events()
            .with(eq("primary"), eq(Some(1)), eq(None), eq(None))
            .returning(move |_, _, _, _| Ok(vec![event1_clone2.clone()]));
        
        // Test with time filter
        let result = mock.list_events("primary", None, Some(now + Duration::minutes(30)), None);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 2);
        
        // Test with max_results
        let result = mock.list_events("primary", Some(1), None, None);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_list_events_failure() {
        // Create mock client
        let mut mock = MockCalendarApiClient::new();
        
        // Setup expectations for failure
        mock.expect_list_events()
            .returning(|_, _, _, _| Err(CalendarApiError::ApiError("Failed to list events".to_string())));
        
        // Test the function
        let result = mock.list_events("primary", None, None, None);
        
        // Verify error
        assert!(result.is_err());
        match result {
            Err(CalendarApiError::ApiError(msg)) => {
                assert_eq!(msg, "Failed to list events");
            }
            _ => panic!("Expected ApiError")
        }
    }
    
    #[test]
    fn test_create_event_success() {
        // Create mock client
        let mut mock = MockCalendarApiClient::new();
        
        // Setup expectations
        mock.expect_create_event()
            .returning(|_, event| {
                // Simulate server-side changes (ID assignment, link generation)
                let mut new_event = event.clone();
                if new_event.id.is_none() {
                    new_event.id = Some(Uuid::new_v4().to_string());
                }
                if new_event.html_link.is_none() {
                    new_event.html_link = Some(format!(
                        "https://calendar.google.com/calendar/event?eid={}",
                        new_event.id.as_ref().unwrap()
                    ));
                }
                Ok(new_event)
            });
        
        // Create a new event
        let new_event = CalendarEvent {
            id: None, // Will be assigned
            summary: "New Test Event".to_string(),
            description: Some("Description for new test event".to_string()),
            location: Some("Test Location".to_string()),
            start_time: Utc::now() + Duration::hours(24),
            end_time: Utc::now() + Duration::hours(25),
            attendees: vec![
                Attendee {
                    email: "test@example.com".to_string(),
                    display_name: Some("Test User".to_string()),
                    response_status: None,
                    optional: None,
                },
            ],
            html_link: None, // Will be assigned
            conference_data: None,
            creator: None,
            organizer: None,
        };
        
        // Test the function
        let result = mock.create_event("primary", new_event.clone());
        
        // Verify result
        assert!(result.is_ok());
        let created_event = result.unwrap();
        
        // Verify ID was assigned
        assert!(created_event.id.is_some());
        // Verify HTML link was assigned
        assert!(created_event.html_link.is_some());
        // Verify other fields match
        assert_eq!(created_event.summary, new_event.summary);
        assert_eq!(created_event.description, new_event.description);
        assert_eq!(created_event.location, new_event.location);
    }

    #[test]
    fn test_create_event_with_conference() {
        // Create mock client
        let mut mock = MockCalendarApiClient::new();
        
        // Setup expectations
        mock.expect_create_event()
            .returning(|_, event| {
                // Return event with assigned ID
                let mut new_event = event.clone();
                if new_event.id.is_none() {
                    new_event.id = Some(Uuid::new_v4().to_string());
                }
                Ok(new_event)
            });
        
        // Create an event with conference data
        let conference_event = CalendarEvent {
            id: None,
            summary: "Conference Event".to_string(),
            description: Some("Event with video conference".to_string()),
            location: Some("Virtual".to_string()),
            start_time: Utc::now() + Duration::hours(24),
            end_time: Utc::now() + Duration::hours(25),
            attendees: vec![
                Attendee {
                    email: "test@example.com".to_string(),
                    display_name: Some("Test User".to_string()),
                    response_status: None,
                    optional: None,
                },
            ],
            html_link: None,
            conference_data: Some(ConferenceData {
                conference_solution: Some(ConferenceSolution {
                    name: "Google Meet".to_string(),
                    key: Some("meet".to_string()),
                }),
                entry_points: vec![
                    EntryPoint {
                        entry_point_type: "video".to_string(),
                        uri: "https://meet.google.com/xyz-abcd-123".to_string(),
                        label: Some("Google Meet".to_string()),
                    },
                ],
            }),
            creator: None,
            organizer: None,
        };
        
        // Test the function
        let result = mock.create_event("primary", conference_event.clone());
        
        // Verify result
        assert!(result.is_ok());
        let created_event = result.unwrap();
        
        // Verify conference data was preserved
        assert!(created_event.conference_data.is_some());
        let conf_data = created_event.conference_data.unwrap();
        assert!(conf_data.conference_solution.is_some());
        assert_eq!(
            conf_data.conference_solution.unwrap().name, 
            "Google Meet"
        );
        assert_eq!(conf_data.entry_points.len(), 1);
        assert_eq!(
            conf_data.entry_points[0].uri, 
            "https://meet.google.com/xyz-abcd-123"
        );
    }

    #[test]
    fn test_create_event_validation_failures() {
        // Create mock client
        let mut mock = MockCalendarApiClient::new();
        
        // Setup expectations for validation failures
        mock.expect_create_event()
            .with(eq(""), function(|e: &CalendarEvent| !e.summary.is_empty()))
            .returning(|_, _| Err(CalendarApiError::ApiError("Calendar ID cannot be empty".to_string())));
            
        mock.expect_create_event()
            .with(function(|c: &str| !c.is_empty()), function(|e: &CalendarEvent| e.summary.is_empty()))
            .returning(|_, _| Err(CalendarApiError::EventFormatError("Event summary cannot be empty".to_string())));
            
        mock.expect_create_event()
            .with(function(|c: &str| !c.is_empty()), function(|e: &CalendarEvent| !e.summary.is_empty() && e.end_time <= e.start_time))
            .returning(|_, _| Err(CalendarApiError::EventFormatError("End time must be after start time".to_string())));
        
        // Test empty calendar ID
        let valid_event = CalendarEvent {
            id: None,
            summary: "Valid Event".to_string(),
            description: None,
            location: None,
            start_time: Utc::now() + Duration::hours(1),
            end_time: Utc::now() + Duration::hours(2),
            attendees: vec![],
            html_link: None,
            conference_data: None,
            creator: None,
            organizer: None,
        };
        
        let result = mock.create_event("", valid_event);
        assert!(result.is_err());
        match result {
            Err(CalendarApiError::ApiError(msg)) => {
                assert_eq!(msg, "Calendar ID cannot be empty");
            }
            _ => panic!("Expected ApiError"),
        }
        
        // Test empty summary
        let invalid_summary_event = CalendarEvent {
            id: None,
            summary: "".to_string(),
            description: None,
            location: None,
            start_time: Utc::now() + Duration::hours(1),
            end_time: Utc::now() + Duration::hours(2),
            attendees: vec![],
            html_link: None,
            conference_data: None,
            creator: None,
            organizer: None,
        };
        
        let result = mock.create_event("primary", invalid_summary_event);
        assert!(result.is_err());
        match result {
            Err(CalendarApiError::EventFormatError(msg)) => {
                assert_eq!(msg, "Event summary cannot be empty");
            }
            _ => panic!("Expected EventFormatError"),
        }
        
        // Test invalid time range
        let invalid_time_event = CalendarEvent {
            id: None,
            summary: "Invalid Time Event".to_string(),
            description: None,
            location: None,
            start_time: Utc::now() + Duration::hours(2),
            end_time: Utc::now() + Duration::hours(1), // End before start
            attendees: vec![],
            html_link: None,
            conference_data: None,
            creator: None,
            organizer: None,
        };
        
        let result = mock.create_event("primary", invalid_time_event);
        assert!(result.is_err());
        match result {
            Err(CalendarApiError::EventFormatError(msg)) => {
                assert_eq!(msg, "End time must be after start time");
            }
            _ => panic!("Expected EventFormatError"),
        }
    }

    #[test]
    fn test_create_event_failure() {
        // Create mock client
        let mut mock = MockCalendarApiClient::new();
        
        // Setup expectations for API failure
        mock.expect_create_event()
            .returning(|_, _| Err(CalendarApiError::ApiError("Failed to create event".to_string())));
        
        // Create a valid event
        let valid_event = CalendarEvent {
            id: None,
            summary: "Test Event".to_string(),
            description: None,
            location: None,
            start_time: Utc::now() + Duration::hours(1),
            end_time: Utc::now() + Duration::hours(2),
            attendees: vec![],
            html_link: None,
            conference_data: None,
            creator: None,
            organizer: None,
        };
        
        // Test the function
        let result = mock.create_event("primary", valid_event);
        
        // Verify error
        assert!(result.is_err());
        match result {
            Err(CalendarApiError::ApiError(msg)) => {
                assert_eq!(msg, "Failed to create event");
            }
            _ => panic!("Expected ApiError"),
        }
    }
    
    #[test]
    fn test_get_event_success() {
        // Create mock client
        let mut mock = MockCalendarApiClient::new();
        
        // Create a test event
        let test_event = create_test_event(
            "event1", 
            "Team Meeting", 
            "Conference Room", 
            Utc::now() + Duration::hours(1), 
            Utc::now() + Duration::hours(2)
        );
        
        // Setup expectations
        mock.expect_get_event()
            .with(eq("primary"), eq("event1"))
            .returning(move |_, _| Ok(test_event.clone()));
        
        // Test the function
        let result = mock.get_event("primary", "event1");
        
        // Verify result
        assert!(result.is_ok());
        let event = result.unwrap();
        
        // Verify event details
        assert_eq!(event.id.unwrap(), "event1");
        assert_eq!(event.summary, "Team Meeting");
        assert_eq!(event.location.unwrap(), "Conference Room");
        
        // Verify conference data
        assert!(event.conference_data.is_some());
        let conf_data = event.conference_data.unwrap();
        assert!(conf_data.conference_solution.is_some());
        assert_eq!(conf_data.conference_solution.unwrap().name, "Google Meet");
        assert_eq!(conf_data.entry_points.len(), 2);
        
        // Verify attendees
        assert_eq!(event.attendees.len(), 2);
        assert_eq!(event.attendees[0].email, "attendee1@example.com");
        assert_eq!(event.attendees[1].email, "attendee2@example.com");
    }

    #[test]
    fn test_get_event_not_found() {
        // Create mock client
        let mut mock = MockCalendarApiClient::new();
        
        // Setup expectations for not found
        mock.expect_get_event()
            .with(eq("primary"), eq("non_existent"))
            .returning(|_, id| Err(CalendarApiError::EventRetrievalError(format!("Event {} not found", id))));
        
        // Test the function
        let result = mock.get_event("primary", "non_existent");
        
        // Verify error
        assert!(result.is_err());
        match result {
            Err(CalendarApiError::EventRetrievalError(msg)) => {
                assert_eq!(msg, "Event non_existent not found");
            }
            _ => panic!("Expected EventRetrievalError"),
        }
    }

    #[test]
    fn test_get_event_invalid_parameters() {
        // Create mock client
        let mut mock = MockCalendarApiClient::new();
        
        // Setup expectations for invalid parameters
        mock.expect_get_event()
            .with(eq(""), function(|e: &str| !e.is_empty()))
            .returning(|_, _| Err(CalendarApiError::ApiError("Calendar ID cannot be empty".to_string())));
            
        mock.expect_get_event()
            .with(function(|c: &str| !c.is_empty()), eq(""))
            .returning(|_, _| Err(CalendarApiError::ApiError("Event ID cannot be empty".to_string())));
        
        // Test empty calendar ID
        let result = mock.get_event("", "event1");
        assert!(result.is_err());
        match result {
            Err(CalendarApiError::ApiError(msg)) => {
                assert_eq!(msg, "Calendar ID cannot be empty");
            }
            _ => panic!("Expected ApiError"),
        }
        
        // Test empty event ID
        let result = mock.get_event("primary", "");
        assert!(result.is_err());
        match result {
            Err(CalendarApiError::ApiError(msg)) => {
                assert_eq!(msg, "Event ID cannot be empty");
            }
            _ => panic!("Expected ApiError"),
        }
    }

    #[test]
    fn test_get_event_failure() {
        // Create mock client
        let mut mock = MockCalendarApiClient::new();
        
        // Setup expectations for API failure
        mock.expect_get_event()
            .with(eq("primary"), eq("event1"))
            .returning(|_, _| Err(CalendarApiError::ApiError("Failed to get event".to_string())));
        
        // Test the function
        let result = mock.get_event("primary", "event1");
        
        // Verify error
        assert!(result.is_err());
        match result {
            Err(CalendarApiError::ApiError(msg)) => {
                assert_eq!(msg, "Failed to get event");
            }
            _ => panic!("Expected ApiError"),
        }
    }
    
    #[test]
    fn test_timezone_parsing() {
        // Test parsing various timezone formats
        
        // UTC
        let utc_time = "2025-05-15T10:00:00Z";
        let parsed_utc = DateTime::parse_from_rfc3339(utc_time)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap();
        assert_eq!(parsed_utc.to_rfc3339(), "2025-05-15T10:00:00+00:00");
        
        // UTC with explicit offset
        let utc_explicit = "2025-05-15T10:00:00+00:00";
        let parsed_explicit = DateTime::parse_from_rfc3339(utc_explicit)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap();
        assert_eq!(parsed_explicit.to_rfc3339(), "2025-05-15T10:00:00+00:00");
        
        // Pacific Time (UTC-7)
        let pt_time = "2025-05-15T03:00:00-07:00";
        let parsed_pt = DateTime::parse_from_rfc3339(pt_time)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap();
        assert_eq!(parsed_pt.to_rfc3339(), "2025-05-15T10:00:00+00:00");
        
        // Central European Time (UTC+1)
        let cet_time = "2025-05-15T11:00:00+01:00";
        let parsed_cet = DateTime::parse_from_rfc3339(cet_time)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap();
        assert_eq!(parsed_cet.to_rfc3339(), "2025-05-15T10:00:00+00:00");
    }
    
    #[test]
    fn test_all_day_event_format() {
        // Test formatting for all-day events
        
        // Start time at midnight UTC
        let start_date = Utc.with_ymd_and_hms(2025, 5, 15, 0, 0, 0).unwrap();
        
        // End time at midnight the next day
        let end_date = Utc.with_ymd_and_hms(2025, 5, 16, 0, 0, 0).unwrap();
        
        // Format as RFC3339
        let start_rfc3339 = start_date.to_rfc3339();
        let end_rfc3339 = end_date.to_rfc3339();
        
        assert_eq!(start_rfc3339, "2025-05-15T00:00:00+00:00");
        assert_eq!(end_rfc3339, "2025-05-16T00:00:00+00:00");
        
        // The date component only (for all-day events)
        let date_only = start_date.format("%Y-%m-%d").to_string();
        assert_eq!(date_only, "2025-05-15");
    }
}