# Project Backlog

This document tracks future improvements, refactoring opportunities, and feature ideas for rust-genai.

## Market Context (2025)

The agentic AI landscape is rapidly evolving:

- **Market Growth**: The AI agent market reached $3.7B in 2023 and is expected to double by end of 2025, with ~85% of businesses adopting agents ([sources](https://www.shakudo.io/blog/top-9-ai-agent-frameworks))
- **Industry Shift**: Moving from experimental prototypes to production-ready infrastructure for autonomous, multimodal systems ([IBM](https://www.ibm.com/think/insights/ai-agents-2025-expectations-vs-reality))
- **Open Standards**: Model Context Protocol (MCP) emerging as the dominant standard, with support from Anthropic, OpenAI, Microsoft, Google, AWS, Bloomberg ([Anthropic](https://www.anthropic.com/news/donating-the-model-context-protocol-and-establishing-of-the-agentic-ai-foundation))
- **Technology Stack**: Python + LangChain + OpenAI dominate (73.6%), but Rust adoption growing due to performance/scalability needs ([Red Hat](https://developers.redhat.com/articles/2025/09/15/why-some-agentic-ai-developers-are-moving-code-python-rust))
- **Pattern Convergence**: ReAct (Reasoning + Acting) and multi-agent orchestration becoming standard patterns ([Google Cloud](https://cloud.google.com/architecture/choose-design-pattern-agentic-ai-system))

**Opportunity**: Rust-genai is well-positioned to become the leading Rust library for production agentic AI, especially as systems scale beyond Python's GIL limitations.

## High Priority

_No high priority items at this time._

---

## Completed

### Unify Logging Approach ✓
**Completed:** 2025-12-22 | **Impact:** Medium | **Effort:** ~1 hour | **Type:** Refactoring

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

## Medium Priority

### Consolidate Error Response Handling
**Impact:** Low | **Effort:** ~1-2 hours | **Type:** Refactoring

Extract common error handling pattern into shared helper function.

**Current Pattern (repeated 3+ times):**
```rust
if !response.status().is_success() {
    let error_text = response.text().await.map_err(InternalError::Http)?;
    return Err(InternalError::Api(error_text));
}
```

**Proposed:**
```rust
async fn handle_api_error(response: Response) -> Result<Response, InternalError> {
    if !response.status().is_success() {
        let error_text = response.text().await.map_err(InternalError::Http)?;
        return Err(InternalError::Api(error_text));
    }
    Ok(response)
}

// Usage:
let response = handle_api_error(response).await?;
```

**Files to Update:**
- `genai-client/src/core.rs`
- `genai-client/src/interactions.rs`
- `src/client.rs` (create_interaction, get_interaction, delete_interaction)

---

### Audit and Simplify Data Structures
**Impact:** Medium | **Effort:** ~4-6 hours | **Type:** Refactoring

Comprehensive audit of the main data structures across the codebase to identify opportunities for simplification, consolidation, and improved ergonomics.

**Areas to Review:**

1. **Request/Response Models:**
   - `genai-client/src/models/request.rs` - GenerateContent API request types
   - `genai-client/src/models/response.rs` - GenerateContent API response types
   - `genai-client/src/models/interactions.rs` - Interactions API types
   - `genai-client/src/models/shared.rs` - Shared types between APIs

2. **Content Representations:**
   - Dual content models: `Content`/`Part` (GenerateContent) vs `InteractionContent` (Interactions)
   - Conversion patterns between internal and public types
   - Possibility of unified content representation

3. **Builder Patterns:**
   - `GenerateContentBuilder` vs `InteractionBuilder` - can they share common traits?
   - Repeated conversion logic for `FunctionDeclaration` → `Tool`

4. **Function Calling Types:**
   - Public vs internal `FunctionCall` types (conversion overhead)
   - `FunctionDeclaration` vs `genai_client::FunctionDeclaration`
   - Registry patterns in `src/function_calling.rs`

**Questions to Answer:**
- Can we reduce the number of type conversions between layers?
- Are there redundant or overly complex structures?
- Can we leverage more traits for polymorphism instead of separate types?
- Are Optional fields correctly positioned (API-level vs builder-level)?
- Can we consolidate the two content models into a single abstraction?

**Expected Outcomes:**
- Documentation of current data structure landscape
- Identified simplification opportunities
- Proposal for consolidations (if any)
- Maintain backward compatibility where possible

---

## Strategic Initiatives (2025+)

These items represent major industry trends and standards that could position rust-genai as a leading Rust library for agentic AI development.

### Model Context Protocol (MCP) Support
**Impact:** Very High | **Effort:** ~2-3 weeks | **Type:** Feature | **Priority:** High

Implement support for [Model Context Protocol](https://modelcontextprotocol.io/specification/2025-11-25), the open standard for LLM-tool integration donated by Anthropic to the [Agentic AI Foundation](https://www.anthropic.com/news/donating-the-model-context-protocol-and-establishing-of-the-agentic-ai-foundation) (co-founded with OpenAI, with support from Google, Microsoft, AWS, Cloudflare, Bloomberg).

**Why This Matters:**
- MCP is being adopted across the industry (OpenAI, Microsoft, Google, AWS)
- It's the "LSP for LLMs" - standardized tool/data integration
- Would make rust-genai interoperable with the entire MCP ecosystem
- Users could plug in any MCP server (databases, APIs, filesystems) transparently

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

### ReAct Pattern Implementation
**Impact:** High | **Effort:** ~1-2 weeks | **Type:** Feature

Implement the [ReAct (Reasoning + Acting) pattern](https://www.dailydoseofds.com/ai-agents-crash-course-part-10-with-implementation/), the dominant agentic AI pattern in 2025. This enables agents to alternate between reasoning and taking actions based on observations.

**Current State:** Tool calling exists, but no structured reasoning loop

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
