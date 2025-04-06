use crate::auth::TokenManager;
use crate::config::Config;
use crate::errors::{PeopleApiError, PeopleResult};
use log::{debug, error};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

const PEOPLE_API_BASE_URL: &str = "https://people.googleapis.com/v1";

// Alias for backward compatibility within this module
type Result<T> = PeopleResult<T>;

// Contact information representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub resource_name: String,
    pub name: Option<PersonName>,
    pub email_addresses: Vec<EmailAddress>,
    pub phone_numbers: Vec<PhoneNumber>,
    pub organizations: Vec<Organization>,
    pub photos: Vec<Photo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonName {
    pub display_name: String,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddress {
    pub value: String,
    pub type_: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneNumber {
    pub value: String,
    pub type_: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub name: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photo {
    pub url: String,
    pub default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactList {
    pub contacts: Vec<Contact>,
    pub next_page_token: Option<String>,
    pub total_items: Option<u32>,
}

// People API client
#[derive(Debug, Clone)]
pub struct PeopleClient {
    client: Client,
    token_manager: Arc<Mutex<TokenManager>>,
}

impl PeopleClient {
    pub fn new(config: &Config) -> Self {
        let client = Client::new();
        // Reuse the Gmail token manager since they share the same OAuth flow
        let token_manager = Arc::new(Mutex::new(TokenManager::new(config)));

        Self {
            client,
            token_manager,
        }
    }

    // Get a list of contacts
    pub async fn list_contacts(&self, max_results: Option<u32>) -> Result<ContactList> {
        let token = self
            .token_manager
            .lock()
            .await
            .get_token(&self.client)
            .await
            .map_err(|e| PeopleApiError::AuthError(e.to_string()))?;

        let mut url = format!("{}/people/me/connections", PEOPLE_API_BASE_URL);

        // Build query parameters
        let mut query_parts = Vec::new();

        // Request specific fields
        let fields = [
            "names",
            "emailAddresses",
            "phoneNumbers",
            "organizations",
            "photos",
        ];
        query_parts.push(format!("personFields={}", fields.join(",")));

        if let Some(max) = max_results {
            query_parts.push(format!("pageSize={}", max));
        }

        if !query_parts.is_empty() {
            url = format!("{}?{}", url, query_parts.join("&"));
        }

        debug!("Listing contacts from: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| PeopleApiError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<no response body>".to_string());
            return Err(PeopleApiError::ApiError(format!(
                "Failed to list contacts. Status: {}, Error: {}",
                status, error_text
            )));
        }

        let json_response = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| PeopleApiError::ParseError(e.to_string()))?;

        let mut contacts = Vec::new();

        if let Some(connections) = json_response.get("connections").and_then(|v| v.as_array()) {
            for connection in connections {
                if let Ok(contact) = self.parse_contact(connection) {
                    contacts.push(contact);
                } else {
                    // Log parsing error but continue with other contacts
                    error!("Failed to parse contact: {:?}", connection);
                }
            }
        }

        let next_page_token = json_response
            .get("nextPageToken")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let total_items = json_response
            .get("totalItems")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32);

        Ok(ContactList {
            contacts,
            next_page_token,
            total_items,
        })
    }

    // Search contacts by query
    pub async fn search_contacts(
        &self,
        query: &str,
        max_results: Option<u32>,
    ) -> Result<ContactList> {
        let token = self
            .token_manager
            .lock()
            .await
            .get_token(&self.client)
            .await
            .map_err(|e| PeopleApiError::AuthError(e.to_string()))?;

        let mut url = format!("{}/people:searchContacts", PEOPLE_API_BASE_URL);

        // Build query parameters
        let mut query_parts = Vec::new();

        // Add search query
        query_parts.push(format!("query={}", query));

        // Request specific fields
        let fields = [
            "names",
            "emailAddresses",
            "phoneNumbers",
            "organizations",
            "photos",
        ];
        query_parts.push(format!("readMask={}", fields.join(",")));

        if let Some(max) = max_results {
            query_parts.push(format!("pageSize={}", max));
        }

        if !query_parts.is_empty() {
            url = format!("{}?{}", url, query_parts.join("&"));
        }

        debug!("Searching contacts: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| PeopleApiError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<no response body>".to_string());
            return Err(PeopleApiError::ApiError(format!(
                "Failed to search contacts. Status: {}, Error: {}",
                status, error_text
            )));
        }

        let json_response = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| PeopleApiError::ParseError(e.to_string()))?;

        let mut contacts = Vec::new();

        if let Some(results) = json_response.get("results").and_then(|v| v.as_array()) {
            for result in results {
                if let Some(person) = result.get("person") {
                    if let Ok(contact) = self.parse_contact(person) {
                        contacts.push(contact);
                    } else {
                        // Log parsing error but continue with other contacts
                        error!("Failed to parse contact: {:?}", person);
                    }
                }
            }
        }

        let next_page_token = json_response
            .get("nextPageToken")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let total_items = json_response
            .get("totalPeople")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32);

        Ok(ContactList {
            contacts,
            next_page_token,
            total_items,
        })
    }

    // Get contact by resource name
    pub async fn get_contact(&self, resource_name: &str) -> Result<Contact> {
        let token = self
            .token_manager
            .lock()
            .await
            .get_token(&self.client)
            .await
            .map_err(|e| PeopleApiError::AuthError(e.to_string()))?;

        let mut url = format!("{}/{}", PEOPLE_API_BASE_URL, resource_name);

        // Build query parameters for fields
        let fields = [
            "names",
            "emailAddresses",
            "phoneNumbers",
            "organizations",
            "photos",
        ];
        url = format!("{}?personFields={}", url, fields.join(","));

        debug!("Getting contact: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| PeopleApiError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<no response body>".to_string());
            return Err(PeopleApiError::ApiError(format!(
                "Failed to get contact. Status: {}, Error: {}",
                status, error_text
            )));
        }

        let json_response = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| PeopleApiError::ParseError(e.to_string()))?;

        self.parse_contact(&json_response)
    }

    // Helper method to parse a contact from API response
    fn parse_contact(&self, data: &serde_json::Value) -> Result<Contact> {
        let resource_name = data
            .get("resourceName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PeopleApiError::ParseError("Missing resourceName".to_string()))?
            .to_string();

        // Parse name
        let name = if let Some(names) = data.get("names").and_then(|v| v.as_array()) {
            if let Some(primary_name) = names.first() {
                let display_name = primary_name
                    .get("displayName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();

                let given_name = primary_name
                    .get("givenName")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let family_name = primary_name
                    .get("familyName")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                Some(PersonName {
                    display_name,
                    given_name,
                    family_name,
                })
            } else {
                None
            }
        } else {
            None
        };

        // Parse email addresses
        let mut email_addresses = Vec::new();
        if let Some(emails) = data.get("emailAddresses").and_then(|v| v.as_array()) {
            for email in emails {
                if let Some(value) = email.get("value").and_then(|v| v.as_str()) {
                    let type_ = email
                        .get("type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    email_addresses.push(EmailAddress {
                        value: value.to_string(),
                        type_,
                    });
                }
            }
        }

        // Parse phone numbers
        let mut phone_numbers = Vec::new();
        if let Some(phones) = data.get("phoneNumbers").and_then(|v| v.as_array()) {
            for phone in phones {
                if let Some(value) = phone.get("value").and_then(|v| v.as_str()) {
                    let type_ = phone
                        .get("type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    phone_numbers.push(PhoneNumber {
                        value: value.to_string(),
                        type_,
                    });
                }
            }
        }

        // Parse organizations
        let mut organizations = Vec::new();
        if let Some(orgs) = data.get("organizations").and_then(|v| v.as_array()) {
            for org in orgs {
                let name = org
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let title = org
                    .get("title")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                organizations.push(Organization { name, title });
            }
        }

        // Parse photos
        let mut photos = Vec::new();
        if let Some(pics) = data.get("photos").and_then(|v| v.as_array()) {
            for pic in pics {
                if let Some(url) = pic.get("url").and_then(|v| v.as_str()) {
                    let default = pic
                        .get("default")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    photos.push(Photo {
                        url: url.to_string(),
                        default,
                    });
                }
            }
        }

        Ok(Contact {
            resource_name,
            name,
            email_addresses,
            phone_numbers,
            organizations,
            photos,
        })
    }
}
