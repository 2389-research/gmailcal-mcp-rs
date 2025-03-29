#!/bin/bash

# Set colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Clean up old log files (optional - uncomment to enable)
# echo -e "${YELLOW}Cleaning up old log files...${NC}"
# rm -f gmail_mcp_*.log

# Build the application in debug mode with verbose output
echo -e "${BLUE}Building the application...${NC}"
RUSTFLAGS="-C debuginfo=2" cargo build || { echo "Build failed"; exit 1; }

# Create a JSON-RPC request for the list_emails command with a small result limit
JSON_REQUEST='{"jsonrpc":"2.0","id":"test-1","method":"tool","params":{"name":"list_emails","input":{"max_results":3}}}'

# Run the application with the JSON request
echo -e "${GREEN}Running list_emails command with max_results=3...${NC}"
echo -e "${YELLOW}Request:${NC} $JSON_REQUEST"
echo $JSON_REQUEST | RUST_LOG=trace ./target/debug/mcp-gmailcal

# Get the most recent log file
LOG_FILE=$(ls -t gmail_mcp_*.log 2>/dev/null | head -1)

if [ -z "$LOG_FILE" ]; then
    echo -e "${YELLOW}No log file found! Logging might not be working.${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}Test completed. Log file: ${BLUE}$LOG_FILE${NC}"

# Show log file size
LOG_SIZE=$(du -h "$LOG_FILE" | cut -f1)
echo -e "${YELLOW}Log file size: ${LOG_SIZE}${NC}"

# Count log entries by level
if [ -f "$LOG_FILE" ]; then
    echo -e "${YELLOW}Log entries by level:${NC}"
    ERROR_COUNT=$(grep -c "ERROR" "$LOG_FILE")
    WARN_COUNT=$(grep -c "WARN" "$LOG_FILE")
    INFO_COUNT=$(grep -c "INFO" "$LOG_FILE")
    DEBUG_COUNT=$(grep -c "DEBUG" "$LOG_FILE")
    TRACE_COUNT=$(grep -c "TRACE" "$LOG_FILE")
    DIRECT_COUNT=$(grep -c "DIRECT" "$LOG_FILE")
    
    echo -e "${YELLOW}ERROR:${NC} $ERROR_COUNT"
    echo -e "${YELLOW}WARN:${NC}  $WARN_COUNT"
    echo -e "${YELLOW}INFO:${NC}  $INFO_COUNT"
    echo -e "${YELLOW}DEBUG:${NC} $DEBUG_COUNT"
    echo -e "${YELLOW}TRACE:${NC} $TRACE_COUNT"
    echo -e "${YELLOW}DIRECT:${NC} $DIRECT_COUNT"
fi

# Display the first few lines of the log file
echo -e "\n${BLUE}First 10 lines of log file:${NC}"
head -n 10 "$LOG_FILE"

echo -e "\n${BLUE}Last 10 lines of log file:${NC}"
tail -n 10 "$LOG_FILE"

echo -e "\n${GREEN}To view the full log, run: ${BLUE}less $LOG_FILE${NC}"
