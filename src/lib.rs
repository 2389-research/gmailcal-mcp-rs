// Error exports
pub use crate::errors::{
    ConfigError, GmailApiError, PeopleApiError, CalendarApiError,
    GmailResult, PeopleResult, CalendarResult, error_codes
};
pub use crate::config::{Config, GMAIL_API_BASE_URL, OAUTH_TOKEN_URL, get_token_expiry_seconds};
pub use crate::gmail_api::{EmailMessage, DraftEmail, GmailService};
pub use crate::logging::setup_logging;
pub use crate::auth::TokenManager;
pub use crate::people_api::{
    Contact, EmailAddress, Organization, PeopleClient, 
    PersonName, PhoneNumber, Photo, ContactList
};
pub use crate::calendar_api::{
    CalendarClient, CalendarEvent, CalendarList, CalendarInfo,
    Attendee, EventOrganizer, ConferenceData, ConferenceSolution, EntryPoint
};
pub use crate::prompts::*;
pub use crate::utils::{
    parse_max_results, decode_base64, encode_base64_url_safe, 
    to_mcp_error, map_gmail_error, error_codes as utils_error_codes
};

// Module for error handling
pub mod errors;
// Module for configuration
pub mod config;
// Module for logging
pub mod logging;
// Module for utilities
pub mod utils;
// Module for authentication
pub mod auth;
// Module for Gmail API
pub mod gmail_api;
// Module for People API
pub mod people_api;
// Module for Calendar API
pub mod calendar_api;
// Module for prompt templates
pub mod prompts;
// Module for server implementation
pub mod server;
/// Gmail MCP Server Implementation
///
/// This crate provides an MCP (Model Completion Protocol) server for Gmail,
/// allowing Claude to read emails from a Gmail account.
///
/// # Features
///
/// - List emails from inbox
/// - Search emails using Gmail search queries
/// - Get details for a specific email
/// - List email labels
/// - Check connection status
///
/// # Testing
///
/// The crate includes unit tests for internal functions and integration tests
/// for testing the MCP commands. Future improvements could include more
/// sophisticated mocking of the Gmail API and more comprehensive tests.
///
// Re-export key types for use in tests
pub use crate::server::GmailServer;

// OAuth authentication module for token refresh flow
pub mod oauth {
    use crate::config::Config;
    use axum::extract::Query;
    use axum::response::Html;
    use axum::routing::get;
    use axum::Router;
    use dotenv::dotenv;
    use log::error;
    use rand::distributions::{Alphanumeric, DistString};
    use serde::{Deserialize, Serialize};
    use serde_json;
    use std::collections::HashMap;
    use std::env;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::net::SocketAddr;
    use std::path::Path;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use url::Url;

    // OAuth scopes needed for Gmail, Calendar, and People API access
    const GMAIL_SCOPE: &str = "https://mail.google.com/";
    const CALENDAR_READ_SCOPE: &str = "https://www.googleapis.com/auth/calendar.readonly";
    const CALENDAR_WRITE_SCOPE: &str = "https://www.googleapis.com/auth/calendar";
    const CONTACTS_READ_SCOPE: &str = "https://www.googleapis.com/auth/contacts.readonly";
    const DIRECTORY_READ_SCOPE: &str = "https://www.googleapis.com/auth/directory.readonly";
    const OAUTH_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/auth";
    const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

    // Local server config
    const DEFAULT_PORT: u16 = 8080;
    const DEFAULT_HOST: &str = "127.0.0.1";

    // Structure to hold the OAuth state
    #[derive(Clone, Debug, Default)]
    struct OAuthState {
        auth_code: Option<String>,
        state_token: Option<String>,
        complete: bool,
    }

    // Structure for OAuth authorization parameters
    #[derive(Debug, Serialize)]
    #[allow(dead_code)]
    struct AuthParams {
        client_id: String,
        redirect_uri: String,
        response_type: String,
        scope: String,
        state: String,
        access_type: String,
        prompt: String,
    }

    // Structure for the callback query parameters
    #[derive(Debug, Deserialize)]
    struct CallbackParams {
        code: Option<String>,
        state: Option<String>,
        error: Option<String>,
    }

    // Structure for the token response
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct TokenResponse {
        access_token: String,
        expires_in: u64,
        refresh_token: String,
        token_type: String,
        scope: Option<String>,
    }

