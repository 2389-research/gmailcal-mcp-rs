/// Example of using parameterized test helpers
///
/// This file demonstrates how to use the parameterized test helpers
/// provided in helper.rs to create concise and maintainable tests
/// that vary input parameters.
///
mod helper;
use helper::{TestCase, run_test_cases};
use serde_json::json;

#[cfg(test)]
mod parameterized_tests {
    use super::*;
    
    // Function that we want to test with multiple inputs
    fn encode_string_to_base64(input: &str) -> String {
        base64::encode(input)
    }
    
    // Convert the return type to String to match our TestCase structure
    fn classify_number(n: i32) -> String {
        let result = if n < 0 {
            "negative"
        } else if n == 0 {
            "zero"
        } else if n % 2 == 0 {
            "even"
        } else {
            "odd"
        };
        result.to_string()
    }
    
    // Simple example of parameterized testing using the TestCase struct
    #[test]
    fn test_number_classification() {
        let test_cases = [
            TestCase::new("negative", -5, "negative".to_string()),
            TestCase::new("zero", 0, "zero".to_string()),
            TestCase::new("odd", 7, "odd".to_string()),
            TestCase::new("even", 10, "even".to_string()),
        ];
        
        run_test_cases(&test_cases, |input| classify_number(input));
    }
    
    // Example with string inputs and expected outputs
    #[test]
    fn test_base64_encoding() {
        let test_cases = [
            TestCase::new("empty_string", "", "".to_string()),
            TestCase::new("hello_world", "Hello, World!", "SGVsbG8sIFdvcmxkIQ==".to_string()),
            TestCase::new("special_chars", "!@#$%^&*()", "IUAjJCVeJiooKQ==".to_string()),
            TestCase::new("utf8_chars", "こんにちは", "44GT44KT44Gr44Gh44Gv".to_string()),
        ];
        
        run_test_cases(&test_cases, |input| encode_string_to_base64(input));
    }
    
    // Import the macro
    use parameterized_test;
    
    // Function that will be parameterized
    fn test_json_parsing(case_name: &str) {
        let input = match case_name {
            "empty_object" => "{}",
            "simple_object" => r#"{"name":"test","value":123}"#,
            "nested_object" => r#"{"outer":{"inner":"value"}}"#,
            "array" => r#"[1,2,3,4]"#,
            _ => panic!("Unknown test case: {}", case_name),
        };
        
        let parsed: serde_json::Value = serde_json::from_str(input).expect("Failed to parse JSON");
        
        match case_name {
            "empty_object" => assert_eq!(parsed, json!({})),
            "simple_object" => assert_eq!(parsed["name"], "test"),
            "nested_object" => assert_eq!(parsed["outer"]["inner"], "value"),
            "array" => assert_eq!(parsed[2], 3),
            _ => panic!("Unknown test case: {}", case_name),
        }
    }
    
    // Use the macro to create parameterized tests
    parameterized_test!(
        test_json_parsing,
        empty_object,
        simple_object,
        nested_object,
        array
    );
}