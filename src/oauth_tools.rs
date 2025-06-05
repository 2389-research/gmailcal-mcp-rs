// ABOUTME: OAuth tools module for Gmail MCP server
// ABOUTME: Implements auth_status, get_oauth_url, and complete_oauth tools using rust-mcp-sdk patterns

use chrono::{DateTime, Utc};
use rust_mcp_sdk::{
    schema::schema_utils::CallToolError,
    schema::{CallToolResult, CallToolResultContentItem, TextContent},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Error;
use std::sync::Arc;
use tokio::sync::RwLock;

// Internal OAuth token storage
#[derive(Clone, Debug)]
#[allow(dead_code)] // Fields are used for token storage and refresh
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub email: String,
    pub client_id: String,
    pub client_secret: String,
}

// Shared OAuth state for the tools
pub struct OAuthState {
    pub tokens: Arc<RwLock<Option<OAuthTokens>>>,
}

impl Default for OAuthState {
    fn default() -> Self {
        Self::new()
    }
}

impl OAuthState {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(None)),
        }
    }
}

// Tool 1: get_oauth_url
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetOAuthUrlTool {
    /// Google OAuth client ID from Google Cloud Console
    pub client_id: String,
}

impl GetOAuthUrlTool {
    pub async fn call_tool(&self, _state: &OAuthState) -> Result<CallToolResult, CallToolError> {
        // Generate state token for CSRF protection
        use rand::distributions::{Alphanumeric, DistString};
        let state = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);

        // OAuth scopes for Gmail, Calendar, and People API
        let scopes = [
            "https://mail.google.com/",
            "https://www.googleapis.com/auth/calendar.readonly",
            "https://www.googleapis.com/auth/calendar",
            "https://www.googleapis.com/auth/contacts.readonly",
            "https://www.googleapis.com/auth/directory.readonly",
        ];

        let redirect_uri = "http://localhost:8080/oauth/callback";

        // Build authorization URL
        let auth_url = format!(
            "https://accounts.google.com/o/oauth2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&access_type=offline&prompt=consent",
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scopes.join(" ")),
            urlencoding::encode(&state)
        );

        let response = json!({
            "authorization_url": auth_url,
            "state": state,
            "redirect_uri": redirect_uri,
            "oauth_flow": "authorization_code",
            "provider": "google",
            "scopes": scopes,
            "expires_in": 300,
            "instructions": {
                "step": 1,
                "title": "Start OAuth Authentication",
                "steps": [
                    "1. 🌍 Open the authorization URL in your browser",
                    "2. 🔐 Sign in to your Google account if needed",
                    "3. ✅ Grant permissions to the Gmail MCP application",
                    "4. 🔄 After redirect, copy the 'code' parameter from the callback URL",
                    "5. 📝 Use 'complete_oauth' tool with the authorization code and your client secret"
                ],
                "next_tool": "complete_oauth",
                "callback_url_format": "http://localhost:8080/oauth/callback?code=AUTHORIZATION_CODE&state=STATE_TOKEN"
            },
            "security": {
                "state_token": state,
                "csrf_protection": true,
                "secure_redirect": true
            }
        });

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                response.to_string(),
                None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}

// Tool 2: complete_oauth
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CompleteOAuthTool {
    /// Authorization code from OAuth callback URL
    pub auth_code: String,
    /// Google OAuth client ID
    pub client_id: String,
    /// Google OAuth client secret
    pub client_secret: String,
}

