# 📧 Gmail MCP Server

Welcome to the **Gmail MCP Server**! This is a Model Completion Protocol (MCP) server designed to interact seamlessly with the Gmail API, empowering Claude to read and manage emails from your Gmail account. 🚀

## 📜 Summary of Project

The Gmail MCP Server is built on Rust, providing a robust and efficient interface to the Gmail API. Through this server, users can perform various functionalities like:
- Listing emails from their inbox 📬
- Searching for emails using Gmail search queries 🔍
- Getting details of specific emails 📑
- Analyzing email content for action items, meetings, contacts, and more 📊
- Batch analyzing multiple emails for quick triage 📋
- Listing all email labels 🏷️
- Checking connection status with the Gmail API 📡

This server enhances Claude's email analysis capabilities with specialized prompts for email analysis, summarization, task extraction, meeting detection, contact extraction, prioritization, and more.

## ⚙️ How to Use

To utilize the Gmail MCP Server, follow these steps:

### 1. Create a Google Cloud Project and Enable the Gmail API
- Go to the [Google Cloud Console](https://console.cloud.google.com/)
- Create a new project.
- Enable the Gmail API.
- Create OAuth2 credentials (Client ID & Client Secret).
- Configure the OAuth consent screen and add necessary scopes.

### 2. Get a Refresh Token
To obtain a refresh token:
- Use the OAuth 2.0 authorization flow.
- Request access to the necessary Gmail API scopes.
- You can utilize the [Google OAuth 2.0 Playground](https://developers.google.com/oauthplayground/) for token generation.

### 3. Configure Environment Variables
In the same directory as the executable, create a `.env` file with:
```
GMAIL_CLIENT_ID=your-client-id
GMAIL_CLIENT_SECRET=your-client-secret
GMAIL_REFRESH_TOKEN=your-refresh-token
```

### 4. Build and Run the MCP Server
To compile and run the server, execute:
```bash
cargo build --release
./target/release/mcp-gmailcal
```

#### Running in Read-Only Environments (e.g., Claude)
When running in read-only environments like Claude AI, use the `--memory-only` flag to prevent file system writes:
```bash
cargo run -- --memory-only
# or
./target/release/mcp-gmailcal --memory-only
```

This will use in-memory logging (via stderr) instead of attempting to write log files to disk.

### 5. Configure Claude to Use the MCP Server
1. Add the MCP server via Claude Code CLI:
   ```bash
   claude mcp add
   ```
2. Follow prompts to integrate your Gmail MCP server.

### 🛠 Usage in Claude
You can directly use tools through commands like:
```
/tool list_emails max_results=5
/tool search_emails query="from:example.com after:2024/01/01" max_results=10
/tool get_email message_id=18c1eab45a2d0123
/tool analyze_email message_id=18c1eab45a2d0123 analysis_type="tasks"
/tool batch_analyze_emails message_ids=["18c1eab45a2d0123", "18c1eab45a2d0456"] analysis_type="summary"
/tool list_labels
/tool check_connection
```

Or through natural language requests:
- "Check my Gmail connection status"
- "Show me my 5 most recent unread emails"
- "Search for emails from example.com sent this year"
- "Get the details of email with ID 18c1eab45a2d0123"
- "Analyze this email for action items and deadlines"
- "Extract meeting details from these emails"
- "Summarize these 3 emails for me"
- "Find all contact information in this email"
- "Help me prioritize these emails"

## 📝 Advanced Email Analysis

The Gmail MCP Server provides specialized analysis capabilities through a set of custom prompts that help Claude understand and extract insights from emails:

### Analysis Types
- **General Email Analysis**: Comprehensive analysis of email content, context, tone, and next steps
- **Task Extraction**: Identifies explicit and implicit action items, deadlines, and responsibilities
- **Meeting Detection**: Extracts meeting details including date, time, location, participants, and agenda items
- **Contact Extraction**: Identifies and formats contact information from email content
- **Email Summarization**: Creates concise summaries of emails based on their length and complexity
- **Email Categorization**: Classifies emails into categories like Action Required, FYI, Follow-up, etc.
- **Email Prioritization**: Assesses urgency and importance of emails for better inbox management
- **Email Drafting Assistance**: Guidelines for writing effective emails for different purposes

### Using Analysis Features
- Individual analysis: `analyze_email message_id="..." analysis_type="tasks|meetings|contacts|summary|priority|all"`
- Batch analysis: `batch_analyze_emails message_ids=["id1", "id2", "id3"] analysis_type="summary"`

These analysis features help users quickly understand email content, extract important information, and take appropriate actions without having to read through lengthy messages.

## 🔧 Tech Info

- **Language**: Rust
- **Primary Dependencies**:
  - `mcp-attr` - For MCP server implementation
  - `tokio` - Asynchronous runtime
  - `reqwest` - HTTP client for Gmail API
  - `serde` and `serde_json` - For JSON serialization
  - `dotenv` - For environment variable management
  - `log`, `simplelog`, and `chrono` - For logging functionality
- **Testing**: Includes a comprehensive suite of unit and integration tests to ensure reliability and performance.

### Project Structure
```
src/
  ├── lib.rs          # Main implementation including Gmail API client and MCP server
  ├── main.rs         # Command-line interface and server startup
  ├── config.rs       # Configuration handling
  ├── gmail_api.rs    # Gmail API client implementation
  ├── logging.rs      # Logging setup
  ├── server.rs       # MCP server implementation
  └── prompts.rs      # Email analysis prompts
tests/
  └── integration_tests.rs  # Integration tests for MCP commands
.env.example          # Example environment variables
.gitignore            # Git ignore configuration
Cargo.toml            # Rust dependencies
mcp-test.sh           # MCP test script
mise.toml             # Development environment configuration
README.md             # This file
test_mcp_server/      # Test utilities for the MCP server
```

### Note:
Make sure to handle sensitive information with care as some files may contain credentials or tokens. Always refer to the official Gmail API documentation for the latest updates and practices.

Thank you for your interest in the Gmail MCP Server! If you have any questions or feedback, feel free to reach out or contribute to the repository! Happy coding! 🎉
