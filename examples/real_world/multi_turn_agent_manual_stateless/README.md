# Multi-Turn Agent (Manual + Stateless)

Customer support agent demonstrating **stateless** multi-turn conversations with manual function calling.

## Key Concept

When `store: false`, the server keeps no conversation state. You must:
- Manually build and maintain conversation history
- Send full history with each request
- Cannot use `previous_interaction_id`

## Stateful vs Stateless

| Aspect | Stateful (`store: true`) | Stateless (`store: false`) |
|--------|--------------------------|----------------------------|
| Server state | Yes | No |
| `previous_interaction_id` | Available | Cannot use |
| History management | Automatic | Manual |
| Auto functions | Available | Blocked at runtime |

## When to Use Stateless

- **Privacy**: No server-side conversation storage
- **Custom persistence**: Store conversations in your own database
- **History modification**: Filter or transform history between turns
- **Testing**: Full control for reproducible test scenarios

## Running

```bash
export GEMINI_API_KEY=your_key
cargo run --example multi_turn_agent_manual_stateless

# With wire debugging
LOUD_WIRE=1 cargo run --example multi_turn_agent_manual_stateless
```

## Core Pattern

```rust
struct StatelessSession {
    client: Client,
    conversation_history: Vec<Content>,  // Manual history
}

impl StatelessSession {
    async fn process_message(&mut self, message: &str) -> Result<String> {
        // Add user message to history
        self.conversation_history.push(Content::text(message));

        // Send FULL history each time
        let response = self.client.interaction()
            .with_input(InteractionInput::Content(self.conversation_history.clone()))
            .with_store_disabled()  // <-- Key difference
            .create()
            .await?;

        // Manual function loop, add results to history...

        // Add final response to history
        self.conversation_history.push(Content::text(response.as_text()));
        Ok(response.as_text().to_string())
    }
}
```

## Comparison with Other Examples

| Example | State | Functions | Best For |
|---------|-------|-----------|----------|
| `multi_turn_agent_auto` | Server | Automatic | Quick prototyping |
| `multi_turn_agent_manual` | Server | Manual | Custom execution logic |
| `multi_turn_agent_manual_stateless` | Client | Manual | Privacy, custom storage |

## See Also

- [Multi-Turn Function Calling Guide](../../../docs/MULTI_TURN_FUNCTION_CALLING.md)
