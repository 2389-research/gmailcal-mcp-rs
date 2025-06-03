#!/bin/bash

# Test script to check what protocol version the server supports
echo "Testing MCP server protocol version..."

# Send an initialize request with the 2024-11-05 protocol version
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{"sampling":{},"roots":{"listChanged":true}},"clientInfo":{"name":"test-client","version":"1.0.0"}}}' | cargo run 2>/dev/null | head -1

echo ""
echo "Testing with 2025-03-26 protocol version..."

# Send an initialize request with the 2025-03-26 protocol version
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{"sampling":{},"roots":{"listChanged":true}},"clientInfo":{"name":"test-client","version":"1.0.0"}}}' | cargo run 2>/dev/null | head -1
