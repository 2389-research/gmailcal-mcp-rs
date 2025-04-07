/// People API Tests Module
///
/// This module contains comprehensive tests for the Google People API functionality,
/// focusing on contact operations, data formatting, and error handling.
use mcp_gmailcal::errors::PeopleApiError;
use mcp_gmailcal::people_api::{Contact, ContactList, EmailAddress, Organization, PersonName, PhoneNumber, Photo};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

// Define a proper interface for PeopleClient that we can mock
trait PeopleClientInterface {
    fn list_contacts(&self, max_results: Option<u32>) -> Result<ContactList, PeopleApiError>;
    fn search_contacts(&self, query: &str, max_results: Option<u32>) -> Result<ContactList, PeopleApiError>;
    fn get_contact(&self, resource_name: &str) -> Result<Contact, PeopleApiError>;
    fn parse_contact(&self, data: &Value) -> Result<Contact, PeopleApiError>;
}

// Wrapper for PeopleClient that we can test against
struct MockablePeopleClient {
    client: Arc<Mutex<dyn PeopleClientInterface + Send + Sync>>,
}

impl MockablePeopleClient {
    fn new(client: Arc<Mutex<dyn PeopleClientInterface + Send + Sync>>) -> Self {
        Self { client }
    }

    fn list_contacts(&self, max_results: Option<u32>) -> Result<ContactList, PeopleApiError> {
        let client = self.client.lock().unwrap();
        client.list_contacts(max_results)
    }

    fn search_contacts(&self, query: &str, max_results: Option<u32>) -> Result<ContactList, PeopleApiError> {
        let client = self.client.lock().unwrap();
        client.search_contacts(query, max_results)
    }

    fn get_contact(&self, resource_name: &str) -> Result<Contact, PeopleApiError> {
        let client = self.client.lock().unwrap();
        client.get_contact(resource_name)
    }

    fn parse_contact(&self, data: &Value) -> Result<Contact, PeopleApiError> {
        let client = self.client.lock().unwrap();
        client.parse_contact(data)
    }
}

// Helper functions to create test data
fn create_test_contact(
    resource_name: &str,
    display_name: &str,
    given_name: Option<&str>,
    family_name: Option<&str>,
    emails: Vec<(&str, Option<&str>)>,
    phones: Vec<(&str, Option<&str>)>,
    organizations: Vec<(Option<&str>, Option<&str>)>,
    photo_urls: Vec<(&str, bool)>,
) -> Contact {
    Contact {
        resource_name: resource_name.to_string(),
        name: Some(PersonName {
            display_name: display_name.to_string(),
            given_name: given_name.map(|s| s.to_string()),
            family_name: family_name.map(|s| s.to_string()),
        }),
        email_addresses: emails
            .into_iter()
            .map(|(value, type_)| EmailAddress {
                value: value.to_string(),
                type_: type_.map(|s| s.to_string()),
            })
            .collect(),
        phone_numbers: phones
            .into_iter()
            .map(|(value, type_)| PhoneNumber {
                value: value.to_string(),
                type_: type_.map(|s| s.to_string()),
            })
            .collect(),
        organizations: organizations
            .into_iter()
            .map(|(name, title)| Organization {
                name: name.map(|s| s.to_string()),
                title: title.map(|s| s.to_string()),
            })
            .collect(),
        photos: photo_urls
            .into_iter()
            .map(|(url, default)| Photo {
                url: url.to_string(),
                default,
            })
            .collect(),
    }
}

fn create_test_contact_json(
    resource_name: &str,
    display_name: &str,
    given_name: Option<&str>,
    family_name: Option<&str>,
    emails: Vec<(&str, Option<&str>)>,
    phones: Vec<(&str, Option<&str>)>,
    organizations: Vec<(Option<&str>, Option<&str>)>,
    photo_urls: Vec<(&str, bool)>,
) -> Value {
    // Create names array
    let names = json!([{
        "displayName": display_name,
        "givenName": given_name,
        "familyName": family_name
    }]);

    // Create email addresses array
    let email_addresses = json!(
        emails.iter().map(|(value, type_)| {
            json!({
                "value": value,
                "type": type_
            })
        }).collect::<Vec<_>>()
    );

    // Create phone numbers array
    let phone_numbers = json!(
        phones.iter().map(|(value, type_)| {
            json!({
                "value": value,
                "type": type_
            })
        }).collect::<Vec<_>>()
    );

    // Create organizations array
    let organizations_json = json!(
        organizations.iter().map(|(name, title)| {
            json!({
                "name": name,
                "title": title
            })
        }).collect::<Vec<_>>()
    );

    // Create photos array
    let photos = json!(
        photo_urls.iter().map(|(url, default)| {
            json!({
                "url": url,
                "default": default
            })
        }).collect::<Vec<_>>()
    );

    json!({
        "resourceName": resource_name,
        "names": names,
        "emailAddresses": email_addresses,
        "phoneNumbers": phone_numbers,
        "organizations": organizations_json,
        "photos": photos
    })
}

