use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mcp_gmailcal::{
    auth::TokenManager,
    calendar_api::{Attendee, CalendarEvent},
    config::Config,
    errors::{CalendarApiError, GmailApiError, PeopleApiError},
    gmail_api::{DraftEmail, EmailMessage},
    utils::{decode_base64, encode_base64_url_safe, parse_max_results},
};
use serde_json::{self, json};

// Mock data for benchmarks
fn create_mock_email() -> EmailMessage {
    EmailMessage {
        id: "msg123456".to_string(),
        thread_id: "thread123456".to_string(),
        subject: Some("Test Email Subject".to_string()),
        from: Some("sender@example.com".to_string()),
        to: Some("recipient@example.com".to_string()),
        date: Some("2023-09-15T15:30:00Z".to_string()),
        snippet: Some("This is a preview of the email content...".to_string()),
        body_text: Some("This is the plain text body of the email.".to_string()),
        body_html: Some("<div>This is the <b>HTML</b> body of the email.</div>".to_string()),
    }
}

fn create_mock_draft() -> DraftEmail {
    DraftEmail {
        to: "recipient@example.com".to_string(),
        subject: "Draft Email Subject".to_string(),
        body: "This is the body of the draft email.".to_string(),
        cc: Some("cc@example.com".to_string()),
        bcc: Some("bcc@example.com".to_string()),
        thread_id: Some("thread123456".to_string()),
        in_reply_to: Some("<original-message@example.com>".to_string()),
        references: Some(
            "<original-message@example.com> <another-message@example.com>".to_string(),
        ),
    }
}

fn create_mock_calendar_event() -> CalendarEvent {
    CalendarEvent {
        id: Some("event123456".to_string()),
        summary: "Sample Calendar Event".to_string(),
        description: Some("This is a description of the event".to_string()),
        location: Some("Conference Room A".to_string()),
        start_time: chrono::Utc::now(),
        end_time: chrono::Utc::now() + chrono::Duration::hours(1),
        attendees: vec![
            Attendee {
                email: "attendee1@example.com".to_string(),
                display_name: Some("Attendee One".to_string()),
                response_status: Some("accepted".to_string()),
                optional: None,
            },
            Attendee {
                email: "attendee2@example.com".to_string(),
                display_name: Some("Attendee Two".to_string()),
                response_status: Some("needsAction".to_string()),
                optional: None,
            },
        ],
        conference_data: None,
        html_link: None,
        creator: None,
        organizer: None,
    }
}

fn mock_config() -> Config {
    Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        refresh_token: "test_refresh_token".to_string(),
        access_token: Some("test_access_token".to_string()),
    }
}

// Create various error instances
fn create_mock_errors() -> (GmailApiError, CalendarApiError, PeopleApiError) {
    let gmail_error = GmailApiError::ApiError("Test Gmail API Error".to_string());
    let calendar_error = CalendarApiError::ApiError("Test Calendar API Error".to_string());
    let people_error = PeopleApiError::ApiError("Test People API Error".to_string());

    (gmail_error, calendar_error, people_error)
}

// Benchmark email parsing
fn email_parsing_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("Email Parsing");

    // Benchmark email serialization
    let email = create_mock_email();
    group.bench_function("email_serialization", |b| {
        b.iter(|| black_box(serde_json::to_string(&email)))
    });

    // Benchmark email deserialization
    let email_json = serde_json::to_string(&email).unwrap();
    group.bench_function("email_deserialization", |b| {
        b.iter(|| black_box(serde_json::from_str::<EmailMessage>(&email_json)))
    });

    // Benchmark draft email serialization
    let draft = create_mock_draft();
    group.bench_function("draft_email_serialization", |b| {
        b.iter(|| black_box(serde_json::to_string(&draft)))
    });

    // Benchmark calendar event serialization
    let event = create_mock_calendar_event();
    group.bench_function("calendar_event_serialization", |b| {
        b.iter(|| black_box(serde_json::to_string(&event)))
    });

    group.finish();
}

// Benchmark token operations
fn token_operations_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("Token Operations");

    // Benchmark token manager creation
    let config = mock_config();
    group.bench_function("token_manager_creation", |b| {
        b.iter(|| black_box(TokenManager::new(&config)))
    });

    // Benchmark token manager clone operation
    let token_manager = TokenManager::new(&config);
    group.bench_function("token_manager_clone", |b| {
        b.iter(|| black_box(token_manager.clone()))
    });

    // We can't directly benchmark token expiry check since expiry is private
    // Instead, we'll measure the overhead of creating a token manager with differing
    // initial states (with and without access token)
    let config_with_token = mock_config();
    let config_without_token = Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        refresh_token: "test_refresh_token".to_string(),
        access_token: None,
    };

    group.bench_function("token_manager_with_token", |b| {
        b.iter(|| black_box(TokenManager::new(&config_with_token)))
    });

    group.bench_function("token_manager_without_token", |b| {
        b.iter(|| black_box(TokenManager::new(&config_without_token)))
    });

    group.finish();
}

