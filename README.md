# 📧 Gmail MCP Server

Welcome to the **Gmail MCP Server**! This is a Model Completion Protocol (MCP) server designed to interact seamlessly with the Gmail API, empowering Claude to read and manage emails from your Gmail account. 🚀

## 📜 Summary of Project

The Gmail MCP Server is built on Rust, providing a robust and efficient interface to the Gmail API. Through this server, users can perform various functionalities like:
- Listing emails from their inbox 📬
- Searching for emails using Gmail search queries 🔍
- Getting details of specific emails 📑
- Listing all email labels 🏷️
- Checking connection status with the Gmail API 📡

This server is perfect for developers looking to integrate email functionalities into their applications or automate email handling.

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
/tool list_labels
/tool check_connection
```

Or through natural language requests:
- "Check my Gmail connection status"
- "Show me my 5 most recent unread emails"
- "Search for emails from example.com sent this year"
- "Get the details of email with ID 18c1eab45a2d0123"

## 🔧 Tech Info

- **Language**: Rust
- **Primary Dependencies**:
  - `gmail` - For Gmail API integration
  - `tokio` - Asynchronous runtime
  - `serde` and `serde_json` - For JSON serialization
  - `dotenv` - For environment variable management
  - `log`, `simplelog`, and `chrono` - For logging functionality
- **Testing**: Includes a comprehensive suite of unit and integration tests to ensure reliability and performance.

### Project Structure
```
src/
  ├── lib.rs
  └── main.rs
tests/
  └── integration_tests.rs
.env.example
.gitignore
Cargo.toml
gmail-rs-guide.md
mise.toml
proper_test.sh
README.md
test_list_emails.sh
```

### Note:
Make sure to handle sensitive information with care as some files may contain credentials or tokens. Always refer to the official Gmail API documentation for the latest updates and practices.

Thank you for your interest in the Gmail MCP Server! If you have any questions or feedback, feel free to reach out or contribute to the repository! Happy coding! 🎉
