use mcp_attr::Result as McpResult;
use mcp_gmailcal::server::GmailServer;
use serde_json::json;

// Test the send_custom_event API with simple payload
#[tokio::test]
async fn test_send_custom_event_api() -> McpResult<()> {
    // Initialize server
    let server = GmailServer::new();

    // Call the MCP API to send a custom event
    let response = server
        .send_custom_event("test-api".to_string(), json!({"message": "test from API"}))
        .await?;

    // Verify response format
    let parsed: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(parsed["status"], "success");
    assert_eq!(parsed["event"]["type"], "test-api");

    Ok(())
}

// Test sending custom events with complex data structures
#[tokio::test]
async fn test_complex_event_types() -> McpResult<()> {
    // Initialize server
    let server = GmailServer::new();
    
    // Test 1: Send a custom event with nested objects
    let nested_data = json!({
        "user": {
            "name": "Test User",
            "contact": {
                "email": "test@example.com",
                "phone": {
                    "home": "123-456-7890",
                    "work": "987-654-3210"
                }
            },
            "preferences": {
                "theme": "dark",
                "notifications": true
            }
        },
        "metadata": {
            "version": "1.0.0",
            "timestamp": 1651234567
        }
    });
    
    let response = server
        .send_custom_event("complex-object".to_string(), nested_data.clone())
        .await?;
    
    // Verify the response format
    let parsed: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(parsed["status"], "success");
    assert_eq!(parsed["event"]["type"], "complex-object");
    assert_eq!(parsed["event"]["data"], nested_data);
    
    // Test 2: Send a custom event with arrays
    let array_data = json!({
        "items": [
            {"id": 1, "name": "Item 1", "tags": ["important", "urgent"]},
            {"id": 2, "name": "Item 2", "tags": ["normal"]},
            {"id": 3, "name": "Item 3", "tags": []}
        ],
        "counts": [5, 10, 15, 20],
        "mixed": ["string", 42, true, null, {"key": "value"}]
    });
    
    let response = server
        .send_custom_event("array-data".to_string(), array_data.clone())
        .await?;
    
    // Verify the response format
    let parsed: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(parsed["status"], "success");
    assert_eq!(parsed["event"]["type"], "array-data");
    assert_eq!(parsed["event"]["data"], array_data);
    
    // Test 3: Test event data with special characters, emojis, and unicode
    let special_data = json!({
        "title": "Special Characters & Unicode Test ✓",
        "description": "Contains emoji 🚀 and special symbols © Ω ∞ € £ ¥",
        "language": {
            "chinese": "你好，世界",
            "arabic": "مرحبا بالعالم",
            "russian": "Привет, мир",
            "japanese": "こんにちは世界"
        },
        "symbols": "&lt;&gt;&amp;\"\'\\/\\\\",
        "multiline": "This is a\nmultiline\ntext with\nnewlines"
    });
    
    let response = server
        .send_custom_event("special-chars".to_string(), special_data.clone())
        .await?;
    
    // Verify the response format
    let parsed: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(parsed["status"], "success");
    assert_eq!(parsed["event"]["type"], "special-chars");
    assert_eq!(parsed["event"]["data"], special_data);
    
    // Verify special character handling
    assert_eq!(parsed["event"]["data"]["title"], "Special Characters & Unicode Test ✓");
    assert_eq!(parsed["event"]["data"]["language"]["chinese"], "你好，世界");
    assert!(parsed["event"]["data"]["multiline"].as_str().unwrap().contains('\n'));
    
    Ok(())
}

// Test sending multiple event types in sequence
#[tokio::test]
async fn test_multiple_event_types() -> McpResult<()> {
    // Initialize server
    let server = GmailServer::new();
    
    // Create test data for different event types
    let event_types = vec![
        ("numeric-data", json!({
            "integer": 42,
            "float": 3.14159,
            "negative": -273.15,
            "scientific": 6.022e23,
            "binary": 0b1010,
            "hex": 0xFF,
            "large": 9223372036854775807i64
        })),
        ("boolean-data", json!({
            "isTrue": true,
            "isFalse": false,
            "mixed": [true, false, true],
            "nested": {"active": true, "verified": false}
        })),
        ("null-data", json!({
            "nullValue": null,
            "mixedArray": [1, null, "string", null],
            "objectWithNull": {"id": 1, "parent": null, "description": "Test"}
        })),
        ("deep-nesting", json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": {
                            "level5": {
                                "data": "Deep nested value",
                                "array": [1, 2, [3, 4, [5, 6]]]
                            }
                        }
                    }
                }
            }
        })),
        ("empty-structures", json!({
            "emptyObject": {},
            "emptyArray": [],
            "objectWithEmpty": {"name": "test", "children": []},
            "arrayWithEmpty": [1, {}, [], "test"]
        }))
    ];
    
    // Send each event type and verify response
    for (event_type, event_data) in event_types {
        let response = server
            .send_custom_event(event_type.to_string(), event_data.clone())
            .await?;
        
        // Verify response format
        let parsed: serde_json::Value = serde_json::from_str(&response)?;
        assert_eq!(parsed["status"], "success");
        assert_eq!(parsed["event"]["type"], event_type);
        assert_eq!(parsed["event"]["data"], event_data);
    }
    
    // Test very large JSON data
    let large_array_size = 100;
    let mut large_array = Vec::with_capacity(large_array_size);
    for i in 0..large_array_size {
        large_array.push(json!({
            "id": i,
            "name": format!("Item {}", i),
            "data": {
                "value": i * 10,
                "squared": i * i,
                "hex": format!("0x{:X}", i)
            }
        }));
    }
    
    let large_data = json!({
        "count": large_array_size,
        "items": large_array
    });
    
    let response = server
        .send_custom_event("large-data".to_string(), large_data.clone())
        .await?;
    
    // Verify the response format
    let parsed: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(parsed["status"], "success");
    assert_eq!(parsed["event"]["type"], "large-data");
    
    // Verify the large data properties
    assert_eq!(parsed["event"]["data"]["count"], large_array_size);
    assert_eq!(parsed["event"]["data"]["items"].as_array().unwrap().len(), large_array_size);
    
    // Check a few random elements
    assert_eq!(parsed["event"]["data"]["items"][0]["id"], 0);
    assert_eq!(parsed["event"]["data"]["items"][50]["id"], 50);
    assert_eq!(parsed["event"]["data"]["items"][99]["id"], 99);
    
    Ok(())
}