/// Extended Integration Tests Module
///
/// This module contains expanded integration tests for end-to-end workflows,
/// focusing on common operations and cross-API scenarios.

use chrono::{DateTime, Duration, Utc};
use mcp_gmailcal::calendar_api::{Attendee, CalendarClient, CalendarEvent};
use mcp_gmailcal::config::Config;
use mcp_gmailcal::errors::{CalendarApiError, GmailApiError, PeopleApiError};
use mcp_gmailcal::gmail_api::{DraftEmail, EmailMessage, GmailService};
use mcp_gmailcal::people_api::{Contact, EmailAddress, PeopleClient};
use mcp_gmailcal::server::GmailServer;
use serde_json::{json, Value};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration as StdDuration, SystemTime};
use uuid::Uuid;

// Mock APIs for integration testing
struct IntegrationTestClient {
    gmail: MockGmailService,
    calendar: MockCalendarClient,
    people: MockPeopleClient,
}

impl IntegrationTestClient {
    fn new() -> Self {
        Self {
            gmail: MockGmailService::new(),
            calendar: MockCalendarClient::new(),
            people: MockPeopleClient::new(),
        }
    }
}

// Mock Gmail service
struct MockGmailService {
    messages: HashMap<String, Value>,
    labels: Vec<Value>,
    drafts: HashMap<String, Value>,
}

impl MockGmailService {
    fn new() -> Self {
        Self {
            messages: HashMap::new(),
            labels: vec![
                json!({"id": "INBOX", "name": "INBOX", "type": "system"}),
                json!({"id": "UNREAD", "name": "UNREAD", "type": "system"}),
                json!({"id": "IMPORTANT", "name": "IMPORTANT", "type": "system"}),
            ],
            drafts: HashMap::new(),
        }
    }
    
    fn add_message(&mut self, id: &str, message: Value) {
        self.messages.insert(id.to_string(), message);
    }
    
    fn get_message(&self, id: &str) -> Result<Value, GmailApiError> {
        match self.messages.get(id) {
            Some(message) => Ok(message.clone()),
            None => Err(GmailApiError::MessageRetrievalError(format!("Message not found: {}", id))),
        }
    }
    
    fn list_messages(&self, query: Option<&str>) -> Result<Vec<Value>, GmailApiError> {
        // Return all messages or filter by query
        let mut messages: Vec<Value> = self.messages.values().cloned().collect();
        
        // If query is provided, simulate filtering
        if let Some(q) = query {
            // Very simple filtering simulation - in a real implementation, this would be more sophisticated
            if q.contains("is:unread") {
                messages.retain(|m| {
                    m["labelIds"].as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .any(|id| id == "UNREAD")
                });
            }
            if q.contains("is:important") {
                messages.retain(|m| {
                    m["labelIds"].as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .any(|id| id == "IMPORTANT")
                });
            }
            // Filter by subject (case-insensitive)
            if q.contains("subject:") {
                let subject_filter = q.split("subject:").nth(1).unwrap_or("").trim();
                if !subject_filter.is_empty() {
                    messages.retain(|m| {
                        let headers = m["payload"]["headers"].as_array().unwrap_or(&vec![]);
                        headers.iter().any(|h| {
                            h["name"] == "Subject" && 
                            h["value"].as_str().unwrap_or("").to_lowercase().contains(&subject_filter.to_lowercase())
                        })
                    });
                }
            }
        }
        
        Ok(messages)
    }
    
    fn list_labels(&self) -> Result<Vec<Value>, GmailApiError> {
        Ok(self.labels.clone())
    }
    
