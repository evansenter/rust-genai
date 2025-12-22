use rust_genai::CallableFunction;
use rust_genai_macros::generate_function_declaration;
use serde_json::json;

#[tokio::test]
async fn test_string_return_wrapped() {
    /// Returns a simple string
    #[generate_function_declaration]
    fn get_greeting(name: String) -> String {
        format!("Hello, {}!", name)
    }

    let callable = GetGreetingCallable;
    let args = json!({ "name": "Alice" });
    let result = callable.call(args).await.unwrap();

    // String return should be wrapped in { "result": ... }
    assert!(result.is_object());
    assert_eq!(result, json!({ "result": "Hello, Alice!" }));
}

#[tokio::test]
async fn test_number_return_wrapped() {
    /// Adds two numbers
    #[generate_function_declaration]
    fn add_numbers(a: i32, b: i32) -> i32 {
        a + b
    }

    let callable = AddNumbersCallable;
    let args = json!({ "a": 5, "b": 3 });
    let result = callable.call(args).await.unwrap();

    // Number return should be wrapped in { "result": ... }
    assert!(result.is_object());
    assert_eq!(result, json!({ "result": 8 }));
}

#[tokio::test]
async fn test_bool_return_wrapped() {
    /// Checks if a number is even
    #[generate_function_declaration]
    fn is_even(number: i32) -> bool {
        number % 2 == 0
    }

    let callable = IsEvenCallable;
    let args = json!({ "number": 4 });
    let result = callable.call(args).await.unwrap();

    // Boolean return should be wrapped in { "result": ... }
    assert!(result.is_object());
    assert_eq!(result, json!({ "result": true }));
}

#[tokio::test]
async fn test_array_return_wrapped() {
    /// Returns a list of items
    #[generate_function_declaration]
    fn get_items(count: usize) -> Vec<String> {
        (0..count).map(|i| format!("item{}", i)).collect()
    }

    let callable = GetItemsCallable;
    let args = json!({ "count": 3 });
    let result = callable.call(args).await.unwrap();

    // Array return should be wrapped in { "result": ... }
    assert!(result.is_object());
    assert_eq!(result, json!({ "result": ["item0", "item1", "item2"] }));
}

#[tokio::test]
async fn test_object_return_not_wrapped() {
    /// Returns weather data as an object
    #[generate_function_declaration]
    fn get_weather_data(city: String) -> serde_json::Value {
        json!({
            "city": city,
            "temperature": 72,
            "condition": "sunny"
        })
    }

    let callable = GetWeatherDataCallable;
    let args = json!({ "city": "San Francisco" });
    let result = callable.call(args).await.unwrap();

    // Object return should NOT be wrapped
    assert!(result.is_object());
    assert_eq!(
        result,
        json!({
            "city": "San Francisco",
            "temperature": 72,
            "condition": "sunny"
        })
    );
}

#[tokio::test]
async fn test_option_some_wrapped() {
    /// Finds a user by ID
    #[generate_function_declaration]
    fn find_user(id: i32) -> Option<String> {
        if id > 0 {
            Some(format!("User{}", id))
        } else {
            None
        }
    }

    let callable = FindUserCallable;

    // Test Some case
    let args = json!({ "id": 1 });
    let result = callable.call(args).await.unwrap();
    assert_eq!(result, json!({ "result": "User1" }));

    // Test None case
    let args = json!({ "id": 0 });
    let result = callable.call(args).await.unwrap();
    assert_eq!(result, json!({ "result": null }));
}

#[tokio::test]
async fn test_unit_return_wrapped() {
    /// Performs an action with no return value
    #[generate_function_declaration]
    fn perform_action(action: String) {
        // Just for testing - normally would do something
        let _ = action;
    }

    let callable = PerformActionCallable;
    let args = json!({ "action": "test" });
    let result = callable.call(args).await.unwrap();

    // Unit return should be wrapped as null
    assert!(result.is_object());
    assert_eq!(result, json!({ "result": null }));
}

#[tokio::test]
async fn test_float_return_wrapped() {
    /// Calculates average
    #[generate_function_declaration]
    fn calculate_average(values: Vec<f64>) -> f64 {
        if values.is_empty() {
            0.0
        } else {
            values.iter().sum::<f64>() / values.len() as f64
        }
    }

    let callable = CalculateAverageCallable;
    let args = json!({ "values": [1.0, 2.0, 3.0, 4.0] });
    let result = callable.call(args).await.unwrap();

    // Float return should be wrapped in { "result": ... }
    assert!(result.is_object());
    assert_eq!(result, json!({ "result": 2.5 }));
}

#[tokio::test]
async fn test_custom_struct_wrapped() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct User {
        name: String,
        age: u32,
    }

    /// Creates a user - returns a custom struct
    #[generate_function_declaration]
    fn create_user(name: String, age: u32) -> User {
        User { name, age }
    }

    let callable = CreateUserCallable;
    let args = json!({ "name": "Bob", "age": 30 });
    let result = callable.call(args).await.unwrap();

    // Custom struct serializes to object, so should NOT be wrapped
    assert!(result.is_object());
    assert_eq!(
        result,
        json!({
            "name": "Bob",
            "age": 30
        })
    );
}
