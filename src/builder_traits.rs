/// Builder traits for shared functionality across GenerateContent and Interactions APIs
use genai_client::{FunctionDeclaration, GenerationConfig, Tool};

/// Core trait for builders that support function calling
pub trait WithFunctionCalling: Sized {
    /// Add a single function declaration
    fn with_function(self, function: FunctionDeclaration) -> Self;

    /// Add multiple function declarations
    fn with_functions(self, functions: Vec<FunctionDeclaration>) -> Self {
        functions
            .into_iter()
            .fold(self, |builder, func| builder.with_function(func))
    }

    /// Access to internal tools storage (for implementation)
    fn tools_mut(&mut self) -> &mut Option<Vec<Tool>>;
}

/// Default implementation for any builder with tools field
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

/// Marker trait for builders with tools field
pub trait HasToolsField {
    fn get_tools_mut(&mut self) -> &mut Option<Vec<Tool>>;
}

/// Trait for builders that support system instructions
pub trait WithSystemInstruction: Sized {
    fn with_system_instruction(self, instruction: impl Into<String>) -> Self;
}

/// Trait for builders that support generation config
pub trait WithGenerationConfig: Sized {
    fn with_generation_config(self, config: GenerationConfig) -> Self;
}

/// Trait for builders that support code execution
pub trait WithCodeExecution: Sized {
    fn with_code_execution(self) -> Self;
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
        assert!(tools[0].function_declarations.is_some());
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
