use chrono::{DateTime, Utc};
/// Calendar API Tests Module
///
/// This module contains tests for the Google Calendar API functionality,
/// focusing on calendar operations, event management, and datetime handling.
///
use mcp_gmailcal::calendar_api::{Calendar, CalendarClient, CalendarEvent, EventDateTime, EventAttendee, EventOrganizer};
use mcp_gmailcal::config::Config;
use serde_json::json;
use std::sync::Arc;
use reqwest::Client;
use uuid::Uuid;

// Define a MockCalendarClient for testing
struct MockCalendarClient {
    _config: Arc<Config>,
    _client: Client,
}

impl MockCalendarClient {
    fn new() -> Self {
        let config = Config {
            client_id: Some("test_client_id".to_string()),
            client_secret: Some("test_client_secret".to_string()),
            refresh_token: Some("test_refresh_token".to_string()),
            access_token: Some("test_access_token".to_string()),
            redirect_uri: Some("test_redirect_uri".to_string()),
        };
        
        let client = Client::new();
        
        Self {
            _config: Arc::new(config),
            _client: client,
        }
    }
    
    fn list_calendars(&self) -> Result<Vec<Calendar>, String> {
        // Return test calendars
        Ok(vec![
            Calendar {
                id: "primary".to_string(),
                summary: "Primary Calendar".to_string(),
                time_zone: Some("America/Los_Angeles".to_string()),
                access_role: Some("owner".to_string()),
                background_color: None,
                foreground_color: None,
                primary: Some(true),
            },
            Calendar {
                id: "work@example.com".to_string(),
                summary: "Work Calendar".to_string(),
                time_zone: Some("America/New_York".to_string()),
                access_role: Some("reader".to_string()),
                background_color: Some("#4285F4".to_string()),
                foreground_color: Some("#FFFFFF".to_string()),
                primary: Some(false),
            },
            Calendar {
                id: "family@example.com".to_string(),
                summary: "Family Calendar".to_string(),
                time_zone: Some("America/Chicago".to_string()),
                access_role: Some("writer".to_string()),
                background_color: Some("#DB4437".to_string()),
                foreground_color: Some("#FFFFFF".to_string()),
                primary: Some(false),
            },
        ])
    }
    
    fn get_event(&self, calendar_id: &str, event_id: &str) -> Result<CalendarEvent, String> {
        // Validate input
        if calendar_id.is_empty() {
            return Err("Calendar ID cannot be empty".to_string());
        }
        if event_id.is_empty() {
            return Err("Event ID cannot be empty".to_string());
        }
        
        // Return a test event
        Ok(CalendarEvent {
            id: event_id.to_string(),
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
            start: EventDateTime {
                date_time: Some("2025-04-15T14:30:00Z".to_string()),
                date: None,
                time_zone: Some("UTC".to_string()),
            },
            end: EventDateTime {
                date_time: Some("2025-04-15T15:30:00Z".to_string()),
                date: None,
                time_zone: Some("UTC".to_string()),
            },
            attendees: Some(vec![
                EventAttendee {
                    email: "attendee1@example.com".to_string(),
                    display_name: Some("Attendee 1".to_string()),
                    response_status: Some("accepted".to_string()),
                    self_: Some(false),
                },
                EventAttendee {
                    email: "attendee2@example.com".to_string(),
                    display_name: Some("Attendee 2".to_string()),
                    response_status: Some("tentative".to_string()),
                    self_: Some(false),
                },
            ]),
            html_link: Some(format!("https://calendar.google.com/calendar/event?eid={}", event_id)),
            conference_data: None,
        })
    }
    
    fn create_event(&self, calendar_id: &str, event: CalendarEvent) -> Result<CalendarEvent, String> {
        // Validate input
        if calendar_id.is_empty() {
            return Err("Calendar ID cannot be empty".to_string());
        }
        
        // Validate event fields
        if event.summary.is_empty() {
            return Err("Event summary cannot be empty".to_string());
        }
        
        if event.start.date_time.is_none() && event.start.date.is_none() {
            return Err("Event must have a start time or date".to_string());
        }
        
        if event.end.date_time.is_none() && event.end.date.is_none() {
            return Err("Event must have an end time or date".to_string());
        }
        
        // Create a new event with an ID
        let mut created_event = event;
        created_event.id = Uuid::new_v4().to_string();
        created_event.html_link = Some(format!("https://calendar.google.com/calendar/event?eid={}", created_event.id));
        
        Ok(created_event)
    }
}

// Mock CalendarClient for testing
#[cfg(test)]
mod mock_calendar_tests {
    use super::*;

    #[test]
    fn test_calendar_client_creation() {
        let client = MockCalendarClient::new();
        // Verify the client was created with valid config
        assert!(client._config.client_id.is_some());
        assert!(client._config.client_secret.is_some());
        assert!(client._config.refresh_token.is_some());
        assert!(client._config.access_token.is_some());
    }

    #[test]
    fn test_list_calendars() {
        let client = MockCalendarClient::new();
        let calendars = client.list_calendars().unwrap();
        
        // Verify we have the expected number of calendars
        assert_eq!(calendars.len(), 3);
        
        // Verify the primary calendar
        let primary_calendar = calendars.iter().find(|c| c.primary == Some(true)).unwrap();
        assert_eq!(primary_calendar.id, "primary");
        assert_eq!(primary_calendar.summary, "Primary Calendar");
        assert_eq!(primary_calendar.time_zone, Some("America/Los_Angeles".to_string()));
        
        // Verify the work calendar
        let work_calendar = calendars.iter().find(|c| c.id == "work@example.com").unwrap();
        assert_eq!(work_calendar.summary, "Work Calendar");
        assert_eq!(work_calendar.access_role, Some("reader".to_string()));
        
        // Verify the family calendar
        let family_calendar = calendars.iter().find(|c| c.id == "family@example.com").unwrap();
        assert_eq!(family_calendar.summary, "Family Calendar");
        assert_eq!(family_calendar.access_role, Some("writer".to_string()));
    }