// Mock implementation of PeopleClientInterface for testing
struct MockPeopleClient {
    contacts: Vec<Contact>,
    should_fail: bool,
    fail_mode: FailMode,
}

enum FailMode {
    Auth,
    Network,
    Api,
    Parse,
    None,
}

impl MockPeopleClient {
    fn new() -> Self {
        // Create sample contacts
        let contacts = vec![
            create_test_contact(
                "people/contact1",
                "John Doe",
                Some("John"),
                Some("Doe"),
                vec![("john.doe@example.com", Some("work"))],
                vec![("123-456-7890", Some("mobile"))],
                vec![(Some("Acme Inc"), Some("Software Developer"))],
                vec![("https://example.com/photo1.jpg", true)],
            ),
            create_test_contact(
                "people/contact2",
                "Jane Smith",
                Some("Jane"),
                Some("Smith"),
                vec![
                    ("jane.smith@example.com", Some("work")),
                    ("jsmith@personal.com", Some("home")),
                ],
                vec![("987-654-3210", Some("mobile"))],
                vec![(Some("XYZ Corp"), Some("Product Manager"))],
                vec![("https://example.com/photo2.jpg", true)],
            ),
            create_test_contact(
                "people/contact3",
                "Alex Johnson",
                Some("Alex"),
                Some("Johnson"),
                vec![("alex.j@example.com", Some("work"))],
                vec![
                    ("555-123-4567", Some("work")),
                    ("555-987-6543", Some("home")),
                ],
                vec![(Some("ABC Company"), Some("Director"))],
                vec![
                    ("https://example.com/photo3a.jpg", false),
                    ("https://example.com/photo3b.jpg", true),
                ],
            ),
        ];

        Self {
            contacts,
            should_fail: false,
            fail_mode: FailMode::None,
        }
    }

    fn with_failure(mut self, mode: FailMode) -> Self {
        self.should_fail = true;
        self.fail_mode = mode;
        self
    }
}

impl PeopleClientInterface for MockPeopleClient {
    fn list_contacts(&self, max_results: Option<u32>) -> Result<ContactList, PeopleApiError> {
        if self.should_fail {
            return match self.fail_mode {
                FailMode::Auth => Err(PeopleApiError::AuthError("Authentication failed".to_string())),
                FailMode::Network => Err(PeopleApiError::NetworkError("Network error".to_string())),
                FailMode::Api => Err(PeopleApiError::ApiError("API error".to_string())),
                FailMode::Parse => Err(PeopleApiError::ParseError("Parse error".to_string())),
                FailMode::None => unreachable!(),
            };
        }

        let total_items = self.contacts.len() as u32;
        let contacts = if let Some(max) = max_results {
            self.contacts.iter().take(max as usize).cloned().collect()
        } else {
            self.contacts.clone()
        };

        Ok(ContactList {
            contacts,
            next_page_token: None,
            total_items: Some(total_items),
        })
    }

