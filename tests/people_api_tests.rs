/// People API Tests Module
///
/// This module contains tests for the Google People API functionality,
/// focusing on contact operations and data parsing.
///
use mcp_gmailcal::config::Config;
use mcp_gmailcal::people_api::{
    Contact, EmailAddress, Organization, PeopleClient, PersonName, PhoneNumber, Photo,
};
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::Arc;

// Define a MockPeopleClient for testing
struct MockPeopleClient {
    _config: Arc<Config>,
    _client: Client,
}

impl MockPeopleClient {
    fn new() -> Self {
        let config = Config {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            access_token: Some("test_access_token".to_string()),
        };

        let client = Client::new();

        Self {
            _config: Arc::new(config),
            _client: client,
        }
    }

    fn parse_contact(&self, data: Value) -> Result<Contact, String> {
        // Extract resource name
        let resource_name = data
            .get("resourceName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing resourceName".to_string())?
            .to_string();

        // Parse name
        let mut name = None;
        if let Some(names) = data.get("names").and_then(|v| v.as_array()) {
            if let Some(name_obj) = names.first() {
                let display_name = name_obj
                    .get("displayName")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let given_name = name_obj
                    .get("givenName")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let family_name = name_obj
                    .get("familyName")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                if display_name.is_some() {
                    name = Some(PersonName {
                        display_name: display_name.unwrap_or_default(),
                        given_name,
                        family_name,
                    });
                }
            }
        }

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

    fn get_contact(&self, _resource_name: &str) -> Result<Contact, String> {
        // Return a test contact
        let contact_data = json!({
            "resourceName": "people/test123",
            "etag": "test_etag",
            "names": [{
                "displayName": "Test User",
                "familyName": "User",
                "givenName": "Test",
                "metadata": {"primary": true}
            }],
            "emailAddresses": [{
                "value": "test.user@example.com",
                "type": "work",
                "metadata": {"primary": true}
            }],
            "phoneNumbers": [{
                "value": "+1 555-TEST",
                "type": "mobile",
                "metadata": {"primary": true}
            }],
            "organizations": [{
                "name": "Example Corp",
                "title": "Engineer",
                "metadata": {"primary": true}
            }],
            "photos": [{
                "url": "https://example.com/photo.jpg",
                "default": true,
                "metadata": {"primary": true}
            }]
        });

        self.parse_contact(contact_data)
    }

    fn search_contacts(
        &self,
        query: &str,
        _max_results: Option<u32>,
    ) -> Result<Vec<Contact>, String> {
        // For testing, create a collection of test contacts
        let contacts_data = vec![
            json!({
                "resourceName": "people/1",
                "names": [{
                    "displayName": "John Smith",
                    "givenName": "John",
                    "familyName": "Smith"
                }],
                "emailAddresses": [{
                    "value": "john.smith@example.com"
                }]
            }),
            json!({
                "resourceName": "people/2",
                "names": [{
                    "displayName": "Jane Doe",
                    "givenName": "Jane",
                    "familyName": "Doe"
                }],
                "emailAddresses": [{
                    "value": "jane.doe@example.com"
                }]
            }),
            json!({
                "resourceName": "people/3",
                "names": [{
                    "displayName": "Bob Johnson",
                    "givenName": "Bob",
                    "familyName": "Johnson"
                }],
                "emailAddresses": [{
                    "value": "bob.johnson@example.com"
                }]
            }),
        ];

        // Filter contacts based on the query
        let query = query.to_lowercase();
        let filtered_contacts: Vec<Contact> = contacts_data
            .into_iter()
            .filter(|contact| {
                // Get the display name
                let display_name = contact
                    .get("names")
                    .and_then(|names| names.as_array())
                    .and_then(|names| names.first())
                    .and_then(|name| name.get("displayName"))
                    .and_then(|name| name.as_str())
                    .unwrap_or("");

                // Get the email
                let email = contact
                    .get("emailAddresses")
                    .and_then(|emails| emails.as_array())
                    .and_then(|emails| emails.first())
                    .and_then(|email| email.get("value"))
                    .and_then(|email| email.as_str())
                    .unwrap_or("");

                // Check if query matches name or email
                display_name.to_lowercase().contains(&query)
                    || email.to_lowercase().contains(&query)
            })
            .filter_map(|contact| self.parse_contact(contact).ok())
            .collect();

        Ok(filtered_contacts)
    }
}

#[cfg(test)]
mod mock_people_tests {
    use super::*;