    // Run the OAuth flow to get a new refresh token
    pub async fn run_oauth_flow() -> Result<(), String> {
        // Attempt to load existing credentials
        let _ = dotenv();

        // Get client ID and secret from environment or prompt user
        let client_id = env::var("GMAIL_CLIENT_ID").unwrap_or_else(|_| {
            println!("Enter your Google OAuth client ID:");
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input");
            input.trim().to_string()
        });

        let client_secret = env::var("GMAIL_CLIENT_SECRET").unwrap_or_else(|_| {
            println!("Enter your Google OAuth client secret:");
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input");
            input.trim().to_string()
        });

        // Generate a random state token for CSRF protection
        let state_token = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);

        // Set up the redirect URI for the local callback server
        let port = env::var("OAUTH_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(DEFAULT_PORT);

        let host = env::var("OAUTH_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
        let redirect_uri = format!("http://{}:{}/oauth/callback", host, port);

        // Create a shared state to store the authorization code
        let oauth_state = Arc::new(Mutex::new(OAuthState {
            auth_code: None,
            state_token: Some(state_token.clone()),
            complete: false,
        }));

        // Build the authorization URL with Gmail, Calendar, and People API scopes
        let auth_url = build_auth_url(
            &client_id,
            &redirect_uri,
            &state_token,
            &[
                GMAIL_SCOPE.to_string(),
                CALENDAR_READ_SCOPE.to_string(),
                CALENDAR_WRITE_SCOPE.to_string(),
                CONTACTS_READ_SCOPE.to_string(),
                DIRECTORY_READ_SCOPE.to_string(),
            ],
        )?;

        // Start the local web server to handle the OAuth callback
        let server_handle = start_oauth_server(port, host.clone(), oauth_state.clone());

        // Open the authorization URL in the default browser
        println!("Opening browser to authorize with Google...");
        println!("\nAuthorization URL: {}", auth_url);

        if let Err(e) = webbrowser::open(&auth_url) {
            println!("Failed to open web browser automatically: {}", e);
            println!("Please manually open the URL in your browser to continue.");
        }

        // Wait for the authorization to complete
        println!("Waiting for authorization...");
        let auth_code = wait_for_auth_code(oauth_state).await?;

        // Exchange the authorization code for tokens
        println!("Exchanging authorization code for tokens...");
        let tokens =
            exchange_code_for_tokens(&client_id, &client_secret, &auth_code, &redirect_uri).await?;

        // Update the .env file with the new tokens
        println!("Updating credentials in .env file...");
        update_env_file(
            &client_id,
            &client_secret,
            &tokens.refresh_token,
            &tokens.access_token,
            &redirect_uri,
        )?;

        // Shut down the server
        server_handle.abort();

        println!("\n🎉 Authentication successful!");
        println!("✅ New tokens have been saved to .env file");
        println!("✅ Claude Desktop config saved to claude_desktop_config.json");

        Ok(())
    }

    // Build the authorization URL
    fn build_auth_url(
        client_id: &str,
        redirect_uri: &str,
        state: &str,
        scopes: &[String],
    ) -> Result<String, String> {
        let mut url = Url::parse(OAUTH_AUTH_URL).map_err(|e| e.to_string())?;

        // Add required OAuth parameters
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("client_id", client_id);
            query.append_pair("redirect_uri", redirect_uri);
            query.append_pair("response_type", "code");
            query.append_pair("scope", &scopes.join(" "));
            query.append_pair("state", state);
            query.append_pair("access_type", "offline");
            query.append_pair("prompt", "consent"); // Ensure we always get a refresh token
            query.finish();
        }

        // Return the URL
        Ok(url.to_string())
    }

    // Start a local web server to handle the OAuth callback
    fn start_oauth_server(
        port: u16,
        host: String,
        state: Arc<Mutex<OAuthState>>,
    ) -> tokio::task::JoinHandle<()> {
        // Create the router with callback and index routes
        let app = Router::new()
            .route(
                "/",
                get(|| async {
                    Html("<h1>Gmail OAuth Server</h1><p>Waiting for OAuth callback...</p>")
                }),
            )
            .route(
                "/oauth/callback",
                get(move |query| handle_callback(query, state.clone())),
            );

        // Start the server in a background task
        tokio::spawn(async move {
            let addr = format!("{host}:{port}").parse::<SocketAddr>().unwrap();
            println!("\nStarting OAuth callback server on http://{host}:{port}");

            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            if let Err(e) = axum::serve(listener, app).await {
                error!("Server error: {}", e);
            }
        })
    }

