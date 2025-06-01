# TODO

This file tracks planned features and improvements for the rust-genai library.

## Features

### Auto-execute Function Calls
- [ ] Add an option to automatically execute function calls returned by the model
- [ ] Design considerations:
  - Should support async functions
  - Need a registry or trait-based approach to map function names to implementations
  - Should return results in a format ready to send back to the model
  - Consider security implications of auto-execution
  - Possibly provide both opt-in automatic execution and manual execution modes
- [ ] Example API:
  ```rust
  // Register functions for auto-execution
  client
      .with_model(model)
      .with_prompt(prompt)
      .with_function(weather_function)
      .with_auto_execute(get_weather) // Pass the actual function
      .generate()
      .await?;
  ```

### Other Planned Features
- [ ] Support for additional Gemini API features (e.g., grounding, safety settings)
- [ ] Support for image/video inputs
- [ ] Better retry logic with exponential backoff
- [ ] Support for batch requests
- [ ] Add more comprehensive examples
- [ ] Improve documentation with more detailed API references

## Technical Improvements
- [ ] Add integration tests that use mock HTTP responses
- [ ] Benchmark performance and optimize hot paths
- [ ] Consider adding a connection pool for better performance
- [ ] Add support for custom HTTP clients

## Macro Improvements
- [ ] Support for async functions in the procedural macro
- [ ] Add validation for function/parameter names (e.g., no spaces, special chars)
- [ ] Support for more complex types (HashMap, custom structs with serde)
- [ ] Generate TypeScript/JavaScript type definitions from Rust functions

## Documentation
- [ ] Add a comprehensive guide for function calling patterns
- [ ] Create a cookbook with common use cases
- [ ] Add troubleshooting guide for common errors
- [ ] Document best practices for prompt engineering with function calls 