    fn create_draft(&mut self, draft: DraftEmail) -> Result<String, GmailApiError> {
        let draft_id = Uuid::new_v4().to_string();
        
        // Convert draft to JSON
        let draft_json = json!({
            "id": draft_id,
            "message": {
                "id": Uuid::new_v4().to_string(),
                "threadId": Uuid::new_v4().to_string(),
                "labelIds": ["DRAFT"],
                "payload": {
                    "headers": [
                        {"name": "Subject", "value": draft.subject},
                        {"name": "From", "value": draft.from},
                        {"name": "To", "value": draft.to.join(", ")},
                        {"name": "Date", "value": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().to_string()}
                    ],
                    "mimeType": "text/plain",
                    "body": {
                        "data": base64::engine::general_purpose::STANDARD.encode(draft.body.as_bytes()),
                        "size": draft.body.len()
                    }
                }
            }
        });
        
        self.drafts.insert(draft_id.clone(), draft_json);
        
        Ok(draft_id)
    }
}

// Mock Calendar client
struct MockCalendarClient {
    calendars: Vec<Value>,
    events: HashMap<String, CalendarEvent>,
}

impl MockCalendarClient {
    fn new() -> Self {
        Self {
            calendars: vec![
                json!({
                    "id": "primary",
                    "summary": "Primary Calendar",
                    "timeZone": "America/Los_Angeles",
                    "primary": true
                }),
                json!({
                    "id": "work@example.com",
                    "summary": "Work Calendar",
                    "timeZone": "America/New_York",
                    "primary": false
                }),
            ],
            events: HashMap::new(),
        }
    }
    
    fn add_event(&mut self, calendar_id: &str, event: CalendarEvent) -> Result<(), CalendarApiError> {
        let event_id = event.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        let mut event_with_id = event;
        
        // Ensure the event has an ID
        if event_with_id.id.is_none() {
            event_with_id.id = Some(event_id.clone());
        }
        
        // Store with a compound key (calendar_id + event_id)
        let key = format!("{}:{}", calendar_id, event_id);
        self.events.insert(key, event_with_id);
        
        Ok(())
    }
    
    fn get_event(&self, calendar_id: &str, event_id: &str) -> Result<CalendarEvent, CalendarApiError> {
        let key = format!("{}:{}", calendar_id, event_id);
        match self.events.get(&key) {
            Some(event) => Ok(event.clone()),
            None => Err(CalendarApiError::EventRetrievalError(format!("Event not found: {}", event_id))),
        }
    }
    
    fn list_calendars(&self) -> Result<Vec<Value>, CalendarApiError> {
        Ok(self.calendars.clone())
    }
}

// Mock People client
struct MockPeopleClient {
    contacts: HashMap<String, Contact>,
}

impl MockPeopleClient {
    fn new() -> Self {
        Self {
            contacts: HashMap::new(),
        }
    }
    
    fn add_contact(&mut self, contact: Contact) -> Result<(), PeopleApiError> {
        let contact_id = contact.resource_name.clone();
        self.contacts.insert(contact_id, contact);
        Ok(())
    }
    
    fn get_contact(&self, resource_name: &str) -> Result<Contact, PeopleApiError> {
        match self.contacts.get(resource_name) {
            Some(contact) => Ok(contact.clone()),
            None => Err(PeopleApiError::ApiError(format!("Contact not found: {}", resource_name))),
        }
    }
    