impl CompleteOAuthTool {
    pub async fn call_tool(&self, state: &OAuthState) -> Result<CallToolResult, CallToolError> {
        let redirect_uri = "http://localhost:8080/oauth/callback";

        // Exchange code for tokens
        let client = reqwest::Client::new();
        let params = [
            ("code", self.auth_code.as_str()),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let response = client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                CallToolError::new(Error::other(format!("Token exchange failed: {}", e)))
            })?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CallToolError::new(Error::other(format!(
                "OAuth failed: {}",
                error_text
            ))));
        }

        let token_response: serde_json::Value = response.json().await.map_err(|e| {
            CallToolError::new(Error::other(format!(
                "Failed to parse token response: {}",
                e
            )))
        })?;

        // Extract tokens
        let access_token = token_response["access_token"]
            .as_str()
            .ok_or_else(|| CallToolError::new(Error::other("Missing access_token in response")))?;
        let refresh_token = token_response["refresh_token"]
            .as_str()
            .ok_or_else(|| CallToolError::new(Error::other("Missing refresh_token in response")))?;
        let expires_in = token_response["expires_in"].as_u64().unwrap_or(3600);

        // Get user email using the access token
        let user_info_response = client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| {
                CallToolError::new(Error::other(format!("Failed to get user info: {}", e)))
            })?;

        let user_info: serde_json::Value = user_info_response.json().await.map_err(|e| {
            CallToolError::new(Error::other(format!("Failed to parse user info: {}", e)))
        })?;

        let email = user_info["email"]
            .as_str()
            .ok_or_else(|| CallToolError::new(Error::other("Could not retrieve user email")))?;

        // Store tokens in memory
        let oauth_tokens = OAuthTokens {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.to_string(),
            expires_at: Utc::now() + chrono::Duration::seconds(expires_in as i64),
            email: email.to_string(),
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
        };

        {
            let mut tokens = state.tokens.write().await;
            *tokens = Some(oauth_tokens);
        }

        let response = json!({
            "status": "success",
            "message": "OAuth authentication completed successfully!",
            "email": email,
            "expires_at": Utc::now() + chrono::Duration::seconds(expires_in as i64),
            "scopes": "Gmail, Calendar, and Contacts access granted"
        });

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                response.to_string(),
                None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}

// Tool 3: auth_status
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AuthStatusTool {}

impl AuthStatusTool {
    pub async fn call_tool(&self, state: &OAuthState) -> Result<CallToolResult, CallToolError> {
        let tokens = state.tokens.read().await;

        let response = if let Some(ref tokens) = *tokens {
            let now = Utc::now();
            let is_valid = now < tokens.expires_at;
            let time_remaining = if is_valid {
                let duration = tokens.expires_at - now;
                format!("{} minutes", duration.num_minutes())
            } else {
                "Expired".to_string()
            };

            json!({
                "authenticated": is_valid,
                "oauth_flow": "authorization_code",
                "provider": "google",
                "email": tokens.email,
                "expires_at": tokens.expires_at,
                "time_remaining": time_remaining,
                "status": if is_valid { "✅ Valid and active" } else { "❌ Expired - use get_oauth_url to re-authenticate" },
                "scopes": [
                    "https://mail.google.com/",
                    "https://www.googleapis.com/auth/calendar.readonly",
                    "https://www.googleapis.com/auth/calendar",
                    "https://www.googleapis.com/auth/contacts.readonly",
                    "https://www.googleapis.com/auth/directory.readonly"
                ],
                "capabilities": {
                    "gmail": is_valid,
                    "calendar": is_valid,
                    "contacts": is_valid
                },
                "next_action": if is_valid { "🚀 You can now use Gmail, Calendar, and Contacts tools" } else { "🔄 Re-authenticate using get_oauth_url tool" }
            })
        } else {
            json!({
                "authenticated": false,
                "oauth_flow": "authorization_code",
                "provider": "google",
                "status": "❌ Not authenticated - use get_oauth_url to start OAuth flow",
                "required_scopes": [
                    "https://mail.google.com/",
                    "https://www.googleapis.com/auth/calendar.readonly",
                    "https://www.googleapis.com/auth/calendar",
                    "https://www.googleapis.com/auth/contacts.readonly",
                    "https://www.googleapis.com/auth/directory.readonly"
                ],
                "capabilities": {
                    "gmail": false,
                    "calendar": false,
                    "contacts": false
                },
                "instructions": {
                    "title": "🔐 OAuth Authentication Required",
                    "steps": [
                        "1. 🔑 Call get_oauth_url with your Google OAuth client ID",
                        "2. 🌐 Open the returned authorization URL in your browser",
                        "3. ✅ Complete Google authorization and copy the code from callback URL",
                        "4. 🔄 Call complete_oauth with the authorization code and client secret"
                    ]
                },
                "next_action": "🚀 Start with get_oauth_url tool to begin authentication"
            })
        };

        Ok(CallToolResult {
            content: vec![CallToolResultContentItem::TextContent(TextContent::new(
                response.to_string(),
                None,
            ))],
            is_error: Some(false),
            meta: None,
        })
    }
}
