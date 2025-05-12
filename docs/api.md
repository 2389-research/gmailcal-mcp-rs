# API Reference

## MCP Protocol

This service implements the Model Context Protocol (MCP) for Gmail and Calendar integration.

## Endpoints

### Gmail API

#### List Messages
```json
{
  "operation": "gmail.messages.list",
  "params": {
    "maxResults": 10,
    "q": "from:example@gmail.com"
  }
}
```

#### Get Message
```json
{
  "operation": "gmail.messages.get",
  "params": {
    "id": "[MESSAGE_ID]"
  }
}
```

#### Create Draft
```json
{
  "operation": "gmail.drafts.create",
  "params": {
    "to": "recipient@example.com",
    "subject": "Message Subject",
    "body": "Message body content"
  }
}
```

### Calendar API

#### List Events
```json
{
  "operation": "calendar.events.list",
  "params": {
    "timeMin": "2024-01-01T00:00:00Z",
    "timeMax": "2024-01-31T23:59:59Z",
    "maxResults": 10
  }
}
```

#### Get Event
```json
{
  "operation": "calendar.events.get",
  "params": {
    "id": "[EVENT_ID]"
  }
}
```

#### Create Event
```json
{
  "operation": "calendar.events.create",
  "params": {
    "summary": "Meeting Title",
    "location": "Conference Room",
    "start": {
      "dateTime": "2024-01-15T09:00:00Z"
    },
    "end": {
      "dateTime": "2024-01-15T10:00:00Z"
    },
    "attendees": [
      {"email": "attendee@example.com"}
    ]
  }
}
```

### People API

#### List Contacts
```json
{
  "operation": "people.connections.list",
  "params": {
    "personFields": "names,emailAddresses,phoneNumbers",
    "pageSize": 10
  }
}
```

## Authentication

All API requests require OAuth2 authentication with the appropriate Google API scopes.
