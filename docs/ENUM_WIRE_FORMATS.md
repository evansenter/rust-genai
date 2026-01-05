# Enum Wire Formats

This document captures the actual wire formats for enums in the Gemini Interactions API. **The official documentation is sometimes wrong** - this reflects empirically tested values.

## Quick Reference

| Enum | Wire Format | Example | Notes |
|------|-------------|---------|-------|
| `ThinkingSummaries` | `THINKING_SUMMARIES_*` | `"THINKING_SUMMARIES_AUTO"` | Docs claim `auto`/`none` - **wrong** |
| `ThinkingLevel` | lowercase | `"low"`, `"medium"`, `"high"` | Docs are correct |
| `FunctionCallingMode` | SCREAMING_CASE | `"AUTO"`, `"ANY"`, `"NONE"`, `"VALIDATED"` | |
| `InteractionStatus` | snake_case | `"in_progress"`, `"requires_action"` | Response-only |
| `Resolution` | snake_case | `"low"`, `"medium"`, `"high"`, `"ultra_high"` | Image/video content |
| `Tool::ComputerUse` | snake_case | `"computer_use"` | **UNVERIFIED** - from docs |
| `InteractionContent::ComputerUseCall` | snake_case | `"computer_use_call"` | **UNVERIFIED** - from docs |
| `InteractionContent::ComputerUseResult` | snake_case | `"computer_use_result"` | **UNVERIFIED** - from docs |

## Details

### ThinkingSummaries (agent_config)

Used in `agent_config.thinkingSummaries` for Deep Research agent.

```json
{
  "agent_config": {
    "type": "deep-research",
    "thinkingSummaries": "THINKING_SUMMARIES_AUTO"
  }
}
```

| Rust Enum | Wire Value | Doc Claims (wrong) |
|-----------|------------|-------------------|
| `ThinkingSummaries::Auto` | `"THINKING_SUMMARIES_AUTO"` | `"auto"` |
| `ThinkingSummaries::None` | `"THINKING_SUMMARIES_NONE"` | `"none"` |

**Discovered**: 2026-01-04 - API returned `"unknown enum value: 'auto'"` until we tested the fully-qualified format.

### ThinkingLevel (generation_config)

Used in `generationConfig.thinkingLevel`.

```json
{
  "generationConfig": {
    "thinkingLevel": "low"
  }
}
```

| Rust Enum | Wire Value |
|-----------|------------|
| `ThinkingLevel::Minimal` | `"minimal"` |
| `ThinkingLevel::Low` | `"low"` |
| `ThinkingLevel::Medium` | `"medium"` |
| `ThinkingLevel::High` | `"high"` |

### FunctionCallingMode (generation_config)

Used in `generationConfig.toolChoice`.

```json
{
  "generationConfig": {
    "toolChoice": "ANY"
  }
}
```

| Rust Enum | Wire Value |
|-----------|------------|
| `FunctionCallingMode::Auto` | `"AUTO"` |
| `FunctionCallingMode::Any` | `"ANY"` |
| `FunctionCallingMode::None` | `"NONE"` |
| `FunctionCallingMode::Validated` | `"VALIDATED"` |

### InteractionStatus (response)

Returned in API responses - we only deserialize, never serialize.

| Rust Enum | Wire Value |
|-----------|------------|
| `InteractionStatus::Completed` | `"completed"` |
| `InteractionStatus::InProgress` | `"in_progress"` |
| `InteractionStatus::RequiresAction` | `"requires_action"` |
| `InteractionStatus::Failed` | `"failed"` |
| `InteractionStatus::Cancelled` | `"cancelled"` |

### Resolution (content)

Used in image and video content for quality vs. token cost trade-off.

```json
{
  "input": [{
    "type": "image",
    "data": "base64...",
    "mime_type": "image/png",
    "resolution": "low"
  }]
}
```

| Rust Enum | Wire Value |
|-----------|------------|
| `Resolution::Low` | `"low"` |
| `Resolution::Medium` | `"medium"` |
| `Resolution::High` | `"high"` |
| `Resolution::UltraHigh` | `"ultra_high"` |

**Verified**: 2026-01-05 - Tested with `LOUD_WIRE=1 cargo run --example multimodal_image`.

### Computer Use (tool and content types)

**⚠️ UNVERIFIED** - Wire format derived from [Interactions API docs](https://ai.google.dev/static/api/interactions.md.txt). Pending verification with `LOUD_WIRE=1` once API access is available.

Tool request format (assumed):
```json
{
  "tools": [{
    "type": "computer_use",
    "environment": "browser",
    "excludedPredefinedFunctions": ["submit_form", "download"]
  }]
}
```

Content types (assumed):
```json
// ComputerUseCall (in response outputs)
{
  "type": "computer_use_call",
  "id": "call_123",
  "action": "navigate",
  "parameters": {"url": "https://example.com"}
}

// ComputerUseResult (in response outputs)
{
  "type": "computer_use_result",
  "call_id": "call_123",
  "success": true,
  "output": {"title": "Example Domain"},
  "screenshot": "base64..."
}
```

| Rust Type | Wire Value | Notes |
|-----------|------------|-------|
| `Tool::ComputerUse` | `"computer_use"` | Tool type |
| `Tool::ComputerUse.environment` | `"browser"` | Only supported value |
| `Tool::ComputerUse.excluded_predefined_functions` | `"excludedPredefinedFunctions"` | camelCase field name |
| `InteractionContent::ComputerUseCall` | `"computer_use_call"` | Content type |
| `InteractionContent::ComputerUseResult` | `"computer_use_result"` | Content type |

**TODO**: Verify with `LOUD_WIRE=1 cargo run --example computer_use` when API access is available.

## Testing New Enums

When adding new enums, always test the actual wire format with `curl`:

```bash
# Test what the API actually accepts
curl -s "https://generativelanguage.googleapis.com/v1beta/interactions?key=$GEMINI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model": "gemini-3-flash-preview", "input": "test", ...}'
```

Common patterns to try:
1. lowercase: `"auto"`
2. SCREAMING_CASE: `"AUTO"`
3. Fully-qualified: `"ENUM_NAME_VALUE"` (e.g., `"THINKING_SUMMARIES_AUTO"`)

## Evergreen Pattern

All enums implement the Evergreen pattern with an `Unknown` variant that preserves unrecognized values:

```rust
#[non_exhaustive]
pub enum ThinkingSummaries {
    Auto,
    None,
    Unknown {
        summaries_type: String,
        data: serde_json::Value,
    },
}
```

This ensures forward compatibility when Google adds new enum values.
