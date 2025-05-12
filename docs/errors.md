# Error Handling

## Error Types

The application uses a centralized error handling approach through the `errors` module. The main error type is `GmailCalError` which encompasses various error scenarios.

### Error Categories

- **Gmail Errors**: Issues with Gmail API operations
- **Calendar Errors**: Issues with Calendar API operations
- **Authentication Errors**: Problems with OAuth tokens or authentication flow
- **Request Errors**: Invalid request formats or parameters
- **Server Errors**: Internal server problems

## Error Codes

Errors return appropriate HTTP status codes based on the error type:

- 400: Bad Request - Invalid parameters or request format
- 401: Unauthorized - Missing or invalid authentication
- 403: Forbidden - Insufficient permissions
- 404: Not Found - Requested resource not found
- 500: Internal Server Error - Unexpected server issues

## Error Response Format

```json
{
  "error": {
    "message": "Error description",
    "code": 400,
    "details": "Additional error details"
  }
}
```