    // Handle the OAuth callback from Google
    async fn handle_callback(
        Query(params): Query<CallbackParams>,
        state: Arc<Mutex<OAuthState>>,
    ) -> Html<String> {
        let mut oauth_state = state.lock().await;

        // Check for errors
        if let Some(error) = params.error {
            oauth_state.complete = true;
            return Html(format!(
                "<html>
<head><title>OAuth Error</title></head>
<body>
    <h1>OAuth Error</h1>
    <p>An error occurred during authentication: {}</p>
    <p>Please close this window and try again.</p>
</body>
</html>",
                error
            ));
        }

        // Check state token to prevent CSRF attacks
        if params.state != oauth_state.state_token {
            oauth_state.complete = true;
            return Html(
                "<html>
<head><title>Authentication Failed</title></head>
<body>
    <h1>Authentication Failed</h1>
    <p>Invalid state parameter. This could be a CSRF attack attempt.</p>
    <p>Please close this window and try again.</p>
</body>
</html>"
                    .to_string(),
            );
        }

        // Store the authorization code
        if let Some(code) = params.code {
            oauth_state.auth_code = Some(code);
            oauth_state.complete = true;

            // Return success page
            Html(
                "<html>
<head>
    <title>Authentication Successful</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; }
        h1 { color: #4285f4; }
        .success { color: green; }
    </style>
</head>
<body>
    <h1>Gmail OAuth Authentication</h1>
    <h2 class=\"success\">Authentication Successful! ✅</h2>
    <p>You have successfully authenticated with Google.</p>
    <p>You can now close this window and return to the application.</p>
</body>
</html>"
                    .to_string(),
            )
        } else {
            oauth_state.complete = true;

            // Missing authorization code
            Html(
                "<html>
<head><title>Authentication Failed</title></head>
<body>
    <h1>Authentication Failed</h1>
    <p>No authorization code received from Google.</p>
    <p>Please close this window and try again.</p>
</body>
</html>"
                    .to_string(),
            )
        }
    }

    // Wait for the authorization code to be received
    async fn wait_for_auth_code(state: Arc<Mutex<OAuthState>>) -> Result<String, String> {
        // Poll for the authorization code with a timeout
        let max_wait_seconds = 300; // 5 minutes
        let poll_interval = std::time::Duration::from_secs(1);

        for _ in 0..max_wait_seconds {
            let oauth_state = state.lock().await;

            // Check if we have the authorization code
            if let Some(code) = oauth_state.auth_code.clone() {
                return Ok(code);
            }

            // Check if the flow completed with an error
            if oauth_state.complete {
                return Err("Authorization failed. Check the browser for details.".to_string());
            }

            // Release the lock and wait before trying again
            drop(oauth_state);
            tokio::time::sleep(poll_interval).await;
        }

        Err("Timed out waiting for authorization. Please try again.".to_string())
    }

