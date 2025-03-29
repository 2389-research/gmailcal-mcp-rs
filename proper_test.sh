#\!/bin/bash

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Build the application
echo -e "${BLUE}Building the application...${NC}"
RUSTFLAGS="-C debuginfo=2" cargo build || { echo "Build failed"; exit 1; }

# Create and start a simple fifo for communication
FIFO="/tmp/mcp_gmailcal_fifo"
rm -f $FIFO
mkfifo $FIFO

# Start the MCP server in the background, reading from the FIFO
echo -e "${GREEN}Starting MCP server...${NC}"
cat $FIFO | RUST_LOG=trace ./target/debug/mcp-gmailcal > mcp_output.log &
SERVER_PID=$\!

# Give server time to start
sleep 1

# Send initialization message first (required by MCP protocol)
echo -e "${YELLOW}Sending initialization message...${NC}"
INIT_MSG='{"jsonrpc":"2.0","id":"init","method":"initialize","params":{"capabilities":{},"clientInfo":{"name":"test-client","version":"1.0"},"protocolVersion":"0.1","trace":"off"}}'
echo $INIT_MSG > $FIFO

# Wait for server to process initialization
sleep 2

# Send the list_emails request
echo -e "${YELLOW}Sending list_emails request...${NC}"
LIST_MSG='{"jsonrpc":"2.0","id":"test-1","method":"tool","params":{"name":"list_emails","input":{"max_results":3}}}'
echo $LIST_MSG > $FIFO

# Wait for output
sleep 5

# Kill the server
echo -e "${BLUE}Cleaning up...${NC}"
kill $SERVER_PID
rm -f $FIFO

# Show the output
echo -e "${GREEN}Server output:${NC}"
cat mcp_output.log

# Get the latest log file
LOG_FILE=$(ls -t gmail_mcp_*.log 2>/dev/null | head -1)

if [ -f "$LOG_FILE" ]; then
  echo -e "\n${GREEN}Checking log file: ${BLUE}$LOG_FILE${NC}"
  echo -e "${YELLOW}Looking for errors related to internalDate...${NC}"
  grep -i "internalDate\|error\|exception" "$LOG_FILE" | grep -v "DIRECT:"
fi

echo -e "\n${GREEN}Test completed.${NC}"