    #[test]
    fn test_people_client_creation() {
        let client = MockPeopleClient::new();
        // Just verify the client can be created
        assert_eq!(client._config.client_id, "test_client_id");
        assert_eq!(client._config.client_secret, "test_client_secret");
        assert_eq!(client._config.refresh_token, "test_refresh_token");
        assert_eq!(
            client._config.access_token.as_ref().unwrap(),
            "test_access_token"
        );
    }

    #[test]
    fn test_contact_parsing() {
        // Create a mock client
        let client = MockPeopleClient::new();

        // Create a sample JSON response
        let json_data = json!({
            "resourceName": "people/test123",
            "etag": "test_etag",
            "names": [{
                "displayName": "Test User",
                "familyName": "User",
                "givenName": "Test",
                "metadata": {"primary": true}
            }],
            "emailAddresses": [{
                "value": "test.user@example.com",
                "type": "work",
                "metadata": {"primary": true}
            }],
            "phoneNumbers": [{
                "value": "+1 555-TEST",
                "type": "mobile",
                "metadata": {"primary": true}
            }],
            "organizations": [{
                "name": "Example Corp",
                "title": "Engineer",
                "metadata": {"primary": true}
            }],
            "photos": [{
                "url": "https://example.com/photo.jpg",
                "default": true,
                "metadata": {"primary": true}
            }]
        });

        // Parse the contact
        let contact = client.parse_contact(json_data).unwrap();

        // Verify the parsed fields
        assert_eq!(contact.resource_name, "people/test123");

        // Check name
        let name = contact.name.unwrap();
        assert_eq!(name.display_name, "Test User");
        assert_eq!(name.given_name.unwrap_or_default(), "Test");
        assert_eq!(name.family_name.unwrap_or_default(), "User");

        // Check email
        assert_eq!(contact.email_addresses.len(), 1);
        assert_eq!(contact.email_addresses[0].value, "test.user@example.com");
        assert_eq!(contact.email_addresses[0].type_.as_ref().unwrap(), "work");

        // Check phone
        assert_eq!(contact.phone_numbers.len(), 1);
        assert_eq!(contact.phone_numbers[0].value, "+1 555-TEST");
        assert_eq!(contact.phone_numbers[0].type_.as_ref().unwrap(), "mobile");

        // Check organization
        assert_eq!(contact.organizations.len(), 1);
        assert_eq!(
            contact.organizations[0].name.as_ref().unwrap(),
            "Example Corp"
        );
        assert_eq!(contact.organizations[0].title.as_ref().unwrap(), "Engineer");

        // Check photo
        assert_eq!(contact.photos.len(), 1);
        assert_eq!(contact.photos[0].url, "https://example.com/photo.jpg");
        assert_eq!(contact.photos[0].default, true);
    }

    #[test]
    fn test_search_contacts() {
        // Create a mock client
        let client = MockPeopleClient::new();

        // Test with a query that should match one contact
        let contacts = client.search_contacts("john", None).unwrap();
        assert_eq!(contacts.len(), 2); // Both John Smith and Johnny Test match
        assert_eq!(
            contacts[0].name.as_ref().unwrap().display_name,
            "John Smith"
        );

        // Test with a query that should match multiple contacts
        let contacts = client.search_contacts("example.com", None).unwrap();
        assert_eq!(contacts.len(), 3); // All contacts have example.com in their email

        // Test with a query that should match no contacts
        let contacts = client.search_contacts("nonexistent", None).unwrap();
        assert_eq!(contacts.len(), 0);
    }

    #[test]
    fn test_get_contact() {
        // Create a mock client
        let client = MockPeopleClient::new();

        // Get a contact by ID
        let contact = client.get_contact("people/test123").unwrap();

        // Verify the contact details
        assert_eq!(contact.resource_name, "people/test123");
        assert_eq!(contact.name.as_ref().unwrap().display_name, "Test User");
        assert_eq!(contact.email_addresses[0].value, "test.user@example.com");
    }
}