    // Exchange the authorization code for access and refresh tokens
    async fn exchange_code_for_tokens(
        client_id: &str,
        client_secret: &str,
        auth_code: &str,
        redirect_uri: &str,
    ) -> Result<TokenResponse, String> {
        let client = reqwest::Client::new();

        // Prepare the token request parameters
        let params = [
            ("code", auth_code),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        // Make the token request
        let response = client
            .post(OAUTH_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Failed to exchange code for tokens: {}", e))?;

        // Check for error responses
        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<no response body>".to_string());

            return Err(format!(
                "Failed to exchange code for tokens. Status: {}, Error: {}",
                status, error_text
            ));
        }

        // Parse the token response
        let tokens: TokenResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse token response: {}", e))?;

        Ok(tokens)
    }

    // Update the .env file with the new tokens and generate Claude Desktop config
    fn update_env_file(
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
        access_token: &str,
        redirect_uri: &str,
    ) -> Result<(), String> {
        // Check if .env file exists
        let env_path = ".env";
        let env_exists = Path::new(env_path).exists();

        // Create or update the .env file
        if env_exists {
            // Read existing .env content
            let content = std::fs::read_to_string(env_path)
                .map_err(|e| format!("Failed to read .env file: {}", e))?;

            // Create a backup of the .env file
            let backup_path = format!(
                ".env.backup.{}",
                chrono::Local::now().format("%Y%m%d_%H%M%S")
            );
            std::fs::write(&backup_path, &content)
                .map_err(|e| format!("Failed to create backup file {}: {}", backup_path, e))?;
            println!("✅ Created backup of .env file at {}", backup_path);

            // Ask for confirmation before proceeding
            println!("⚠️ About to update .env file with new OAuth credentials.");
            println!("🔄 Press Enter to continue or Ctrl+C to abort...");
            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).is_err() {
                println!("❌ Failed to read input, continuing anyway");
            }

            // Parse the content into a HashMap
            let mut env_vars = HashMap::new();
            for line in content.lines() {
                // Skip comments and empty lines
                if line.starts_with('#') || line.trim().is_empty() {
                    continue;
                }

                // Parse key-value pairs
                if let Some(pos) = line.find('=') {
                    let key = line[..pos].trim().to_string();
                    let value = line[pos + 1..].trim().to_string();
                    env_vars.insert(key, value);
                }
            }

            // Update the values
            env_vars.insert("GMAIL_CLIENT_ID".to_string(), client_id.to_string());
            env_vars.insert("GMAIL_CLIENT_SECRET".to_string(), client_secret.to_string());
            env_vars.insert("GMAIL_REFRESH_TOKEN".to_string(), refresh_token.to_string());
            env_vars.insert("GMAIL_ACCESS_TOKEN".to_string(), access_token.to_string());
            env_vars.insert("GMAIL_REDIRECT_URI".to_string(), redirect_uri.to_string());

            // Build the new content
            let mut new_content = String::new();
            new_content.push_str("# Gmail API OAuth2 credentials\n");
            for (key, value) in &env_vars {
                new_content.push_str(&format!("{key}={value}\n"));
            }

            // Write the updated content back to the file
            std::fs::write(env_path, new_content)
                .map_err(|e| format!("Failed to write to .env file: {}", e))?;
        } else {
            // Create a new .env file
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(env_path)
                .map_err(|e| format!("Failed to create .env file: {}", e))?;

            // Write the credentials
            writeln!(file, "# Gmail API OAuth2 credentials")
                .map_err(|e| format!("Failed to write to .env file: {}", e))?;
            writeln!(file, "GMAIL_CLIENT_ID={}", client_id)
                .map_err(|e| format!("Failed to write to .env file: {}", e))?;
            writeln!(file, "GMAIL_CLIENT_SECRET={}", client_secret)
                .map_err(|e| format!("Failed to write to .env file: {}", e))?;
            writeln!(file, "GMAIL_REFRESH_TOKEN={}", refresh_token)
                .map_err(|e| format!("Failed to write to .env file: {}", e))?;
            writeln!(file, "GMAIL_ACCESS_TOKEN={}", access_token)
                .map_err(|e| format!("Failed to write to .env file: {}", e))?;
            writeln!(file, "GMAIL_REDIRECT_URI={}", redirect_uri)
                .map_err(|e| format!("Failed to write to .env file: {}", e))?;
        }

        // Also generate the Claude Desktop config file
        generate_claude_desktop_config(client_id, client_secret, refresh_token, access_token)
            .map_err(|e| format!("Failed to create Claude Desktop config: {}", e))?;

        Ok(())
    }

    // Generate the Claude Desktop configuration file
    fn generate_claude_desktop_config(
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
        access_token: &str,
    ) -> Result<(), String> {
        use serde_json::{json, to_string_pretty};

        // Determine the executable path
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?;

        // Get the target/release version of the path if possible
        let mut command_path = current_exe.to_string_lossy().to_string();
        if let Some(debug_index) = command_path.find("target/debug") {
            // If we're running in debug mode, use the release path for the config
            command_path = format!(
                "{}target/release/mcp-gmailcal",
                &command_path[0..debug_index]
            );
        }

        // Create the config JSON
        let config = json!({
            "mcpServers": {
                "gmailcal": {
                    "command": command_path,
                    "args": ["--memory-only"],
                    "env": {
                        "GMAIL_CLIENT_ID": client_id,
                        "GMAIL_CLIENT_SECRET": client_secret,
                        "GMAIL_REFRESH_TOKEN": refresh_token,
                        "GMAIL_ACCESS_TOKEN": access_token
                    }
                }
            }
        });

        // Convert to pretty JSON
        let json_string =
            to_string_pretty(&config).map_err(|e| format!("Failed to serialize config: {}", e))?;

        // Write to file
        let config_path = "claude_desktop_config.json";
        std::fs::write(config_path, json_string)
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        println!("Claude Desktop config saved to {}", config_path);

        Ok(())
    }

    // Utility to test the saved credentials
    pub async fn test_credentials() -> Result<String, String> {
        // Load the config from environment
        let config =
            Config::from_env().map_err(|e| format!("Failed to load credentials: {}", e))?;

        // Create a Gmail service client
        let mut service = crate::gmail_api::GmailService::new(&config)
            .map_err(|e| format!("Failed to create Gmail service: {}", e))?;

        // Try to check the connection
        match service.check_connection().await {
            Ok((email, count)) => Ok(format!(
                "Successfully connected to Gmail for {}! Found {} messages.",
                email, count
            )),
            Err(e) => Err(format!("Failed to connect to Gmail: {}", e)),
        }
    }
}

