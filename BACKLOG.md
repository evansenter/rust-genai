# Project Backlog

This document tracks future improvements, refactoring opportunities, and feature ideas for rust-genai.

**Last Updated:** 2025-12-23

## Summary of Changes (2025-12-23)

### Research Findings (Anthropic, Google, OpenAI Blogs)

Conducted comprehensive research on 2025 agentic AI trends and Gemini API updates:

**✅ Validated High Priority Items:**
- **MCP Support** - Confirmed as industry standard, donated to Linux Foundation, 97M+ monthly SDK downloads
- **ReAct Pattern** - Validated as dominant agentic pattern across all major providers

**✅ Fixed Critical Bugs:**
- **Thought Signatures** - Implemented full support for Gemini 3 thought signatures in both GenerateContent and Interactions APIs

**🎯 New High Priority Items:**
- **Grounding with Google Search** - Gemini-specific real-time web grounding, unique differentiator
- **thinking_level Parameter** - Gemini 3 parameter for controlling reasoning depth (already partially supported for Interactions API, need GenerateContent API support)

**📋 New Features Identified:**
- Deep Research Agent support (via Interactions API)
- Multi-tool use (combine search + code execution)
- Remote MCP tool support in Gemini

**Current Status:**
- 0 CRITICAL items ✓
- 6 High Priority items (2 strategic standards, 2 Gemini-specific features, 2 production)
- 9 Medium Priority items (features, quality, documentation)
- 164 tests passing (138 regular, 26 ignored/require API key)
- ~9,100 lines of Rust code

---

## Market Context (2025)

The agentic AI landscape is rapidly evolving:

