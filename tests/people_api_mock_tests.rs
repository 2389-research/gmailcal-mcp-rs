/// People API Mock Tests
///
/// This module tests the People API functionality using mockall
/// to avoid the tokio runtime issues.
///
use mcp_gmailcal::errors::PeopleApiError;
use mcp_gmailcal::people_api::{Contact, ContactList, EmailAddress, Organization, PersonName, PhoneNumber, Photo};
use serde_json::{json, Value};
use mockall::predicate::*;

// Mock the People API client 
mockall::mock! {
    pub PeopleApiClient {
        pub fn list_contacts<'a>(&'a self, max_results: Option<u32>) -> Result<ContactList, PeopleApiError>;
        pub fn search_contacts<'a>(&'a self, query: &'a str, max_results: Option<u32>) -> Result<ContactList, PeopleApiError>;
        pub fn get_contact<'a>(&'a self, resource_name: &'a str) -> Result<Contact, PeopleApiError>;
        pub fn parse_contact<'a>(&'a self, data: &'a Value) -> Result<Contact, PeopleApiError>;
    }
}

// Helper function to create a test contact
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

// Helper function to create a test contact JSON
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_contacts_success() {
        // Create mock client
        let mut mock = MockPeopleApiClient::new();
        
        // Create test contacts
        let contact1 = create_test_contact(
            "people/contact1",
            "John Doe",
            Some("John"),
            Some("Doe"),
            vec![("john.doe@example.com", Some("work"))],
            vec![("123-456-7890", Some("mobile"))],
            vec![(Some("Acme Inc"), Some("Software Developer"))],
            vec![("https://example.com/photo1.jpg", true)],
        );
        
        let contact2 = create_test_contact(
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
        );
        
        let test_contacts = vec![contact1.clone(), contact2.clone()];
        
        // Clone for use in the closure
        let test_contacts_clone = test_contacts.clone();
        
        // Setup expectations
        mock.expect_list_contacts()
            .with(eq(None))
            .returning(move |_| {
                Ok(ContactList {
                    contacts: test_contacts_clone.clone(),
                    next_page_token: None,
                    total_items: Some(test_contacts_clone.len() as u32),
                })
            });
            
        // Test with limited results
        let limited_contacts = vec![contact1.clone()];
        mock.expect_list_contacts()
            .with(eq(Some(1)))
            .returning(move |_| {
                Ok(ContactList {
                    contacts: limited_contacts.clone(),
                    next_page_token: Some("next_page_token".to_string()),
                    total_items: Some(2), // Total available is still 2
                })
            });
        
        // Test the function with no limit
        let result = mock.list_contacts(None);
        
        // Verify result
        assert!(result.is_ok());
        let contacts = result.unwrap();
        assert_eq!(contacts.contacts.len(), 2);
        assert_eq!(contacts.total_items, Some(2));
        assert!(contacts.next_page_token.is_none());
        
        // Verify contact details
        assert_eq!(contacts.contacts[0].resource_name, "people/contact1");
        assert_eq!(contacts.contacts[0].name.as_ref().unwrap().display_name, "John Doe");
        assert_eq!(contacts.contacts[0].email_addresses.len(), 1);
        
        // Test the function with limit
        let result = mock.list_contacts(Some(1));
        
        // Verify result
        assert!(result.is_ok());
        let contacts = result.unwrap();
        assert_eq!(contacts.contacts.len(), 1);
        assert_eq!(contacts.total_items, Some(2)); // Total is still 2
        assert_eq!(contacts.next_page_token, Some("next_page_token".to_string()));
    }

    #[test]
    fn test_list_contacts_failure() {
        // Create mock client
        let mut mock = MockPeopleApiClient::new();
        
        // Setup expectations for auth error
        mock.expect_list_contacts()
            .with(eq(None))
            .returning(|_| Err(PeopleApiError::AuthError("Authentication failed".to_string())));
            
        // Setup expectations for network error with specific params
        mock.expect_list_contacts()
            .with(eq(Some(10)))
            .returning(|_| Err(PeopleApiError::NetworkError("Network error".to_string())));
            
        // Setup expectations for API error
        mock.expect_list_contacts()
            .with(eq(Some(20)))
            .returning(|_| Err(PeopleApiError::ApiError("API error".to_string())));
            
        // Setup expectations for parse error
        mock.expect_list_contacts()
            .with(eq(Some(30)))
            .returning(|_| Err(PeopleApiError::ParseError("Parse error".to_string())));
        
        // Test auth error
        let result = mock.list_contacts(None);
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::AuthError(msg)) => {
                assert_eq!(msg, "Authentication failed");
            }
            _ => panic!("Expected AuthError")
        }
        
        // Test network error
        let result = mock.list_contacts(Some(10));
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::NetworkError(msg)) => {
                assert_eq!(msg, "Network error");
            }
            _ => panic!("Expected NetworkError")
        }
        
        // Test API error
        let result = mock.list_contacts(Some(20));
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::ApiError(msg)) => {
                assert_eq!(msg, "API error");
            }
            _ => panic!("Expected ApiError")
        }
        
        // Test parse error
        let result = mock.list_contacts(Some(30));
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::ParseError(msg)) => {
                assert_eq!(msg, "Parse error");
            }
            _ => panic!("Expected ParseError")
        }
    }
    
    #[test]
    fn test_search_contacts_success() {
        // Create mock client
        let mut mock = MockPeopleApiClient::new();
        
        // Create test contacts for search results
        let contact1 = create_test_contact(
            "people/contact1",
            "John Doe",
            Some("John"),
            Some("Doe"),
            vec![("john.doe@example.com", Some("work"))],
            vec![("123-456-7890", Some("mobile"))],
            vec![(Some("Acme Inc"), Some("Software Developer"))],
            vec![("https://example.com/photo1.jpg", true)],
        );
        
        let test_contacts = vec![contact1.clone()];
        
        // Clone for closures
        let test_contacts_clone1 = test_contacts.clone();
        let test_contacts_clone2 = test_contacts.clone();
        
        // Setup expectations for specific search term
        mock.expect_search_contacts()
            .with(eq("John"), eq(None))
            .returning(move |_, _| {
                Ok(ContactList {
                    contacts: test_contacts_clone1.clone(),
                    next_page_token: None,
                    total_items: Some(1),
                })
            });
            
        // Setup expectations for empty results
        mock.expect_search_contacts()
            .with(eq("NonExistent"), eq(None))
            .returning(|_, _| {
                Ok(ContactList {
                    contacts: vec![],
                    next_page_token: None,
                    total_items: Some(0),
                })
            });
            
        // Setup expectations for search with limit
        mock.expect_search_contacts()
            .with(eq("Test"), eq(Some(5)))
            .returning(move |_, _| {
                Ok(ContactList {
                    contacts: test_contacts_clone2.clone(),
                    next_page_token: None,
                    total_items: Some(1),
                })
            });
        
        // Test search with results
        let result = mock.search_contacts("John", None);
        assert!(result.is_ok());
        let contacts = result.unwrap();
        assert_eq!(contacts.contacts.len(), 1);
        assert_eq!(contacts.contacts[0].name.as_ref().unwrap().display_name, "John Doe");
        
        // Test search with no results
        let result = mock.search_contacts("NonExistent", None);
        assert!(result.is_ok());
        let contacts = result.unwrap();
        assert_eq!(contacts.contacts.len(), 0);
        
        // Test search with limit
        let result = mock.search_contacts("Test", Some(5));
        assert!(result.is_ok());
        let contacts = result.unwrap();
        assert_eq!(contacts.contacts.len(), 1);
    }
    
    #[test]
    fn test_search_contacts_failure() {
        // Create mock client
        let mut mock = MockPeopleApiClient::new();
        
        // Setup expectations for auth error
        mock.expect_search_contacts()
            .with(eq("auth_error"), eq(None))
            .returning(|_, _| Err(PeopleApiError::AuthError("Authentication failed".to_string())));
            
        // Setup expectations for network error
        mock.expect_search_contacts()
            .with(eq("network_error"), eq(None))
            .returning(|_, _| Err(PeopleApiError::NetworkError("Network error".to_string())));
            
        // Setup expectations for API error
        mock.expect_search_contacts()
            .with(eq("api_error"), eq(None))
            .returning(|_, _| Err(PeopleApiError::ApiError("API error".to_string())));
            
        // Setup expectations for parse error
        mock.expect_search_contacts()
            .with(eq("parse_error"), eq(None))
            .returning(|_, _| Err(PeopleApiError::ParseError("Parse error".to_string())));
        
        // Test auth error
        let result = mock.search_contacts("auth_error", None);
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::AuthError(msg)) => {
                assert_eq!(msg, "Authentication failed");
            }
            _ => panic!("Expected AuthError")
        }
        
        // Test network error
        let result = mock.search_contacts("network_error", None);
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::NetworkError(msg)) => {
                assert_eq!(msg, "Network error");
            }
            _ => panic!("Expected NetworkError")
        }
        
        // Test API error
        let result = mock.search_contacts("api_error", None);
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::ApiError(msg)) => {
                assert_eq!(msg, "API error");
            }
            _ => panic!("Expected ApiError")
        }
        
        // Test parse error
        let result = mock.search_contacts("parse_error", None);
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::ParseError(msg)) => {
                assert_eq!(msg, "Parse error");
            }
            _ => panic!("Expected ParseError")
        }
    }
    
    #[test]
    fn test_get_contact_success() {
        // Create mock client
        let mut mock = MockPeopleApiClient::new();
        
        // Create test contact
        let test_contact = create_test_contact(
            "people/contact1",
            "John Doe",
            Some("John"),
            Some("Doe"),
            vec![("john.doe@example.com", Some("work"))],
            vec![("123-456-7890", Some("mobile"))],
            vec![(Some("Acme Inc"), Some("Software Developer"))],
            vec![("https://example.com/photo1.jpg", true)],
        );
        
        // Contact with multiple fields
        let complex_contact = create_test_contact(
            "people/contact2",
            "Jane Smith",
            Some("Jane"),
            Some("Smith"),
            vec![
                ("jane.smith@example.com", Some("work")),
                ("jsmith@personal.com", Some("home")),
            ],
            vec![
                ("987-654-3210", Some("mobile")),
                ("555-123-4567", Some("work")),
            ],
            vec![
                (Some("XYZ Corp"), Some("Product Manager")),
                (Some("ABC Inc"), Some("Consultant")),
            ],
            vec![
                ("https://example.com/photo2a.jpg", false),
                ("https://example.com/photo2b.jpg", true),
            ],
        );
        
        // Setup expectations
        mock.expect_get_contact()
            .with(eq("people/contact1"))
            .returning(move |_| Ok(test_contact.clone()));
            
        mock.expect_get_contact()
            .with(eq("people/contact2"))
            .returning(move |_| Ok(complex_contact.clone()));
            
        mock.expect_get_contact()
            .with(eq("people/nonexistent"))
            .returning(|resource_name| {
                Err(PeopleApiError::ApiError(format!(
                    "Contact not found: {}",
                    resource_name
                )))
            });
        
        // Test getting a simple contact
        let result = mock.get_contact("people/contact1");
        assert!(result.is_ok());
        let contact = result.unwrap();
        
        // Verify details
        assert_eq!(contact.resource_name, "people/contact1");
        assert_eq!(contact.name.as_ref().unwrap().display_name, "John Doe");
        assert_eq!(contact.email_addresses.len(), 1);
        assert_eq!(contact.email_addresses[0].value, "john.doe@example.com");
        assert_eq!(contact.phone_numbers.len(), 1);
        assert_eq!(contact.organizations.len(), 1);
        assert_eq!(contact.photos.len(), 1);
        
        // Test getting a complex contact
        let result = mock.get_contact("people/contact2");
        assert!(result.is_ok());
        let contact = result.unwrap();
        
        // Verify multiple fields
        assert_eq!(contact.email_addresses.len(), 2);
        assert_eq!(contact.phone_numbers.len(), 2);
        assert_eq!(contact.organizations.len(), 2);
        assert_eq!(contact.photos.len(), 2);
        
        // Test getting a non-existent contact
        let result = mock.get_contact("people/nonexistent");
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::ApiError(msg)) => {
                assert!(msg.contains("Contact not found"));
            }
            _ => panic!("Expected ApiError")
        }
    }
    
    #[test]
    fn test_get_contact_failure() {
        // Create mock client
        let mut mock = MockPeopleApiClient::new();
        
        // Setup expectations for auth error
        mock.expect_get_contact()
            .with(eq("people/auth_error"))
            .returning(|_| Err(PeopleApiError::AuthError("Authentication failed".to_string())));
            
        // Setup expectations for network error
        mock.expect_get_contact()
            .with(eq("people/network_error"))
            .returning(|_| Err(PeopleApiError::NetworkError("Network error".to_string())));
            
        // Setup expectations for API error
        mock.expect_get_contact()
            .with(eq("people/api_error"))
            .returning(|_| Err(PeopleApiError::ApiError("API error".to_string())));
            
        // Setup expectations for parse error
        mock.expect_get_contact()
            .with(eq("people/parse_error"))
            .returning(|_| Err(PeopleApiError::ParseError("Parse error".to_string())));
        
        // Test auth error
        let result = mock.get_contact("people/auth_error");
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::AuthError(msg)) => {
                assert_eq!(msg, "Authentication failed");
            }
            _ => panic!("Expected AuthError")
        }
        
        // Test network error
        let result = mock.get_contact("people/network_error");
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::NetworkError(msg)) => {
                assert_eq!(msg, "Network error");
            }
            _ => panic!("Expected NetworkError")
        }
        
        // Test API error
        let result = mock.get_contact("people/api_error");
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::ApiError(msg)) => {
                assert_eq!(msg, "API error");
            }
            _ => panic!("Expected ApiError")
        }
        
        // Test parse error
        let result = mock.get_contact("people/parse_error");
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::ParseError(msg)) => {
                assert_eq!(msg, "Parse error");
            }
            _ => panic!("Expected ParseError")
        }
    }
    
    #[test]
    fn test_parse_contact_success() {
        // Create mock client
        let mut mock = MockPeopleApiClient::new();
        
        // Setup expectations for parsing valid contacts
        mock.expect_parse_contact()
            .returning(|data| {
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
            });
        
        // Create test JSON for a complete contact
        let contact_json = create_test_contact_json(
            "people/test",
            "Test User",
            Some("Test"),
            Some("User"),
            vec![("test@example.com", Some("work"))],
            vec![("555-123-4567", Some("mobile"))],
            vec![(Some("Test Company"), Some("Test Position"))],
            vec![("https://example.com/test.jpg", true)],
        );
        
        // Test parsing
        let result = mock.parse_contact(&contact_json);
        assert!(result.is_ok());
        
        let contact = result.unwrap();
        assert_eq!(contact.resource_name, "people/test");
        assert_eq!(contact.name.as_ref().unwrap().display_name, "Test User");
        assert_eq!(contact.email_addresses.len(), 1);
        assert_eq!(contact.phone_numbers.len(), 1);
        assert_eq!(contact.organizations.len(), 1);
        assert_eq!(contact.photos.len(), 1);
        
        // Test minimal JSON
        let minimal_json = json!({
            "resourceName": "people/minimal"
        });
        
        let result = mock.parse_contact(&minimal_json);
        assert!(result.is_ok());
        
        let contact = result.unwrap();
        assert_eq!(contact.resource_name, "people/minimal");
        assert!(contact.name.is_none());
        assert!(contact.email_addresses.is_empty());
        assert!(contact.phone_numbers.is_empty());
        assert!(contact.organizations.is_empty());
        assert!(contact.photos.is_empty());
        
        // Test contact with international characters
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
        
        let result = mock.parse_contact(&international_json);
        assert!(result.is_ok());
        
        let contact = result.unwrap();
        assert_eq!(contact.name.as_ref().unwrap().display_name, "José Müller");
        assert_eq!(contact.name.as_ref().unwrap().given_name, Some("José".to_string()));
        assert_eq!(contact.organizations[0].name, Some("Deutsche GmbH".to_string()));
    }
    
    #[test]
    fn test_parse_contact_failure() {
        // Create mock client
        let mut mock = MockPeopleApiClient::new();
        
        // Setup expectations for failure when resourceName is missing
        mock.expect_parse_contact()
            .with(function(|json: &Value| json.get("resourceName").is_none()))
            .returning(|_| {
                Err(PeopleApiError::ParseError("Missing resourceName".to_string()))
            });
            
        // Setup expectations for other parse errors
        mock.expect_parse_contact()
            .with(function(|json: &Value| {
                json.get("resourceName").is_some() && 
                json.get("resourceName").unwrap().as_str().unwrap() == "people/error"
            }))
            .returning(|_| {
                Err(PeopleApiError::ParseError("Error parsing contact".to_string()))
            });
        
        // Test missing resourceName
        let invalid_json = json!({
            "names": [{
                "displayName": "Invalid Contact"
            }]
        });
        
        let result = mock.parse_contact(&invalid_json);
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::ParseError(msg)) => {
                assert_eq!(msg, "Missing resourceName");
            }
            _ => panic!("Expected ParseError")
        }
        
        // Test other parse error
        let error_json = json!({
            "resourceName": "people/error",
            "names": [{
                "displayName": "Error Contact"
            }]
        });
        
        let result = mock.parse_contact(&error_json);
        assert!(result.is_err());
        match result {
            Err(PeopleApiError::ParseError(msg)) => {
                assert_eq!(msg, "Error parsing contact");
            }
            _ => panic!("Expected ParseError")
        }
    }
    
    #[test]
    fn test_contact_type_operations() {
        // Test contact data structure operations
        
        // Create a contact
        let contact = create_test_contact(
            "people/test",
            "Test User",
            Some("Test"),
            Some("User"),
            vec![("test@example.com", Some("work"))],
            vec![("555-123-4567", Some("mobile"))],
            vec![(Some("Test Company"), Some("Test Position"))],
            vec![("https://example.com/test.jpg", true)],
        );
        
        // Test cloning
        let cloned_contact = contact.clone();
        assert_eq!(contact.resource_name, cloned_contact.resource_name);
        assert_eq!(
            contact.name.as_ref().unwrap().display_name,
            cloned_contact.name.as_ref().unwrap().display_name
        );
        
        // Create a contact list
        let contact_list = ContactList {
            contacts: vec![contact.clone()],
            next_page_token: Some("next_page".to_string()),
            total_items: Some(1),
        };
        
        // Test cloning of contact list
        let cloned_list = contact_list.clone();
        assert_eq!(contact_list.contacts.len(), cloned_list.contacts.len());
        assert_eq!(contact_list.next_page_token, cloned_list.next_page_token);
        assert_eq!(contact_list.total_items, cloned_list.total_items);
        
        // Test debug formatting
        let debug_output = format!("{:?}", contact);
        assert!(debug_output.contains("resource_name"));
        assert!(debug_output.contains("Test User"));
        
        let debug_output = format!("{:?}", contact_list);
        assert!(debug_output.contains("contacts"));
        assert!(debug_output.contains("next_page_token"));
    }
}