#!/bin/bash

# Start the SSE server in background
echo "Starting SSE server..."
cargo run -- --transport sse --port 8080 &
SERVER_PID=$!

# Wait for server to start
sleep 3

# Test SSE endpoint
echo "Testing SSE endpoint..."
curl -N -H "Accept: text/event-stream" -H "Cache-Control: no-cache" http://127.0.0.1:8080/sse &
CURL_PID=$!

# Wait a moment to see if we get any data
sleep 5

# Test messages endpoint
echo "Testing messages endpoint..."
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  "http://127.0.0.1:8080/messages?session_id=test"

# Clean up
kill $CURL_PID 2>/dev/null
kill $SERVER_PID 2>/dev/null

echo "Test completed"