    fn search_contacts(&self, query: &str) -> Result<Vec<Contact>, PeopleApiError> {
        let query_lower = query.to_lowercase();
        
        // Filter contacts by name or email
        let matching_contacts: Vec<Contact> = self.contacts.values()
            .filter(|contact| {
                // Match by name
                let name_match = contact.names.iter().any(|name| {
                    name.display_name.as_ref()
                        .map(|display_name| display_name.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                });
                
                // Match by email
                let email_match = contact.email_addresses.iter().any(|email| {
                    email.value.to_lowercase().contains(&query_lower)
                });
                
                name_match || email_match
            })
            .cloned()
            .collect();
        
        Ok(matching_contacts)
    }
}

// Create test email
fn create_test_email(id: &str, subject: &str, body: &str, labels: Vec<&str>, sender: &str, recipient: &str) -> Value {
    json!({
        "id": id,
        "threadId": format!("thread_{}", id),
        "labelIds": labels,
        "snippet": body.chars().take(50).collect::<String>(),
        "payload": {
            "mimeType": "text/plain",
            "headers": [
                { "name": "From", "value": sender },
                { "name": "To", "value": recipient },
                { "name": "Subject", "value": subject },
                { "name": "Date", "value": "Mon, 15 Apr 2025 10:00:00 +0000" }
            ],
            "body": {
                "data": base64::engine::general_purpose::STANDARD.encode(body.as_bytes()),
                "size": body.len()
            }
        }
    })
}

// Create test contact
fn create_test_contact(resource_name: &str, display_name: &str, email: &str) -> Contact {
    Contact {
        resource_name: resource_name.to_string(),
        etag: "etag123".to_string(),
        names: vec![
            mcp_gmailcal::people_api::PersonName {
                display_name: Some(display_name.to_string()),
                family_name: Some(display_name.split_whitespace().last().unwrap_or("").to_string()),
                given_name: Some(display_name.split_whitespace().next().unwrap_or("").to_string()),
                middle_name: None,
                display_name_last_first: None,
                unstructured_name: None,
            }
        ],
        email_addresses: vec![
            EmailAddress {
                value: email.to_string(),
                type_: Some("work".to_string()),
                display_name: None,
            }
        ],
        phone_numbers: vec![],
        photos: vec![],
        organizations: vec![],
        occupations: vec![],
        urls: vec![],
        user_defined: HashMap::new(),
    }
}

// Simulate searching and creating calendar event from email
async fn search_and_create_event(client: &IntegrationTestClient, query: &str) -> Result<String, GmailApiError> {
    // 1. Search for emails matching the query
    let messages = client.gmail.list_messages(Some(query))?;
    
    if messages.is_empty() {
        return Err(GmailApiError::MessageRetrievalError("No messages found".to_string()));
    }
    
    // 2. Get the first matching message
    let message_id = messages[0]["id"].as_str().unwrap();
    let message_data = client.gmail.get_message(message_id)?;
    
    // 3. Parse the message to extract meeting details
    let headers = message_data["payload"]["headers"].as_array().unwrap();
    let subject = headers.iter()
        .find(|h| h["name"] == "Subject")
        .map(|h| h["value"].as_str().unwrap())
        .unwrap_or("No Subject");
    
    let from = headers.iter()
        .find(|h| h["name"] == "From")
        .map(|h| h["value"].as_str().unwrap())
        .unwrap_or("unknown@example.com");
    
    // 4. Look up the sender in contacts
    let sender_email = if from.contains('<') && from.contains('>') {
        from.split('<').nth(1).unwrap().split('>').next().unwrap()
    } else {
        from
    };
    
    let contacts = client.people.search_contacts(sender_email).unwrap_or_default();
    let sender_contact = contacts.first();
    
    // 5. Create a calendar event
    let mut event = CalendarEvent {
        id: None,
        summary: format!("Meeting: {}", subject),
        description: Some(format!("Meeting with {}", from)),
        location: None,
        start_time: Utc::now() + Duration::hours(24), // Tomorrow
        end_time: Utc::now() + Duration::hours(25),   // 1 hour meeting
        attendees: vec![
            Attendee {
                email: sender_email.to_string(),
                display_name: sender_contact.and_then(|c| c.names.first()).and_then(|n| n.display_name.clone()),
                response_status: Some("needsAction".to_string()),
                optional: None,
            }
        ],
        creator: None,
        organizer: None,
        html_link: None,
        conference_data: None,
    };
    
    // 6. Add the event to the calendar
    if let Err(e) = client.calendar.add_event("primary", event.clone()) {
        return Err(GmailApiError::ApiError(format!("Failed to create calendar event: {}", e)));
    }
    
    // Return the event summary
    Ok(format!("Created event: {}", event.summary))
}

#[cfg(test)]
mod extended_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_email_workflow() {
        // Create test client
        let mut client = IntegrationTestClient::new();
        