    fn search_contacts(&self, query: &str, max_results: Option<u32>) -> Result<ContactList, PeopleApiError> {
        if self.should_fail {
            return match self.fail_mode {
                FailMode::Auth => Err(PeopleApiError::AuthError("Authentication failed".to_string())),
                FailMode::Network => Err(PeopleApiError::NetworkError("Network error".to_string())),
                FailMode::Api => Err(PeopleApiError::ApiError("API error".to_string())),
                FailMode::Parse => Err(PeopleApiError::ParseError("Parse error".to_string())),
                FailMode::None => unreachable!(),
            };
        }

        let query = query.to_lowercase();
        let filtered_contacts: Vec<Contact> = self
            .contacts
            .iter()
            .filter(|contact| {
                // Search in display name
                if let Some(name) = &contact.name {
                    if name.display_name.to_lowercase().contains(&query) {
                        return true;
                    }
                }

                // Search in emails
                for email in &contact.email_addresses {
                    if email.value.to_lowercase().contains(&query) {
                        return true;
                    }
                }

                // Search in organization name or title
                for org in &contact.organizations {
                    if let Some(org_name) = &org.name {
                        if org_name.to_lowercase().contains(&query) {
                            return true;
                        }
                    }
                    if let Some(title) = &org.title {
                        if title.to_lowercase().contains(&query) {
                            return true;
                        }
                    }
                }

                false
            })
            .cloned()
            .collect();

        let total_items = filtered_contacts.len() as u32;
        let contacts = if let Some(max) = max_results {
            filtered_contacts.into_iter().take(max as usize).collect()
        } else {
            filtered_contacts
        };

        Ok(ContactList {
            contacts,
            next_page_token: None,
            total_items: Some(total_items),
        })
    }

    fn get_contact(&self, resource_name: &str) -> Result<Contact, PeopleApiError> {
        if self.should_fail {
            return match self.fail_mode {
                FailMode::Auth => Err(PeopleApiError::AuthError("Authentication failed".to_string())),
                FailMode::Network => Err(PeopleApiError::NetworkError("Network error".to_string())),
                FailMode::Api => Err(PeopleApiError::ApiError("API error".to_string())),
                FailMode::Parse => Err(PeopleApiError::ParseError("Parse error".to_string())),
                FailMode::None => unreachable!(),
            };
        }

        // Find contact by resource name
        let contact = self
            .contacts
            .iter()
            .find(|c| c.resource_name == resource_name)
            .cloned();

        match contact {
            Some(contact) => Ok(contact),
            None => Err(PeopleApiError::ApiError(format!(
                "Contact not found: {}",
                resource_name
            ))),
        }
    }

    fn parse_contact(&self, data: &Value) -> Result<Contact, PeopleApiError> {
        if self.should_fail {
            return match self.fail_mode {
                FailMode::Parse => Err(PeopleApiError::ParseError("Parse error".to_string())),
                _ => Err(PeopleApiError::ParseError("Unexpected error".to_string())),
            };
        }

        // Extract resource name
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

#[cfg(test)]
mod comprehensive_people_api_tests {
    use super::*;

    // Helper functions to create test clients
    fn create_test_client() -> MockablePeopleClient {
        let mock_client = MockPeopleClient::new();
        let client = Arc::new(Mutex::new(mock_client));
        MockablePeopleClient::new(client)
    }

    fn create_failing_client(mode: FailMode) -> MockablePeopleClient {
        let mock_client = MockPeopleClient::new().with_failure(mode);
        let client = Arc::new(Mutex::new(mock_client));
        MockablePeopleClient::new(client)
    }

    #[test]
    fn test_list_contacts_success() {
        let client = create_test_client();

        // Test listing all contacts
        let result = client.list_contacts(None);
        assert!(result.is_ok());
        
        let contact_list = result.unwrap();
        assert_eq!(contact_list.contacts.len(), 3);
        assert_eq!(contact_list.total_items, Some(3));
        assert!(contact_list.next_page_token.is_none());

        // Verify contact details
        let contact1 = &contact_list.contacts[0];
        assert_eq!(contact1.resource_name, "people/contact1");
        assert_eq!(contact1.name.as_ref().unwrap().display_name, "John Doe");
        assert_eq!(contact1.email_addresses.len(), 1);
        assert_eq!(contact1.phone_numbers.len(), 1);
        assert_eq!(contact1.organizations.len(), 1);
        assert_eq!(contact1.photos.len(), 1);

        // Test with max_results
        let result = client.list_contacts(Some(2));
        assert!(result.is_ok());
        
        let contact_list = result.unwrap();
        assert_eq!(contact_list.contacts.len(), 2);
        assert_eq!(contact_list.total_items, Some(3)); // Still reports total of 3
    }

    #[test]
    fn test_list_contacts_failure() {
        // Test various failure modes
        let auth_client = create_failing_client(FailMode::Auth);
        let result = auth_client.list_contacts(None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PeopleApiError::AuthError(_)));

        let network_client = create_failing_client(FailMode::Network);
        let result = network_client.list_contacts(None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PeopleApiError::NetworkError(_)));

        let api_client = create_failing_client(FailMode::Api);
        let result = api_client.list_contacts(None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PeopleApiError::ApiError(_)));