// Benchmark error handling
fn error_handling_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("Error Handling");

    let (gmail_error, calendar_error, people_error) = create_mock_errors();

    // Benchmark Gmail API error formatting
    group.bench_function("gmail_error_display", |b| {
        b.iter(|| black_box(format!("{}", gmail_error)))
    });

    // Benchmark Calendar API error formatting
    group.bench_function("calendar_error_display", |b| {
        b.iter(|| black_box(format!("{}", calendar_error)))
    });

    // Benchmark People API error formatting
    group.bench_function("people_error_display", |b| {
        b.iter(|| black_box(format!("{}", people_error)))
    });

    group.finish();
}

// Benchmark utility functions
fn utility_functions_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("Utility Functions");

    // Benchmark base64 encoding
    let data = b"This is some test data for base64 encoding and decoding benchmarks";
    group.bench_function("base64_encoding", |b| {
        b.iter(|| black_box(encode_base64_url_safe(data)))
    });

    // Benchmark base64 decoding
    let encoded = encode_base64_url_safe(data);
    group.bench_function("base64_decoding", |b| {
        b.iter(|| black_box(decode_base64(&encoded)))
    });

    // Benchmark max results parsing with number
    let number_value = json!(10);
    group.bench_function("parse_max_results_number", |b| {
        b.iter(|| black_box(parse_max_results(Some(number_value.clone()), 5)))
    });

    // Benchmark max results parsing with string
    let string_value = json!("15");
    group.bench_function("parse_max_results_string", |b| {
        b.iter(|| black_box(parse_max_results(Some(string_value.clone()), 5)))
    });

    // Benchmark max results parsing with invalid value
    let invalid_value = json!(null);
    group.bench_function("parse_max_results_invalid", |b| {
        b.iter(|| black_box(parse_max_results(Some(invalid_value.clone()), 5)))
    });

    group.finish();
}

// Benchmark search operations
fn search_operations_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("Search Operations");

    // Benchmark searching through a list of emails (simulated with iteration)
    let emails: Vec<EmailMessage> = (0..100)
        .map(|i| {
            let mut email = create_mock_email();
            email.id = format!("msg{}", i);
            email.subject = Some(format!("Subject {}", i));
            email
        })
        .collect();

    group.bench_function("search_emails_by_subject", |b| {
        b.iter(|| {
            black_box(
                emails
                    .iter()
                    .filter(|e| e.subject.as_ref().map_or(false, |s| s.contains("50")))
                    .count(),
            )
        })
    });

    // Benchmark searching through a list of events (simulated with iteration)
    let events: Vec<CalendarEvent> = (0..50)
        .map(|i| {
            let mut event = create_mock_calendar_event();
            event.id = Some(format!("event{}", i));
            event.summary = format!("Event Summary {}", i);
            event
        })
        .collect();

    group.bench_function("search_events_by_summary", |b| {
        b.iter(|| black_box(events.iter().filter(|e| e.summary.contains("25")).count()))
    });

    group.finish();
}

// Setup for API request simulation benchmarks
fn api_request_simulation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("API Request Simulation");
    group.sample_size(100); // Reduce sample size for simulated API calls

    // Simulate API latency by adding a small delay
    group.bench_function("simulate_api_request_success", |b| {
        b.iter_with_setup(
            || {},
            |_| {
                // Simulate processing before/after API call
                let result: Result<String, GmailApiError> = Ok("API Response Data".to_string());
                black_box(result)
            },
        )
    });

    // Simulate error handling after API failure
    group.bench_function("simulate_api_request_error", |b| {
        b.iter_with_setup(
            || {},
            |_| {
                // Simulate error handling
                let error = GmailApiError::NetworkError("Connection timeout".to_string());
                let result: Result<String, GmailApiError> = Err(error);

                // Common error handling pattern - provide fallback or handle error
                black_box(result.unwrap_or_else(|e| format!("Error: {}", e)))
            },
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    email_parsing_benchmarks,
    token_operations_benchmarks,
    error_handling_benchmarks,
    utility_functions_benchmarks,
    search_operations_benchmarks,
    api_request_simulation_benchmarks,
);
criterion_main!(benches);