        // Add test emails
        client.gmail.add_message(
            "msg1",
            create_test_email(
                "msg1",
                "Project Meeting",
                "Let's schedule a project meeting for tomorrow.",
                vec!["INBOX", "UNREAD"],
                "John Doe <john@example.com>",
                "receiver@example.com"
            )
        );
        
        client.gmail.add_message(
            "msg2",
            create_test_email(
                "msg2",
                "Vacation Plans",
                "Here are the details for our vacation next month.",
                vec!["INBOX", "IMPORTANT"],
                "Jane Smith <jane@example.com>",
                "receiver@example.com"
            )
        );
        
        // Add test contacts
        client.people.add_contact(create_test_contact(
            "people/123",
            "John Doe",
            "john@example.com"
        )).unwrap();
        
        client.people.add_contact(create_test_contact(
            "people/456",
            "Jane Smith",
            "jane@example.com"
        )).unwrap();
        
        // Test the workflow
        let result = search_and_create_event(&client, "subject:Project Meeting").await;
        
        // Verify the result
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Created event: Meeting: Project Meeting"));
        
        // Verify the calendar event was created
        let events = client.calendar.events.values().collect::<Vec<_>>();
        assert!(!events.is_empty());
        
        let project_meeting = events.iter().find(|e| e.summary.contains("Project Meeting"));
        assert!(project_meeting.is_some());
        
        let meeting = project_meeting.unwrap();
        assert!(meeting.description.as_ref().unwrap().contains("John Doe"));
        assert_eq!(meeting.attendees.len(), 1);
        assert_eq!(meeting.attendees[0].email, "john@example.com");
    }

    #[tokio::test]
    async fn test_calendar_integration_with_email_content() {
        // Create test client
        let mut client = IntegrationTestClient::new();
        
        // Add a test email with an event invitation
        let invitation_body = r"
        You're invited to a team meeting!
        
        Date: May 15, 2025
        Time: 10:00 AM - 11:00 AM
        Location: Conference Room A
        
        Please RSVP by replying to this email.
        ";
        
        client.gmail.add_message(
            "invite1",
            create_test_email(
                "invite1",
                "Team Meeting Invitation",
                invitation_body,
                vec!["INBOX", "UNREAD"],
                "Team Lead <lead@example.com>",
                "team@example.com"
            )
        );
        
        // Add contact for the sender
        client.people.add_contact(create_test_contact(
            "people/789",
            "Team Lead",
            "lead@example.com"
        )).unwrap();
        
        // Test creating an event from the email content
        let result = search_and_create_event(&client, "subject:Team Meeting").await;
        
        // Verify the result
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Created event: Meeting: Team Meeting Invitation"));
        
        // Verify the calendar event was created with correct details
        let events = client.calendar.events.values().collect::<Vec<_>>();
        assert!(!events.is_empty());
        
        let team_meeting = events.iter().find(|e| e.summary.contains("Team Meeting"));
        assert!(team_meeting.is_some());
        
        let meeting = team_meeting.unwrap();
        assert!(meeting.description.as_ref().unwrap().contains("Team Lead"));
        assert_eq!(meeting.attendees.len(), 1);
        assert_eq!(meeting.attendees[0].email, "lead@example.com");
    }

