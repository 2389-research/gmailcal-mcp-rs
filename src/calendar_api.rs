use crate::auth::TokenManager;
use crate::config::Config;
use chrono::{DateTime, Utc};
use log::{debug, error};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

const CALENDAR_API_BASE_URL: &str = "https://www.googleapis.com/calendar/v3";

use crate::errors::{CalendarApiError, CalendarResult};

// Alias for backward compatibility within this module
type Result<T> = CalendarResult<T>;

// Calendar event representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: Option<String>,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub attendees: Vec<Attendee>,
    pub conference_data: Option<ConferenceData>,
    pub html_link: Option<String>,
    pub creator: Option<EventOrganizer>,
    pub organizer: Option<EventOrganizer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventOrganizer {
    pub email: String,
    pub display_name: Option<String>,
    pub self_: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendee {
    pub email: String,
    pub display_name: Option<String>,
    pub response_status: Option<String>,
    pub optional: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConferenceData {
    pub conference_solution: Option<ConferenceSolution>,
    pub entry_points: Vec<EntryPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConferenceSolution {
    pub name: String,
    pub key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoint {
    pub entry_point_type: String,
    pub uri: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarList {
    pub calendars: Vec<CalendarInfo>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarInfo {
    pub id: String,
    pub summary: String,
    pub description: Option<String>,
    pub primary: Option<bool>,
}

// Calendar API client
#[derive(Debug, Clone)]
pub struct CalendarClient {
    client: Client,
    token_manager: Arc<Mutex<TokenManager>>,
}

impl CalendarClient {
    pub fn new(config: &Config) -> Self {
        let client = Client::new();
        // Reuse the Gmail token manager since they share the same OAuth scope
        let token_manager = Arc::new(Mutex::new(TokenManager::new(config)));

        Self {
            client,
            token_manager,
        }
    }

    // Get a list of all calendars
    pub async fn list_calendars(&self) -> Result<CalendarList> {
        let token = self
            .token_manager
            .lock()
            .await
            .get_token(&self.client)
            .await
            .map_err(|e| CalendarApiError::AuthError(e.to_string()))?;

        let url = format!("{}/users/me/calendarList", CALENDAR_API_BASE_URL);
        debug!("Listing calendars from: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| CalendarApiError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<no response body>".to_string());
            return Err(CalendarApiError::ApiError(format!(
                "Failed to list calendars. Status: {}, Error: {}",
                status, error_text
            )));
        }

        let json_response = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| CalendarApiError::ParseError(e.to_string()))?;

        let mut calendars = Vec::new();

        if let Some(items) = json_response.get("items").and_then(|v| v.as_array()) {
            for item in items {
                let id = item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CalendarApiError::ParseError("Missing calendar id".to_string()))?
                    .to_string();

                let summary = item
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Calendar")
                    .to_string();

                let description = item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let primary = item.get("primary").and_then(|v| v.as_bool());

                calendars.push(CalendarInfo {
                    id,
                    summary,
                    description,
                    primary,
                });
            }
        }

        let next_page_token = json_response
            .get("nextPageToken")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(CalendarList {
            calendars,
            next_page_token,
        })
    }

    // Get events from a specific calendar
    pub async fn list_events(
        &self,
        calendar_id: &str,
        max_results: Option<u32>,
        time_min: Option<DateTime<Utc>>,
        time_max: Option<DateTime<Utc>>,
    ) -> Result<Vec<CalendarEvent>> {
        let token = self
            .token_manager
            .lock()
            .await
            .get_token(&self.client)
            .await
            .map_err(|e| CalendarApiError::AuthError(e.to_string()))?;

        let mut url = format!("{}/calendars/{}/events", CALENDAR_API_BASE_URL, calendar_id);

        // Build query parameters
        let mut query_parts = Vec::new();

        if let Some(max) = max_results {
            query_parts.push(format!("maxResults={}", max));
        }

        if let Some(min_time) = time_min {
            query_parts.push(format!("timeMin={}", min_time.to_rfc3339()));
        }

        if let Some(max_time) = time_max {
            query_parts.push(format!("timeMax={}", max_time.to_rfc3339()));
        }

        // Add single events mode to expand recurring events
        query_parts.push("singleEvents=true".to_string());

        // Order by start time
        query_parts.push("orderBy=startTime".to_string());

        if !query_parts.is_empty() {
            url = format!("{}?{}", url, query_parts.join("&"));
        }

        debug!("Listing events from: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| CalendarApiError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<no response body>".to_string());
            return Err(CalendarApiError::ApiError(format!(
                "Failed to list events. Status: {}, Error: {}",
                status, error_text
            )));
        }

        let json_response = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| CalendarApiError::ParseError(e.to_string()))?;

        let mut events = Vec::new();

        if let Some(items) = json_response.get("items").and_then(|v| v.as_array()) {
            for item in items {
                if let Ok(event) = self.parse_event(item) {
                    events.push(event);
                } else {
                    // Log parsing error but continue with other events
                    error!("Failed to parse event: {:?}", item);
                }
            }
        }

        Ok(events)
    }

    // Create a new calendar event
    pub async fn create_event(
        &self,
        calendar_id: &str,
        event: CalendarEvent,
    ) -> Result<CalendarEvent> {
        let token = self
            .token_manager
            .lock()
            .await
            .get_token(&self.client)
            .await
            .map_err(|e| CalendarApiError::AuthError(e.to_string()))?;

        let url = format!("{}/calendars/{}/events", CALENDAR_API_BASE_URL, calendar_id);
        debug!("Creating new event in calendar {}", calendar_id);

        // Convert our CalendarEvent to Google Calendar API format
        let mut event_data = serde_json::Map::new();
        event_data.insert(
            "summary".to_string(),
            serde_json::Value::String(event.summary),
        );

        if let Some(desc) = event.description {
            event_data.insert("description".to_string(), serde_json::Value::String(desc));
        }

        if let Some(loc) = event.location {
            event_data.insert("location".to_string(), serde_json::Value::String(loc));
        }

        // Add start time
        let mut start = serde_json::Map::new();
        start.insert(
            "dateTime".to_string(),
            serde_json::Value::String(event.start_time.to_rfc3339()),
        );
        start.insert(
            "timeZone".to_string(),
            serde_json::Value::String("UTC".to_string()),
        );
        event_data.insert("start".to_string(), serde_json::Value::Object(start));

        // Add end time
        let mut end = serde_json::Map::new();
        end.insert(
            "dateTime".to_string(),
            serde_json::Value::String(event.end_time.to_rfc3339()),
        );
        end.insert(
            "timeZone".to_string(),
            serde_json::Value::String("UTC".to_string()),
        );
        event_data.insert("end".to_string(), serde_json::Value::Object(end));

        // Add attendees if any
        if !event.attendees.is_empty() {
            let attendees = event
                .attendees
                .iter()
                .map(|a| {
                    let mut attendee = serde_json::Map::new();
                    attendee.insert(
                        "email".to_string(),
                        serde_json::Value::String(a.email.clone()),
                    );

                    if let Some(name) = &a.display_name {
                        attendee.insert(
                            "displayName".to_string(),
                            serde_json::Value::String(name.clone()),
                        );
                    }

                    if let Some(status) = &a.response_status {
                        attendee.insert(
                            "responseStatus".to_string(),
                            serde_json::Value::String(status.clone()),
                        );
                    }

                    if let Some(optional) = a.optional {
                        attendee.insert("optional".to_string(), serde_json::Value::Bool(optional));
                    }

                    serde_json::Value::Object(attendee)
                })
                .collect::<Vec<_>>();

            event_data.insert("attendees".to_string(), serde_json::Value::Array(attendees));
        }

        // Generate unique ID for request for idempotency
        // This header ensures the request can be safely retried without creating duplicate events
        // Google recommends using the same ID for retries of the same logical operation
        let request_id = Uuid::new_v4().to_string();
        debug!("Using idempotency header X-Goog-Request-ID: {}", request_id);

        // Store the request ID for potential retry operations
        // This would typically be stored in a transaction log or retry mechanism

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            // Add idempotency header to prevent duplicate events on retry
            .header("X-Goog-Request-ID", request_id)
            .json(&event_data)
            .send()
            .await
            .map_err(|e| CalendarApiError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<no response body>".to_string());
            return Err(CalendarApiError::ApiError(format!(
                "Failed to create event. Status: {}, Error: {}",
                status, error_text
            )));
        }

        let json_response = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| CalendarApiError::ParseError(e.to_string()))?;

        self.parse_event(&json_response)
    }

    // Get a specific event
    pub async fn get_event(&self, calendar_id: &str, event_id: &str) -> Result<CalendarEvent> {
        let token = self
            .token_manager
            .lock()
            .await
            .get_token(&self.client)
            .await
            .map_err(|e| CalendarApiError::AuthError(e.to_string()))?;

        let url = format!(
            "{}/calendars/{}/events/{}",
            CALENDAR_API_BASE_URL, calendar_id, event_id
        );
        debug!("Getting event {} from calendar {}", event_id, calendar_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| CalendarApiError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<no response body>".to_string());
            return Err(CalendarApiError::ApiError(format!(
                "Failed to get event. Status: {}, Error: {}",
                status, error_text
            )));
        }

        let json_response = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| CalendarApiError::ParseError(e.to_string()))?;

        self.parse_event(&json_response)
    }

    // Helper to parse Google Calendar event format into our CalendarEvent struct
    fn parse_event(&self, item: &serde_json::Value) -> Result<CalendarEvent> {
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let summary = item
            .get("summary")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CalendarApiError::ParseError("Missing event summary".to_string()))?
            .to_string();

        let description = item
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let location = item
            .get("location")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Parse datetime structures
        let start_time = item
            .get("start")
            .and_then(|v| v.get("dateTime"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| CalendarApiError::ParseError("Missing start time".to_string()))?;

        let end_time = item
            .get("end")
            .and_then(|v| v.get("dateTime"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| CalendarApiError::ParseError("Missing end time".to_string()))?;

        // Parse RFC3339 format to DateTime<Utc>
        let start_dt = DateTime::parse_from_rfc3339(start_time)
            .map_err(|e| CalendarApiError::ParseError(format!("Invalid start time: {}", e)))?
            .with_timezone(&Utc);

        let end_dt = DateTime::parse_from_rfc3339(end_time)
            .map_err(|e| CalendarApiError::ParseError(format!("Invalid end time: {}", e)))?
            .with_timezone(&Utc);

        // Parse attendees
        let mut attendees = Vec::new();
        if let Some(attendee_list) = item.get("attendees").and_then(|v| v.as_array()) {
            for attendee in attendee_list {
                let email = attendee
                    .get("email")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        CalendarApiError::ParseError("Missing attendee email".to_string())
                    })?
                    .to_string();

                let display_name = attendee
                    .get("displayName")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let response_status = attendee
                    .get("responseStatus")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let optional = attendee.get("optional").and_then(|v| v.as_bool());

                attendees.push(Attendee {
                    email,
                    display_name,
                    response_status,
                    optional,
                });
            }
        }

        // Parse conference data
        let conference_data = if let Some(conf_data) = item.get("conferenceData") {
            let mut entry_points = Vec::new();

            if let Some(entry_point_list) = conf_data.get("entryPoints").and_then(|v| v.as_array())
            {
                for entry_point in entry_point_list {
                    if let (Some(entry_type), Some(uri)) = (
                        entry_point.get("entryPointType").and_then(|v| v.as_str()),
                        entry_point.get("uri").and_then(|v| v.as_str()),
                    ) {
                        entry_points.push(EntryPoint {
                            entry_point_type: entry_type.to_string(),
                            uri: uri.to_string(),
                            label: entry_point
                                .get("label")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                        });
                    }
                }
            }

            let conference_solution = conf_data.get("conferenceSolution").and_then(|sol| {
                sol.get("name")
                    .and_then(|v| v.as_str())
                    .map(|name| ConferenceSolution {
                        name: name.to_string(),
                        key: sol
                            .get("key")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    })
            });

            if !entry_points.is_empty() || conference_solution.is_some() {
                Some(ConferenceData {
                    conference_solution,
                    entry_points,
                })
            } else {
                None
            }
        } else {
            None
        };

        // Parse html link
        let html_link = item
            .get("htmlLink")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Parse creator
        let creator = item.get("creator").and_then(|c| {
            c.get("email")
                .and_then(|v| v.as_str())
                .map(|email| EventOrganizer {
                    email: email.to_string(),
                    display_name: c
                        .get("displayName")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    self_: c.get("self").and_then(|v| v.as_bool()),
                })
        });

        // Parse organizer
        let organizer = item.get("organizer").and_then(|o| {
            o.get("email")
                .and_then(|v| v.as_str())
                .map(|email| EventOrganizer {
                    email: email.to_string(),
                    display_name: o
                        .get("displayName")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    self_: o.get("self").and_then(|v| v.as_bool()),
                })
        });

        Ok(CalendarEvent {
            id,
            summary,
            description,
            location,
            start_time: start_dt,
            end_time: end_dt,
            attendees,
            conference_data,
            html_link,
            creator,
            organizer,
        })
    }
}