- **Market Growth**: The AI agent market reached $3.7B in 2023 and is expected to double by end of 2025, with ~85% of businesses adopting agents ([sources](https://www.shakudo.io/blog/top-9-ai-agent-frameworks))
- **Industry Shift**: Moving from experimental prototypes to production-ready infrastructure for autonomous, multimodal systems ([IBM](https://www.ibm.com/think/insights/ai-agents-2025-expectations-vs-reality))
- **Open Standards**: Model Context Protocol (MCP) emerging as the dominant standard, with support from Anthropic, OpenAI, Microsoft, Google, AWS, Bloomberg ([Anthropic](https://www.anthropic.com/news/donating-the-model-context-protocol-and-establishing-of-the-agentic-ai-foundation))
- **Technology Stack**: Python + LangChain + OpenAI dominate (73.6%), but Rust adoption growing due to performance/scalability needs ([Red Hat](https://developers.redhat.com/articles/2025/09/15/why-some-agentic-ai-developers-are-moving-code-python-rust))
- **Pattern Convergence**: ReAct (Reasoning + Acting) and multi-agent orchestration becoming standard patterns ([Google Cloud](https://cloud.google.com/architecture/choose-design-pattern-agentic-ai-system))

**Opportunity**: Rust-genai is well-positioned to become the leading Rust library for production agentic AI, especially as systems scale beyond Python's GIL limitations.

## 🚨 CRITICAL (Breaking Compatibility)

_No critical items at this time._

---

## High Priority

### Model Context Protocol (MCP) Support 🎯
**Impact:** Very High | **Effort:** ~2-3 weeks | **Type:** Feature

Implement support for [Model Context Protocol](https://modelcontextprotocol.io/specification/2025-11-25), the open standard for LLM-tool integration. MCP has emerged as the industry standard with adoption from OpenAI, Microsoft, Google, AWS, and Bloomberg (donated by Anthropic to the [Agentic AI Foundation](https://www.anthropic.com/news/donating-the-model-context-protocol-and-establishing-of-the-agentic-ai-foundation)).

**Why High Priority:**
- Industry-wide adoption momentum in 2025
- Makes rust-genai interoperable with entire MCP ecosystem
- Users can leverage any MCP server (databases, APIs, filesystems) transparently
- Differentiates us from competing Rust libraries

**What It Enables:**
```rust
// Connect to MCP servers
let mcp_client = client.mcp()
    .add_server("filesystem", "npx @modelcontextprotocol/server-filesystem")
    .add_server("github", "mcp-server-github")
    .build();

// Use MCP tools in interactions
let response = client.interaction()
    .with_model("gemini-3-flash-preview")
    .with_mcp_tools(&mcp_client)
    .with_input("List files in /tmp and create a GitHub issue")
    .create()
    .await?;
```

**Technical Requirements:**
- JSON-RPC 2.0 client implementation
- stdio and HTTP+SSE transport support
- Tool discovery and schema validation
- Asynchronous operation handling (per 2025-11-25 spec)
- Server identity management

**References:**
- [MCP Specification](https://modelcontextprotocol.io/specification/2025-11-25)
- [MCP GitHub](https://github.com/modelcontextprotocol)
- [Building AI Agents with MCP in Rust](https://composio.dev/blog/how-to-build-your-first-ai-agent-with-mcp-in-rust)

---

### ReAct Pattern Implementation 🎯
**Impact:** High | **Effort:** ~1-2 weeks | **Type:** Feature

Implement the [ReAct (Reasoning + Acting) pattern](https://www.dailydoseofds.com/ai-agents-crash-course-part-10-with-implementation/), the dominant agentic AI pattern in 2025. This enables agents to alternate between reasoning and taking actions based on observations.

**Why High Priority:**
- Fundamental pattern for agentic AI (becoming table stakes)
- Relatively straightforward implementation (~350 lines)
- Unlocks practical agent use cases
- Strong demand in the market

**Proposed API:**
```rust
let agent = client.react_agent()
    .with_model("gemini-3-flash-preview")
    .with_tools(vec![weather_tool, calculator_tool])
    .with_max_iterations(10)
    .build();

// Agent will loop: think → act → observe → think → act...
let result = agent.run("What's the weather in Tokyo and what's 25°C in Fahrenheit?").await?;

// Access the reasoning trace
for step in result.trace {
    match step {
        ReActStep::Thought(text) => println!("💭 {text}"),
        ReActStep::Action(call) => println!("🔧 {call.name}({call.args:?})"),
        ReActStep::Observation(output) => println!("👁 {output}"),
    }
}
```

**Components:**
- Thought/Action/Observation loop (~150 lines)
- Prompt engineering for ReAct format (~50 lines)
- Trace/history management (~100 lines)
- Exit condition detection (~50 lines)

**References:**
- [Google Cloud: Choose a design pattern for agentic AI](https://cloud.google.com/architecture/choose-design-pattern-agentic-ai-system)
- [ReAct Pattern Implementation Guide](https://www.dailydoseofds.com/ai-agents-crash-course-part-10-with-implementation/)

---

### Enhanced Error Context
**Impact:** Medium-High | **Effort:** ~3-4 hours | **Type:** Enhancement

Improve error messages to include structured context for better debugging and observability.

**Current State:**
```rust
return Err(GenaiError::Api(error_text));
```

**Proposed:**
```rust
#[derive(Debug, Error)]
pub enum GenaiError {
    #[error("API error (status: {status_code}): {message}")]
    Api {
        status_code: u16,
        message: String,
        request_id: Option<String>,
    },
    // ... other variants
}
```

**Benefits:**
- Better debugging in production
- Request ID tracking for support cases
- HTTP status code for automated error handling
- Structured logging integration

**Files to Update:**
- `src/lib.rs` - GenaiError enum
- `genai-client/src/errors.rs` - InternalError enum
- All error handling sites (5 locations)

---

### Grounding with Google Search 🎯
**Impact:** High | **Effort:** ~1 week | **Type:** Feature

Implement Gemini's unique real-time web grounding capability that connects models to current web content.

**Why High Priority:**
- Gemini-exclusive feature (differentiates from OpenAI/Anthropic)
- Critical for RAG and real-time information use cases
- Usage-based pricing model ($14/1000 queries) announced for 2025
- Can be combined with other tools (multi-tool use)

**What It Enables:**
```rust
let response = client.with_model("gemini-3-flash")
    .with_prompt("What are the latest developments in Rust async?")
    .with_google_search()  // Enable grounding
    .generate()
    .await?;

// Response includes citations with verifiable sources
for citation in response.grounding_metadata.search_results {
    println!("Source: {}", citation.url);
}
```

**Implementation Details:**
- Add `google_search` tool to Tool enum
- Support grounding metadata in responses
- Handle search query generation by model
- Support multi-tool use (search + code execution)
- Add grounding configuration options

**API Changes:**
```rust
// Tool enum
pub enum ToolType {
    FunctionDeclarations(Vec<FunctionDeclaration>),
    CodeExecution,
    GoogleSearch(GoogleSearchConfig),  // NEW
}

// Response metadata
pub struct GroundingMetadata {
    pub search_results: Vec<SearchResult>,
    pub grounding_support: Option<Vec<GroundingSupport>>,
}
```

**References:**
- [Grounding with Google Search](https://ai.google.dev/gemini-api/docs/google-search)
- [Multi-tool Use Announcement](https://developers.googleblog.com/new-gemini-api-updates-for-gemini-3/)

---

### Complete thinking_level Parameter Support 🎯
**Impact:** Medium-High | **Effort:** ~3-4 hours | **Type:** Feature

Add full support for Gemini 3's `thinking_level` parameter across both APIs.

**Current Status:**
- ✅ Supported in Interactions API (`InteractionConfig`)
- ❌ NOT supported in GenerateContent API

**What It Does:**
Controls the depth of the model's internal reasoning before producing a response.

**Values:**
- Gemini 3 Flash: "minimal", "low", "medium", "high"
- Gemini 3 Pro: "low", "high"

**Implementation:**
```rust
// Add to GenerateContentBuilder
pub fn thinking_level(mut self, level: impl Into<String>) -> Self {
    self.thinking_level = Some(level.into());
    self
}

// Usage
let response = client.with_model("gemini-3-flash")
    .with_prompt("Solve this complex problem...")
    .thinking_level("high")  // NEW
    .generate()
    .await?;
```

**Files to Update:**
- `genai-client/src/models/request.rs` - Add thinking_level to GenerationConfig
- `src/request_builder.rs` - Add thinking_level method
- Add tests for different thinking levels

**References:**
- [Gemini Thinking Documentation](https://ai.google.dev/gemini-api/docs/thinking)
- [thinking_level Parameter Guide](https://www.aifreeapi.com/en/posts/gemini-api-thinking-level)

---

### Rate Limiting & Retry Logic
**Impact:** High | **Effort:** ~6-8 hours | **Type:** Feature

Add production-ready retry logic with exponential backoff for transient failures.

**Features:**
- Automatic retry for 429 (rate limit) and 5xx errors
- Configurable retry attempts and backoff strategy
- Respect `Retry-After` headers
- Circuit breaker pattern for repeated failures

**Proposed API:**
```rust
let client = Client::builder(api_key)
    .retry_config(RetryConfig {
        max_attempts: 3,
        initial_backoff: Duration::from_secs(1),
        max_backoff: Duration::from_secs(30),
        backoff_multiplier: 2.0,
    })
    .build()?;
```

**Dependencies:**
- Consider `tokio-retry` or implement custom logic

---

## Completed

### Thought Signatures Support for Gemini 3 ✓
**Completed:** 2025-12-23 | **Impact:** CRITICAL | **Effort:** ~8 hours | **Type:** Bug Fix / Feature

Fixed critical bug where Gemini 3 function calling was broken due to missing thought signature support.

**What Was Implemented:**
- Added `thought_signature` field to `PartResponse` (GenerateContent API responses)
- Added `thought_signature` field to `Part` (GenerateContent API requests)
- Added `thought_signature` to `InteractionContent::FunctionCall` (Interactions API)
- Added `thought_signatures` field to `GenerateContentResponse` public type
- Updated response processing to extract and preserve signatures
- Created helper functions:
  - `model_function_call_with_signature()`
  - `model_function_calls_request_with_signatures()`
  - `function_call_content_with_signature()`

**Code Sharing Analysis:**
- Core `FunctionCall` struct remains shared between APIs ✓
- Thought signature storage location differs by API design (necessary)
- No unnecessary duplication - implementation is optimal

**What It Enables:**
```rust
// Extract signatures from response
let response = client.with_model("gemini-3-flash-preview")
    .with_prompt("What's the weather?")
    .with_function(weather_fn)
    .generate()
    .await?;

// Pass them back in conversation history for multi-turn
let contents = vec![
    user_text(prompt),
    model_function_calls_request_with_signatures(
        response.function_calls.unwrap(),
        response.thought_signatures  // Critical for Gemini 3!
    ),
    user_tool_response("get_weather", result),
];
```

**Files Modified:**
- `genai-client/src/models/response.rs`
- `genai-client/src/models/shared.rs`
- `genai-client/src/models/interactions.rs`
- `src/types.rs`
- `src/internal/response_processing.rs`
- `src/client.rs`
- `src/content_api.rs`
- `src/interactions_api.rs`
- All tests updated (164 tests passing)

---

### Audit and Simplify Data Structures ✓
**Completed:** 2025-12-23 (PR #7) | **Impact:** Medium | **Effort:** ~6 hours | **Type:** Refactoring

Comprehensive clean architecture refactoring that unified type systems and eliminated duplication.

**Key Improvements:**
- Unified `FunctionDeclaration` type (eliminated ~80 lines of duplication)
- Ergonomic builder pattern for function declarations
- Trait-based reuse via `WithFunctionCalling`
- Symmetric Interactions API design
- Zero-cost abstractions

**Breaking Changes:**
- `FunctionDeclaration` now has nested `FunctionParameters` struct
- `.to_tool()` renamed to `.into_tool()`
- Requires `use rust_genai::WithFunctionCalling;` import

**Net Impact:** +5 lines with major structural improvements (301 insertions, 296 deletions)

---

### Unify Logging Approach ✓
**Completed:** 2025-12-22 (PR #6) | **Impact:** Medium | **Effort:** ~1 hour | **Type:** Refactoring

Replaced ad-hoc println! statements with structured logging using the `log` crate.

**Changes Made:**
- Removed `debug` field from `Client` and `ClientBuilder` structs
- Removed `.debug()` method from `ClientBuilder` (breaking change)
- Replaced all 23 `println!` statements in `src/client.rs` with `log::debug!()`
- Removed all `if self.debug` guards
- Added logging documentation to CLAUDE.md

**Migration Path:**
Users now configure logging via their preferred backend (e.g., `env_logger::init()`) and control output via `RUST_LOG` environment variable.

---

### Consolidate Error Response Handling ✓
**Completed:** 2025-12-23 | **Impact:** Low | **Effort:** ~1 hour | **Type:** Refactoring

Extracted common HTTP error response handling pattern into shared `check_response` helper function.

**Changes Made:**
- Added `check_response()` async function in `genai-client/src/error_helpers.rs`
- Function checks response status, returns `Ok(response)` on success or detailed error on failure
- Leverages existing `read_error_with_context()` for consistent error formatting with HTTP status codes
- Made `error_helpers` module public to allow use from root crate
- Updated 7 call sites across 3 files:
  - `genai-client/src/core.rs`: `generate_content_internal`, `generate_content_stream_internal`
  - `genai-client/src/interactions.rs`: `create_interaction`, `create_interaction_stream`, `get_interaction`, `delete_interaction`
  - `src/client.rs`: `generate_from_request`

**Benefit:** Consistent error handling with HTTP status codes included in all API error messages.

---

## Medium Priority

### Request Timeout Configuration
**Impact:** Medium | **Effort:** ~2-3 hours | **Type:** Feature

Allow users to configure request timeouts for better control over long-running operations.

**Current State:**
- Uses reqwest defaults (30 seconds)
- No user control over timeout behavior
- Can cause issues with long-running generation requests

**Proposed API:**
```rust
let client = Client::builder(api_key)
    .timeout(Duration::from_secs(120))
    .connect_timeout(Duration::from_secs(10))
    .build()?;
```

**Files to Update:**
- `src/client.rs` - Add timeout fields to Client and ClientBuilder
- `genai-client/src/core.rs` - Apply timeout to reqwest client

---

### Response Validation & Better Error Messages
**Impact:** Medium | **Effort:** ~3-4 hours | **Type:** Enhancement

Improve error messages when API returns malformed or unexpected responses.

**Current Issues:**
- Deserialization errors are cryptic
- No validation of required fields
- Unclear errors when API changes

**Improvements:**
```rust
// Current: "missing field `text`"
// Proposed: "API response missing required field `text` in Content.parts[0]"

// Current: "invalid type: null, expected string"
// Proposed: "API returned null for required field `model` (this may indicate an API version mismatch)"
```

**Implementation:**
- Custom deserializers with better error context
- Validation layer between deserialization and business logic
- Version compatibility checks

---

### Performance Benchmarks
**Impact:** Low-Medium | **Effort:** ~4-6 hours | **Type:** Tooling

Establish baseline performance metrics for the library.

**Benchmarks to Create:**
- Request/response serialization overhead
- Streaming throughput (chunks/sec)
- Function calling execution latency
- Memory usage for large conversations
- Concurrent request handling

**Tools:**
- Criterion for benchmarking
- Memory profiling with `heaptrack` or `valgrind`

**Output:**
- `benches/` directory with benchmark suite
- CI integration to track performance over time
- Performance baseline documentation

---

### Security Audit
**Impact:** Medium | **Effort:** ~2-3 hours | **Type:** Audit

Review codebase for common security issues and add security best practices.

**Areas to Review:**
1. **API Key Handling:**
   - Ensure keys aren't logged or exposed in errors
   - Memory zeroing for sensitive data
   - No key leakage in panic messages

2. **Input Validation:**
   - User input sanitization in prompts
   - File path validation (if added in future)
   - Injection prevention in function calls

3. **Dependencies:**
   - Audit for known vulnerabilities with `cargo audit`
   - Keep dependencies up to date
   - Remove unnecessary features

4. **Error Messages:**
   - Don't leak sensitive data in error messages
   - Sanitize API responses before logging

**Deliverables:**
- Security audit report
- `cargo-audit` integration in CI
- Security best practices documentation

---

### Deep Research Agent Support
**Impact:** Medium-High | **Effort:** ~4-6 hours | **Type:** Feature

Add support for Gemini's Deep Research agent, available via the Interactions API.

**What It Is:**
Built-in agent designed for complex, long-running research tasks with unified information synthesis across documents and web data.

**Features:**
- Analyzes PDFs, CSVs, docs, and public web data
- Detailed citations with granular sourcing
- Structured JSON schema outputs
- Report steerability via prompting

**Implementation:**
```rust
// Simple API for Deep Research agent
let research = client.deep_research()
    .with_query("Comprehensive analysis of Rust async patterns")
    .with_documents(vec![pdf1, pdf2])
    .with_output_schema(json_schema)
    .execute()
    .await?;

// Access results
println!("Report: {}", research.report);
for citation in research.citations {
    println!("Source: {}", citation.url);
}
```

**Files to Update:**
- Add `deep_research()` builder method to Client
- Model: "deep-research-pro-preview-12-2025"
- Pricing: $2 per million input tokens
- Use existing Interactions API infrastructure

**References:**
- [Build with Gemini Deep Research](https://blog.google/technology/developers/deep-research-agent-gemini-api/)
- [Deep Research Documentation](https://ai.google.dev/gemini-api/docs/deep-research)

---

### Multi-Tool Use Support
**Impact:** Medium | **Effort:** ~3-4 hours | **Type:** Feature

Support Gemini's ability to use multiple tools simultaneously in a single request.

**Current State:**
- Can use function calling OR code execution
- Cannot combine tools in one request

**What Multi-Tool Enables:**
```rust
let response = client.with_model("gemini-3-flash")
    .with_prompt("Search for current weather data and plot it")
    .with_google_search()
    .with_code_execution()  // Both at once!
    .generate()
    .await?;
```

**Implementation:**
- Modify Tool struct to allow multiple tool types
- Update request serialization to support tool arrays
- Add validation for compatible tool combinations

**Compatible Combinations:**
- Google Search + Code Execution ✅
- Function Calling + Code Execution ✅
- Google Search + Function Calling ✅
- All three together ✅

**References:**
- [Multi-tool Use Announcement](https://developers.googleblog.com/new-gemini-api-updates-for-gemini-3/)

---

### Documentation: Real-World Examples
**Impact:** Medium | **Effort:** ~4-6 hours | **Type:** Documentation

Add more comprehensive examples demonstrating real-world use cases.

**Examples to Add:**
1. **RAG System** - Document Q&A with embeddings and retrieval
2. **Multi-Turn Agent** - Customer support bot with context
3. **Code Assistant** - Code analysis and generation tool
4. **Data Analysis** - CSV analysis with function calling
5. **Web Scraper Agent** - Automated web research with grounding
6. **Testing Assistant** - Test generation from code
7. **Deep Research Demo** - Using the Deep Research agent

**Location:**
- `examples/real_world/` directory
- Each with comprehensive README and comments
- Focus on production patterns and error handling

---

## Strategic Initiatives (2025+)

These items represent major industry trends and standards that could position rust-genai as a leading Rust library for agentic AI development.

_Note: MCP Support and ReAct Pattern have been promoted to High Priority (see above)._

---

### Multi-Agent Orchestration
**Impact:** High | **Effort:** ~3-4 weeks | **Type:** Feature

Implement [multi-agent orchestration patterns](https://research.aimultiple.com/agentic-orchestration/) that mirror enterprise teams - multiple specialized agents coordinated by an orchestrator.

**Patterns to Support:**
1. **Sequential**: Agent A → Agent B → Agent C
2. **Concurrent**: Parallel agent execution with result aggregation
3. **Dynamic Handoff**: Agents decide who handles next step
4. **Hierarchical**: Manager agent delegates to specialist agents

**Proposed API:**
```rust
// Define specialized agents
let researcher = Agent::new("researcher", research_tools);
let writer = Agent::new("writer", writing_tools);
let reviewer = Agent::new("reviewer", review_tools);

// Sequential orchestration
let pipeline = Orchestrator::sequential()
    .add(researcher)
    .add(writer)
    .add(reviewer)
    .build();

// Or hierarchical with manager
let team = Orchestrator::hierarchical()
    .manager(manager_agent)
    .workers(vec![researcher, writer, reviewer])
    .build();

let result = team.execute("Research and write a blog post about Rust async").await?;
```

**Challenges:**
- State sharing between agents
- Partial failure handling
- Cost tracking across agent calls
- Trace/observability for debugging

**References:**
- [Azure: Agent Factory Design Patterns](https://azure.microsoft.com/en-us/blog/agent-factory-the-new-era-of-agentic-ai-common-use-cases-and-design-patterns/)
- [Top Agentic Orchestration Frameworks](https://research.aimultiple.com/agentic-orchestration/)

---

### Study Rust Agent Frameworks
**Impact:** Medium | **Effort:** ~1 week research | **Type:** Research

Evaluate existing Rust agentic frameworks to identify best practices and potential collaboration opportunities.

**Frameworks to Study:**
- **[Kowalski](https://dev.to/yarenty/kowalski-the-rust-native-agentic-ai-framework-53k4)** - Multi-agent orchestration, federation support
- **[AutoAgents](https://github.com/liquidos-ai/AutoAgents)** - WASM compilation, edge deployment
- **[Rig](https://rig.rs/)** - Modular LLM applications
- **[AgentAI](https://docs.rs/agentai)** - Simplified agent creation
- **[Anda](https://github.com/ldclabs/anda)** - ICP blockchain + TEE support

**Goals:**
- Identify common abstractions we should adopt
- Learn from their API designs
- Discover integration opportunities
- Understand why developers are [moving from Python to Rust](https://developers.redhat.com/articles/2025/09/15/why-some-agentic-ai-developers-are-moving-code-python-rust) for agentic AI

---

## Future Features

### Agentic Capabilities
**Impact:** High | **Effort:** ~4-6 hours | **Type:** Feature

Add high-level abstractions for building agentic workflows on top of the Interactions API.

**Status:** 80% ready - Interactions API provides the foundation

**Proposed APIs:**
```rust
// Agent builder
let agent = Agent::builder()
    .with_model("gemini-3-flash-preview")
    .with_tools(vec![...])
    .with_system_instruction("You are a helpful coding assistant")
    .build();

// Conversational agent with memory
let mut conversation = agent.start_conversation();
let response = conversation.send("Hello").await?;
let response2 = conversation.send("What did I just say?").await?;

// Multi-step agent task
let result = agent
    .execute_task("Research and summarize the latest Rust features")
    .with_max_steps(10)
    .with_callback(|step| println!("Step: {step:?}"))
    .await?;
```

**Components Needed:**
- Agent struct wrapping Interactions API (~50 lines)
- Conversation state management (~100 lines)
- Task execution with step tracking (~150 lines)
- Tool execution coordination (~100 lines)

**Estimated Total:** ~400 lines

---

### Gemini Live API Support
**Impact:** High | **Effort:** ~2-3 weeks | **Type:** Feature

Add support for Gemini's real-time bidirectional voice/text API.

**Status:** Not started - significant new work required

**Requirements:**
- WebSocket support (not currently in dependencies)
- Audio streaming (PCM format handling)
- Real-time state management
- Interruption handling
- Voice activity detection integration

**New Dependencies:**
- `tokio-tungstenite` or `async-tungstenite` for WebSocket
- Audio codec support (possibly `opus` or similar)

**Proposed API:**
```rust
let session = client
    .live_session()
    .with_model("gemini-3-flash-preview")
    .with_modalities(vec![Modality::Audio, Modality::Text])
    .connect()
    .await?;

// Send audio
session.send_audio(audio_chunk).await?;

// Receive responses
while let Some(response) = session.next().await {
    match response? {
        LiveResponse::Audio(data) => play_audio(data),
        LiveResponse::Text(text) => println!("{text}"),
        LiveResponse::ToolCall(call) => handle_tool(call),
    }
}
```

**Estimated Total:** ~1500 lines

---

## Contributing

When working on items from this backlog:

1. Create a feature branch from `master`
2. Update this document to move items from their current section to "In Progress" (add a new section if needed)
3. When complete, move to "Completed" section with completion date and relevant commit SHA
4. Consider breaking large features into smaller milestones