    #[tokio::test]
    async fn test_contact_lookup_during_email_processing() {
        // Create test client
        let mut client = IntegrationTestClient::new();
        
        // Add test emails from multiple contacts
        client.gmail.add_message(
            "msg1",
            create_test_email(
                "msg1",
                "Project Status",
                "Here's the latest project status.",
                vec!["INBOX"],
                "Alice Johnson <alice@example.com>",
                "receiver@example.com"
            )
        );
        
        client.gmail.add_message(
            "msg2",
            create_test_email(
                "msg2",
                "Budget Review",
                "Let's review the budget this week.",
                vec!["INBOX"],
                "Bob Wilson <bob@example.com>",
                "receiver@example.com"
            )
        );
        
        // Add several contacts
        client.people.add_contact(create_test_contact(
            "people/001",
            "Alice Johnson",
            "alice@example.com"
        )).unwrap();
        
        client.people.add_contact(create_test_contact(
            "people/002",
            "Bob Wilson",
            "bob@example.com"
        )).unwrap();
        
        client.people.add_contact(create_test_contact(
            "people/003",
            "Charlie Davis",
            "charlie@example.com"
        )).unwrap();
        
        // Search for emails and verify contact lookup
        let result1 = search_and_create_event(&client, "subject:Project Status").await;
        let result2 = search_and_create_event(&client, "subject:Budget Review").await;
        
        // Verify results
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        // Check that the events were created with proper contact info
        let events = client.calendar.events.values().collect::<Vec<_>>();
        assert_eq!(events.len(), 2);
        
        // Find events for each contact
        let alice_event = events.iter().find(|e| e.attendees[0].email == "alice@example.com");
        let bob_event = events.iter().find(|e| e.attendees[0].email == "bob@example.com");
        
        assert!(alice_event.is_some());
        assert!(bob_event.is_some());
        
        // Verify the display names were correctly retrieved from contacts
        assert_eq!(
            alice_event.unwrap().attendees[0].display_name.as_ref().map(|s| s.as_str()),
            Some("Alice Johnson")
        );
        
        assert_eq!(
            bob_event.unwrap().attendees[0].display_name.as_ref().map(|s| s.as_str()),
            Some("Bob Wilson")
        );
    }

    #[tokio::test]
    async fn test_parallel_api_operations() {
        // Create test client
        let mut client = IntegrationTestClient::new();
        
        // Add test data
        for i in 1..10 {
            // Add test emails
            client.gmail.add_message(
                &format!("msg{}", i),
                create_test_email(
                    &format!("msg{}", i),
                    &format!("Test Message {}", i),
                    &format!("This is test message {}.", i),
                    vec!["INBOX"],
                    &format!("sender{}@example.com", i),
                    "receiver@example.com"
                )
            );
            
            // Add test contacts
            client.people.add_contact(create_test_contact(
                &format!("people/{}", i),
                &format!("Contact {}", i),
                &format!("contact{}@example.com", i)
            )).unwrap();
        }
        
        // Prepare multiple parallel operations
        let mut handles = vec![];
        
        // Operation 1: Create an event from email 1
        let client_ref = &client;
        let handle1 = tokio::spawn(async move {
            search_and_create_event(client_ref, "subject:\"Test Message 1\"").await
        });
        handles.push(handle1);
        
        // Operation 2: Create an event from email 2
        let client_ref = &client;
        let handle2 = tokio::spawn(async move {
            search_and_create_event(client_ref, "subject:\"Test Message 2\"").await
        });
        handles.push(handle2);
        
        // Operation 3: Create an event from email 3
        let client_ref = &client;
        let handle3 = tokio::spawn(async move {
            search_and_create_event(client_ref, "subject:\"Test Message 3\"").await
        });
        handles.push(handle3);
        
        // Wait for all operations to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
        
        // Verify all events were created
        let events = client.calendar.events.values().collect::<Vec<_>>();
        assert_eq!(events.len(), 3);
        
        // Verify the specific events
        let event_titles: Vec<_> = events.iter().map(|e| e.summary.clone()).collect();
        assert!(event_titles.contains(&"Meeting: Test Message 1".to_string()));
        assert!(event_titles.contains(&"Meeting: Test Message 2".to_string()));
        assert!(event_titles.contains(&"Meeting: Test Message 3".to_string()));
    }