    #[test]
    fn test_create_event() {
        let client = MockCalendarClient::new();
        
        // Create a new event
        let event = CalendarEvent {
            id: "".to_string(), // ID will be assigned by the server
            summary: "New Test Event".to_string(),
            description: Some("This is a new test event".to_string()),
            location: Some("Test Location".to_string()),
            creator: None, // Will be assigned by the server
            organizer: None, // Will be assigned by the server
            start: EventDateTime {
                date_time: Some("2025-05-15T10:00:00Z".to_string()),
                date: None,
                time_zone: Some("UTC".to_string()),
            },
            end: EventDateTime {
                date_time: Some("2025-05-15T11:00:00Z".to_string()),
                date: None,
                time_zone: Some("UTC".to_string()),
            },
            attendees: Some(vec![
                EventAttendee {
                    email: "attendee1@example.com".to_string(),
                    display_name: Some("Attendee 1".to_string()),
                    response_status: None,
                    self_: None,
                },
            ]),
            html_link: None, // Will be assigned by the server
            conference_data: None,
        };
        
        // Create the event
        let created_event = client.create_event("primary", event.clone()).unwrap();
        
        // Verify the created event
        assert!(!created_event.id.is_empty()); // Should have been assigned an ID
        assert_eq!(created_event.summary, "New Test Event");
        assert_eq!(created_event.description, Some("This is a new test event".to_string()));
        assert_eq!(created_event.start.date_time, Some("2025-05-15T10:00:00Z".to_string()));
        assert_eq!(created_event.end.date_time, Some("2025-05-15T11:00:00Z".to_string()));
        assert!(created_event.html_link.is_some()); // Should have been assigned a link
        
        // Test validation errors
        
        // Missing summary
        let invalid_event = CalendarEvent {
            summary: "".to_string(),
            ..event.clone()
        };
        assert!(client.create_event("primary", invalid_event).is_err());
        
        // Missing start time
        let invalid_event = CalendarEvent {
            start: EventDateTime {
                date_time: None,
                date: None,
                time_zone: Some("UTC".to_string()),
            },
            ..event.clone()
        };
        assert!(client.create_event("primary", invalid_event).is_err());
        
        // Missing end time
        let invalid_event = CalendarEvent {
            end: EventDateTime {
                date_time: None,
                date: None,
                time_zone: Some("UTC".to_string()),
            },
            ..event
        };
        assert!(client.create_event("primary", invalid_event).is_err());
        
        // Empty calendar ID
        assert!(client.create_event("", CalendarEvent {
            summary: "Test Event".to_string(),
            ..Default::default()
        }).is_err());
    }

    #[test]
    fn test_get_event() {
        let client = MockCalendarClient::new();
        
        // Get an event
        let event_id = "test_event_123";
        let event = client.get_event("primary", event_id).unwrap();
        
        // Verify the event
        assert_eq!(event.id, event_id);
        assert_eq!(event.summary, "Test Event");
        assert_eq!(event.description, Some("This is a test event".to_string()));
        assert_eq!(event.start.date_time, Some("2025-04-15T14:30:00Z".to_string()));
        assert_eq!(event.end.date_time, Some("2025-04-15T15:30:00Z".to_string()));
        
        // Verify attendees
        assert!(event.attendees.is_some());
        let attendees = event.attendees.unwrap();
        assert_eq!(attendees.len(), 2);
        assert_eq!(attendees[0].email, "attendee1@example.com");
        assert_eq!(attendees[0].response_status, Some("accepted".to_string()));
        assert_eq!(attendees[1].email, "attendee2@example.com");
        assert_eq!(attendees[1].response_status, Some("tentative".to_string()));
        
        // Test validation errors
        
        // Empty calendar ID
        assert!(client.get_event("", event_id).is_err());
        
        // Empty event ID
        assert!(client.get_event("primary", "").is_err());
    }

    // This test is for date/time handling, which doesn't depend on the client implementation
    #[test]
    fn test_date_handling() {
        // Test date parsing and formatting
        let date_str = "2025-04-15T14:30:00Z";
        let datetime = DateTime::parse_from_rfc3339(date_str)
            .unwrap()
            .with_timezone(&Utc);

        assert_eq!(datetime.to_rfc3339(), "2025-04-15T14:30:00+00:00");

        // Test with timezone
        let date_str = "2025-04-15T14:30:00-07:00";
        let datetime = DateTime::parse_from_rfc3339(date_str).unwrap();

        // Convert to UTC
        let utc_datetime = datetime.with_timezone(&Utc);

        // Verify UTC time is 7 hours ahead
        assert_eq!(utc_datetime.to_rfc3339(), "2025-04-15T21:30:00+00:00");
        
        // Test event date time representation
        let event_dt = EventDateTime {
            date_time: Some("2025-04-15T14:30:00Z".to_string()),
            date: None,
            time_zone: Some("UTC".to_string()),
        };
        
        assert!(event_dt.date_time.is_some());
        assert_eq!(event_dt.date_time.unwrap(), "2025-04-15T14:30:00Z");
        
        // Test all-day event representation
        let all_day_event_dt = EventDateTime {
            date_time: None,
            date: Some("2025-04-15".to_string()),
            time_zone: Some("UTC".to_string()),
        };
        
        assert!(all_day_event_dt.date.is_some());
        assert_eq!(all_day_event_dt.date.unwrap(), "2025-04-15");
    }
}
