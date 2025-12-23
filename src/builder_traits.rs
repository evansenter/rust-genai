/// Builder traits for shared functionality across Interactions API builders
///
/// # Architecture
///
/// This module provides a trait hierarchy that enables code reuse across builder types:
///
/// - [`InteractionBuilder`]: For the Interactions API (models and agents)
///
/// By implementing [`HasToolsField`], a builder automatically gets the [`WithFunctionCalling`]
/// trait implementation through a blanket impl.
///
/// # Trait Hierarchy
///
/// ```text
/// HasToolsField (marker trait)
///       ↓
/// WithFunctionCalling (blanket impl for all T: HasToolsField)
///       ↓
/// InteractionBuilder (implements HasToolsField)
/// ```
///
/// # Example
///
/// ```no_run
/// use rust_genai::{Client, FunctionDeclaration, WithFunctionCalling};
///
/// let client = Client::new("api-key".to_string(), None);
/// let func = FunctionDeclaration::builder("get_weather")
///     .description("Get the current weather")
///     .build();
///
/// // Builders support function calling via the trait
/// let builder = client
///     .interaction()
///     .with_model("gemini-3-flash-preview")
///     .with_text("What's the weather?")
///     .with_function(func);
/// ```
///
/// [`InteractionBuilder`]: crate::InteractionBuilder
use genai_client::{FunctionDeclaration, Tool};

/// Core trait for builders that support function calling.
///
/// This trait provides methods for adding function declarations to API requests.
/// It is automatically implemented for any type that implements [`HasToolsField`]
/// through a blanket implementation.
///
/// # Methods
///
/// - [`with_function`]: Add a single function declaration
/// - [`with_functions`]: Add multiple function declarations at once
///
/// # Implementation Note
///
/// You should not implement this trait directly. Instead, implement [`HasToolsField`]
/// and you'll get [`WithFunctionCalling`] for free.
///
/// [`with_function`]: WithFunctionCalling::with_function
/// [`with_functions`]: WithFunctionCalling::with_functions
pub trait WithFunctionCalling: Sized {
    /// Add a single function declaration to the request.
    ///
    /// This method can be called multiple times to add several functions.
    /// Each function is converted into a [`Tool`] and added to the request.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::{Client, FunctionDeclaration, WithFunctionCalling};
    /// use serde_json::json;
    ///
    /// let client = Client::new("api-key".to_string(), None);
    ///
    /// let func = FunctionDeclaration::builder("get_temperature")
    ///     .description("Get the temperature for a location")
    ///     .parameter("location", json!({"type": "string"}))
    ///     .required(vec!["location".to_string()])
    ///     .build();
    ///
    /// let builder = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What's the temperature in Paris?")
    ///     .with_function(func);
    /// ```
    fn with_function(self, function: FunctionDeclaration) -> Self;

    /// Add multiple function declarations to the request at once.
    ///
    /// This is a convenience method that calls [`with_function`] for each
    /// function in the provided vector. It's functionally equivalent to calling
    /// [`with_function`] multiple times, but can be more ergonomic when you
    /// have a collection of functions.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_genai::{Client, FunctionDeclaration, WithFunctionCalling};
    ///
    /// let client = Client::new("api-key".to_string(), None);
    ///
    /// let functions = vec![
    ///     FunctionDeclaration::builder("get_weather").build(),
    ///     FunctionDeclaration::builder("get_time").build(),
    /// ];
    ///
    /// let builder = client
    ///     .interaction()
    ///     .with_model("gemini-3-flash-preview")
    ///     .with_text("What's the weather and time?")
    ///     .with_functions(functions);
    /// ```
    ///
    /// [`with_function`]: WithFunctionCalling::with_function
    fn with_functions(self, functions: Vec<FunctionDeclaration>) -> Self {
        functions
            .into_iter()
            .fold(self, |builder, func| builder.with_function(func))
    }

    /// Access to internal tools storage (for implementation).
    ///
    /// This method is used internally by the blanket implementation and should
    /// not be called directly by users of the library.
    #[doc(hidden)]
    fn tools_mut(&mut self) -> &mut Option<Vec<Tool>>;
}

/// Default implementation for any builder with tools field.
///
/// This blanket implementation provides [`WithFunctionCalling`] for any type
/// that implements [`HasToolsField`]. This ensures consistent function calling
/// behavior across all builders.
impl<T> WithFunctionCalling for T
where
    T: HasToolsField,
{
    fn with_function(mut self, function: FunctionDeclaration) -> Self {
        let tool = function.into_tool();
        self.tools_mut().get_or_insert_with(Vec::new).push(tool);
        self
    }

    fn tools_mut(&mut self) -> &mut Option<Vec<Tool>> {
        self.get_tools_mut()
    }
}

/// Marker trait for builders with a tools field.
///
/// This trait acts as a marker to identify builders that support function calling.
/// By implementing this trait, a builder automatically gets the [`WithFunctionCalling`]
/// trait through the blanket implementation.
///
/// # Implementation
///
/// To add function calling support to a builder:
///
/// 1. Add a `tools: Option<Vec<Tool>>` field to your builder struct
/// 2. Implement `HasToolsField` with `get_tools_mut()` returning `&mut self.tools`
/// 3. Your builder now has [`WithFunctionCalling`] automatically
///
/// # Example Implementation
///
/// ```ignore
/// struct MyBuilder {
///     tools: Option<Vec<Tool>>,
///     // ... other fields
/// }
///
/// impl HasToolsField for MyBuilder {
///     fn get_tools_mut(&mut self) -> &mut Option<Vec<Tool>> {
///         &mut self.tools
///     }
/// }
///
/// // Now MyBuilder has with_function() and with_functions() methods
/// ```
pub trait HasToolsField {
    /// Returns a mutable reference to the internal tools storage.
    ///
    /// This method should return a reference to the `Option<Vec<Tool>>` field
    /// in your builder struct.
    fn get_tools_mut(&mut self) -> &mut Option<Vec<Tool>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Mock builder for testing
    struct MockBuilder {
        tools: Option<Vec<Tool>>,
    }

    impl HasToolsField for MockBuilder {
        fn get_tools_mut(&mut self) -> &mut Option<Vec<Tool>> {
            &mut self.tools
        }
    }

    #[test]
    fn test_with_function() {
        let mut builder = MockBuilder { tools: None };

        let func = FunctionDeclaration::builder("test_function")
            .description("Test function")
            .parameter("param1", json!({"type": "string"}))
            .required(vec!["param1".to_string()])
            .build();

        builder = builder.with_function(func);

        assert!(builder.tools.is_some());
        let tools = builder.tools.unwrap();
        assert_eq!(tools.len(), 1);
        assert!(matches!(tools[0], Tool::Function { .. }));
    }

    #[test]
    fn test_with_functions_multiple() {
        let builder = MockBuilder { tools: None };

        let funcs = vec![
            FunctionDeclaration::builder("func1")
                .description("Function 1")
                .build(),
            FunctionDeclaration::builder("func2")
                .description("Function 2")
                .build(),
        ];

        let builder = builder.with_functions(funcs);

        assert!(builder.tools.is_some());
        let tools = builder.tools.unwrap();
        assert_eq!(tools.len(), 2);
    }
}