    #[tokio::test]
    async fn test_end_to_end_workflow() {
        // Create test client
        let mut client = IntegrationTestClient::new();
        
        // 1. Add emails with meeting requests
        client.gmail.add_message(
            "meeting1",
            create_test_email(
                "meeting1",
                "Quarterly Planning Meeting",
                "Let's schedule our quarterly planning meeting for next Wednesday at 2pm.",
                vec!["INBOX", "IMPORTANT"],
                "CEO <ceo@example.com>",
                "team@example.com"
            )
        );
        
        // 2. Add contacts
        client.people.add_contact(create_test_contact(
            "people/ceo",
            "Company CEO",
            "ceo@example.com"
        )).unwrap();
        
        // 3. Simulate finding the email
        let messages = client.gmail.list_messages(Some("is:important")).unwrap();
        assert!(!messages.is_empty());
        
        // 4. Process the most important email
        let important_message_id = messages[0]["id"].as_str().unwrap();
        let message = client.gmail.get_message(important_message_id).unwrap();
        
        // 5. Extract sender information
        let headers = message["payload"]["headers"].as_array().unwrap();
        let from = headers.iter()
            .find(|h| h["name"] == "From")
            .map(|h| h["value"].as_str().unwrap())
            .unwrap_or("");
        
        let sender_email = if from.contains('<') && from.contains('>') {
            from.split('<').nth(1).unwrap().split('>').next().unwrap()
        } else {
            from
        };
        
        // 6. Look up the sender in contacts
        let contacts = client.people.search_contacts(sender_email).unwrap();
        assert!(!contacts.is_empty());
        
        let contact = &contacts[0];
        let contact_name = contact.names[0].display_name.as_ref().unwrap();
        
        // 7. Create a calendar event
        let event = CalendarEvent {
            id: None,
            summary: "Quarterly Planning Meeting",
            description: Some(format!("Meeting requested by {}", contact_name)),
            location: Some("Main Conference Room".to_string()),
            start_time: Utc::now() + Duration::days(7) + Duration::hours(14), // Next Wednesday at 2pm
            end_time: Utc::now() + Duration::days(7) + Duration::hours(15),   // 1 hour meeting
            attendees: vec![
                Attendee {
                    email: sender_email.to_string(),
                    display_name: Some(contact_name.clone()),
                    response_status: Some("accepted".to_string()),
                    optional: None,
                },
                Attendee {
                    email: "team@example.com".to_string(),
                    display_name: None,
                    response_status: Some("needsAction".to_string()),
                    optional: None,
                },
            ],
            creator: None,
            organizer: None,
            html_link: None,
            conference_data: None,
        };
        
        // 8. Add the event to the calendar
        client.calendar.add_event("primary", event.clone()).unwrap();
        
        // 9. Create a draft email response
        let draft = DraftEmail {
            to: vec![sender_email.to_string()],
            cc: vec![],
            bcc: vec![],
            subject: "Re: Quarterly Planning Meeting",
            from: "user@example.com".to_string(),
            body: format!("Hi {},\n\nI've scheduled the quarterly planning meeting for next Wednesday at 2pm in the Main Conference Room.\n\nBest regards,\nUser", contact_name),
            reply_to: None,
        };
        
        let draft_id = client.gmail.create_draft(draft).unwrap();
        
        // 10. Verify the workflow completed successfully
        assert!(!draft_id.is_empty());
        
        // Verify the event was created properly
        let events = client.calendar.events.values().collect::<Vec<_>>();
        assert!(!events.is_empty());
        
        let planning_meeting = events.iter().find(|e| e.summary == "Quarterly Planning Meeting");
        assert!(planning_meeting.is_some());
        
        let meeting = planning_meeting.unwrap();
        assert_eq!(meeting.attendees.len(), 2);
        assert_eq!(meeting.attendees[0].email, "ceo@example.com");
        assert_eq!(meeting.attendees[0].display_name.as_ref().unwrap(), "Company CEO");
    }
}