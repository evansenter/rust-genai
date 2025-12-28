# Code Assistant Example

A code analysis and documentation generation tool demonstrating structured output for code tasks.

## Overview

This example demonstrates a code assistant that:

1. **Code Analysis**: Analyzes structure, complexity, and identifies issues
2. **Documentation Generation**: Creates language-appropriate doc comments
3. **Code Explanation**: Explains code behavior in plain language
4. **Refactoring Suggestions**: Provides actionable improvement recommendations

## Features

### Structured Analysis Output

Uses JSON schema to ensure consistent, parseable results:

```rust
#[derive(Serialize, Deserialize)]
struct CodeAnalysis {
    summary: String,
    complexity: ComplexityInfo,
    functions: Vec<FunctionInfo>,
    suggestions: Vec<String>,
    potential_issues: Vec<Issue>,
}
```

### Multi-Language Support

Adapts documentation style to the target language:

| Language | Doc Style |
|----------|-----------|
| Rust | `///` doc comments with examples |
| Python | Docstrings with Args, Returns, Raises |
| JavaScript | JSDoc with @param, @returns |

### Specialized System Prompts

Each task uses tailored system instructions:

```rust
.with_system_instructions(vec![
    "You are an expert code analyst. Analyze code thoroughly..."
])
```

## Running

```bash
export GEMINI_API_KEY=your_api_key
cargo run --example code_assistant
```

## Sample Output

```
📊 CODE ANALYSIS

Summary: Two utility functions for processing string collections
and calculating numeric statistics.

Complexity: medium (score: 5/10)
  Manual iteration instead of iterators, potential panic in unwrap()

Functions found:
  • process_items -> Vec<String>
    Purpose: Transforms strings to uppercase and removes duplicates
  • calculate_stats -> (f64, i32, i32)
    Purpose: Computes average, maximum, and minimum values

Potential Issues:
  [WARNING] Possible panic if numbers vector is empty
    Fix: Use unwrap_or_default() or return Option
```

## API Usage

### Initialize the Assistant

```rust
let client = Client::builder(api_key).build();
let assistant = CodeAssistant::new(client);
```

### Analyze Code

```rust
let analysis = assistant.analyze_code(code, "rust").await?;
println!("Complexity: {} ({}/10)",
    analysis.complexity.level,
    analysis.complexity.score);
```

### Generate Documentation

```rust
let docs = assistant.generate_documentation(code, "rust").await?;
for func_doc in docs.function_docs {
    println!("{}\n{}", func_doc.function_name, func_doc.doc_comment);
}
```

### Explain Code

```rust
let explanation = assistant.explain_code(code, "rust").await?;
```

## Production Enhancements

### IDE Integration

```rust
// LSP-compatible response format
struct DiagnosticResult {
    range: Range,
    severity: DiagnosticSeverity,
    message: String,
    code: Option<String>,
}
```

### Batch Processing

```rust
async fn analyze_codebase(dir: &Path) -> Vec<FileAnalysis> {
    // Process multiple files in parallel
    let tasks: Vec<_> = files.iter()
        .map(|f| assistant.analyze_code(&f.content, &f.lang))
        .collect();
    futures::future::join_all(tasks).await
}
```

### Caching Layer

```rust
struct CachedAssistant {
    assistant: CodeAssistant,
    cache: HashMap<Hash, CodeAnalysis>,
}
```

### Custom Rules Integration

```rust
// Combine with language-specific linters
let gemini_issues = assistant.analyze_code(code, "rust").await?;
let clippy_issues = run_clippy(code)?;
merge_issues(gemini_issues, clippy_issues)
```

## Complexity Scoring

The assistant evaluates code complexity based on:

- Cyclomatic complexity (branches, loops)
- Cognitive complexity (nesting depth)
- Code duplication
- Error handling patterns
- Function length

Scores range from 1-10:
- 1-3: Low complexity, easy to maintain
- 4-6: Medium complexity, some attention needed
- 7-10: High complexity, consider refactoring

## Best Practices

1. **Chunk Large Files**: Break large files into functions before analysis
2. **Verify Suggestions**: Always review AI suggestions before applying
3. **Iterate**: Use refactoring suggestions iteratively
4. **Context Matters**: Provide surrounding code for better analysis
5. **Language Hints**: Always specify the language for accurate results