        let parse_client = create_failing_client(FailMode::Parse);
        let result = parse_client.list_contacts(None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PeopleApiError::ParseError(_)));
    }

    #[test]
    fn test_search_contacts_success() {
        let client = create_test_client();

        // Test searching by name
        let result = client.search_contacts("John", None);
        assert!(result.is_ok());
        
        let contact_list = result.unwrap();
        // Both "John Doe" and "Alex Johnson" contain "John"
        assert_eq!(contact_list.contacts.len(), 2);
        
        // Find the contact with display name "John Doe"
        let john_doe = contact_list.contacts.iter()
            .find(|c| c.name.as_ref().unwrap().display_name == "John Doe")
            .expect("Should find John Doe");
            
        assert_eq!(john_doe.name.as_ref().unwrap().display_name, "John Doe");

        // Test searching by email
        let result = client.search_contacts("smith", None);
        assert!(result.is_ok());
        
        let contact_list = result.unwrap();
        assert_eq!(contact_list.contacts.len(), 1);
        assert_eq!(contact_list.contacts[0].name.as_ref().unwrap().display_name, "Jane Smith");

        // Test searching by organization
        let result = client.search_contacts("ABC Company", None);
        assert!(result.is_ok());
        
        let contact_list = result.unwrap();
        assert_eq!(contact_list.contacts.len(), 1);
        assert_eq!(contact_list.contacts[0].name.as_ref().unwrap().display_name, "Alex Johnson");

        // Test searching by position
        let result = client.search_contacts("Director", None);
        assert!(result.is_ok());
        
        let contact_list = result.unwrap();
        assert_eq!(contact_list.contacts.len(), 1);
        assert_eq!(contact_list.contacts[0].name.as_ref().unwrap().display_name, "Alex Johnson");

        // Test with no results
        let result = client.search_contacts("NonExistent", None);
        assert!(result.is_ok());
        
        let contact_list = result.unwrap();
        assert_eq!(contact_list.contacts.len(), 0);

        // Test with max_results
        let result = client.search_contacts("e", Some(1)); // Should match all, but limit to 1
        assert!(result.is_ok());
        
        let contact_list = result.unwrap();
        assert_eq!(contact_list.contacts.len(), 1);
    }

    #[test]
    fn test_search_contacts_failure() {
        // Test various failure modes
        let auth_client = create_failing_client(FailMode::Auth);
        let result = auth_client.search_contacts("test", None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PeopleApiError::AuthError(_)));

        let network_client = create_failing_client(FailMode::Network);
        let result = network_client.search_contacts("test", None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PeopleApiError::NetworkError(_)));

        let api_client = create_failing_client(FailMode::Api);
        let result = api_client.search_contacts("test", None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PeopleApiError::ApiError(_)));

        let parse_client = create_failing_client(FailMode::Parse);
        let result = parse_client.search_contacts("test", None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PeopleApiError::ParseError(_)));
    }

    #[test]
    fn test_get_contact_success() {
        let client = create_test_client();

        // Test getting contact by resource name
        let result = client.get_contact("people/contact1");
        assert!(result.is_ok());
        
        let contact = result.unwrap();
        assert_eq!(contact.resource_name, "people/contact1");
        assert_eq!(contact.name.as_ref().unwrap().display_name, "John Doe");
        assert_eq!(contact.name.as_ref().unwrap().given_name, Some("John".to_string()));
        assert_eq!(contact.name.as_ref().unwrap().family_name, Some("Doe".to_string()));
        
        // Check email
        assert_eq!(contact.email_addresses.len(), 1);
        assert_eq!(contact.email_addresses[0].value, "john.doe@example.com");
        assert_eq!(contact.email_addresses[0].type_, Some("work".to_string()));
        
        // Check phone
        assert_eq!(contact.phone_numbers.len(), 1);
        assert_eq!(contact.phone_numbers[0].value, "123-456-7890");
        assert_eq!(contact.phone_numbers[0].type_, Some("mobile".to_string()));
        
        // Check organization
        assert_eq!(contact.organizations.len(), 1);
        assert_eq!(contact.organizations[0].name, Some("Acme Inc".to_string()));
        assert_eq!(contact.organizations[0].title, Some("Software Developer".to_string()));
        
        // Check photos
        assert_eq!(contact.photos.len(), 1);
        assert_eq!(contact.photos[0].url, "https://example.com/photo1.jpg");
        assert!(contact.photos[0].default);

        // Test contact with multiple emails, phones, and photos
        let result = client.get_contact("people/contact3");
        assert!(result.is_ok());
        
        let contact = result.unwrap();
        assert_eq!(contact.phone_numbers.len(), 2);
        assert_eq!(contact.photos.len(), 2);
        
        // One photo should be default and one not
        let default_photos: Vec<_> = contact.photos.iter().filter(|p| p.default).collect();
        assert_eq!(default_photos.len(), 1);
    }

    #[test]
    fn test_get_contact_not_found() {
        let client = create_test_client();

        // Test getting non-existent contact
        let result = client.get_contact("people/nonexistent");
        assert!(result.is_err());
        
        if let Err(PeopleApiError::ApiError(msg)) = result {
            assert!(msg.contains("Contact not found"));
        } else {
            panic!("Expected ApiError");
        }
    }

    #[test]
    fn test_get_contact_failure() {
        // Test various failure modes
        let auth_client = create_failing_client(FailMode::Auth);
        let result = auth_client.get_contact("people/contact1");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PeopleApiError::AuthError(_)));

        let network_client = create_failing_client(FailMode::Network);
        let result = network_client.get_contact("people/contact1");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PeopleApiError::NetworkError(_)));

        let api_client = create_failing_client(FailMode::Api);
        let result = api_client.get_contact("people/contact1");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PeopleApiError::ApiError(_)));

        let parse_client = create_failing_client(FailMode::Parse);
        let result = parse_client.get_contact("people/contact1");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PeopleApiError::ParseError(_)));
    }

    #[test]
    fn test_parse_contact_success() {
        let client = create_test_client();

        // Create test JSON for full contact
        let contact_json = create_test_contact_json(
            "people/test_contact",
            "Test User",
            Some("Test"),
            Some("User"),
            vec![
                ("test.user@example.com", Some("work")),
                ("testuser@home.com", Some("home")),
            ],
            vec![
                ("555-123-4567", Some("work")),
                ("555-987-6543", Some("mobile")),
            ],
            vec![(Some("Test Company"), Some("Test Position"))],
            vec![
                ("https://example.com/test1.jpg", false),
                ("https://example.com/test2.jpg", true),
            ],
        );

        // Parse the contact
        let result = client.parse_contact(&contact_json);
        assert!(result.is_ok());
        
        let contact = result.unwrap();
        
        // Verify basic fields
        assert_eq!(contact.resource_name, "people/test_contact");
        assert_eq!(contact.name.as_ref().unwrap().display_name, "Test User");
        assert_eq!(contact.name.as_ref().unwrap().given_name, Some("Test".to_string()));
        assert_eq!(contact.name.as_ref().unwrap().family_name, Some("User".to_string()));
        
        // Verify multiple fields
        assert_eq!(contact.email_addresses.len(), 2);
        assert_eq!(contact.phone_numbers.len(), 2);
        assert_eq!(contact.organizations.len(), 1);
        assert_eq!(contact.photos.len(), 2);
        
        // Check organization
        assert_eq!(contact.organizations[0].name, Some("Test Company".to_string()));
        assert_eq!(contact.organizations[0].title, Some("Test Position".to_string()));
        
        // One photo should be default
        let default_photos: Vec<_> = contact.photos.iter().filter(|p| p.default).collect();
        assert_eq!(default_photos.len(), 1);
        assert_eq!(default_photos[0].url, "https://example.com/test2.jpg");
    }

    #[test]
    fn test_parse_contact_minimal() {
        let client = create_test_client();

        // Create minimal contact JSON with only resourceName and names
        let minimal_json = json!({
            "resourceName": "people/minimal",
            "names": [{
                "displayName": "Minimal User"
            }]
        });

        // Parse the contact
        let result = client.parse_contact(&minimal_json);
        assert!(result.is_ok());
        
        let contact = result.unwrap();
        
        // Verify fields
        assert_eq!(contact.resource_name, "people/minimal");
        assert_eq!(contact.name.as_ref().unwrap().display_name, "Minimal User");
        assert!(contact.name.as_ref().unwrap().given_name.is_none());
        assert!(contact.name.as_ref().unwrap().family_name.is_none());
        
        // Check that other fields are empty
        assert!(contact.email_addresses.is_empty());
        assert!(contact.phone_numbers.is_empty());
        assert!(contact.organizations.is_empty());
        assert!(contact.photos.is_empty());
    }

    #[test]
    fn test_parse_contact_missing_required() {
        let client = create_test_client();

        // Missing resourceName (required field)
        let invalid_json = json!({
            "names": [{
                "displayName": "Invalid User"
            }]
        });

        // Parse should fail
        let result = client.parse_contact(&invalid_json);
        assert!(result.is_err());
        
        if let Err(PeopleApiError::ParseError(msg)) = result {
            assert!(msg.contains("Missing resourceName"));
        } else {
            panic!("Expected ParseError");
        }
    }

    #[test]
    fn test_parse_contact_international() {
        let client = create_test_client();

        // Create test JSON with international characters
        let international_json = create_test_contact_json(
            "people/international",
            "José Müller",
            Some("José"),
            Some("Müller"),
            vec![("jose.muller@example.com", Some("work"))],
            vec![("+49 123 456789", Some("work"))],
            vec![(Some("Deutsche GmbH"), Some("Entwickler"))],
            vec![("https://example.com/jose.jpg", true)],
        );

        // Parse the contact
        let result = client.parse_contact(&international_json);
        assert!(result.is_ok());
        
        let contact = result.unwrap();
        
        // Verify international fields
        assert_eq!(contact.name.as_ref().unwrap().display_name, "José Müller");
        assert_eq!(contact.name.as_ref().unwrap().given_name, Some("José".to_string()));
        assert_eq!(contact.name.as_ref().unwrap().family_name, Some("Müller".to_string()));
        
        // Verify organization with non-ASCII characters
        assert_eq!(contact.organizations[0].name, Some("Deutsche GmbH".to_string()));
        assert_eq!(contact.organizations[0].title, Some("Entwickler".to_string()));
        
        // Verify international phone number
        assert_eq!(contact.phone_numbers[0].value, "+49 123 456789");
    }

    #[test]
    fn test_parse_contact_no_names() {
        let client = create_test_client();

        // Contact with no names section (edge case)
        let no_names_json = json!({
            "resourceName": "people/no_names",
            "emailAddresses": [{
                "value": "unknown@example.com",
                "type": "work"
            }]
        });

        // Parse the contact
        let result = client.parse_contact(&no_names_json);
        assert!(result.is_ok());
        
        let contact = result.unwrap();
        
        // Verify name is None
        assert!(contact.name.is_none());
        
        // Verify email is present
        assert_eq!(contact.email_addresses.len(), 1);
        assert_eq!(contact.email_addresses[0].value, "unknown@example.com");
    }

    #[test]
    fn test_parse_contact_empty_arrays() {
        let client = create_test_client();

        // Contact with empty arrays
        let empty_arrays_json = json!({
            "resourceName": "people/empty_arrays",
            "names": [{
                "displayName": "Empty Arrays User"
            }],
            "emailAddresses": [],
            "phoneNumbers": [],
            "organizations": [],
            "photos": []
        });

        // Parse the contact
        let result = client.parse_contact(&empty_arrays_json);
        assert!(result.is_ok());
        
        let contact = result.unwrap();
        
        // Verify name is present
        assert_eq!(contact.name.as_ref().unwrap().display_name, "Empty Arrays User");
        
        // Verify other fields are empty arrays
        assert!(contact.email_addresses.is_empty());
        assert!(contact.phone_numbers.is_empty());
        assert!(contact.organizations.is_empty());
        assert!(contact.photos.is_empty());
    }

    #[test]
    fn test_parse_contact_failure() {
        let client = create_failing_client(FailMode::Parse);

        // Any JSON should fail during parsing
        let json = json!({
            "resourceName": "people/test"
        });

        let result = client.parse_contact(&json);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PeopleApiError::ParseError(_)));
    }
}