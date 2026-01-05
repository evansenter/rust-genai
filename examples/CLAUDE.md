# Examples Guidelines

## Required Example Format

All examples MUST end with two standardized sections printed at runtime:

### 1. LOUD_WIRE Section

```rust
println!("--- What You'll See with LOUD_WIRE=1 ---");
println!("  [REQ#1] <method> with <key parameters>");
println!("  [RES#1] <status>: <description>");
// ... additional request/response pairs as needed
```

This section documents the wire-level request/response flow when `LOUD_WIRE=1` is set.

### 2. Production Considerations Section

```rust
println!("--- Production Considerations ---");
println!("• <consideration 1>");
println!("• <consideration 2>");
// ... additional considerations
```

This section provides practical tips for using the demonstrated feature in production.

## Template

```rust
println!("\n=== Example Complete ===\n");

println!("--- What You'll See with LOUD_WIRE=1 ---");
println!("  [REQ#1] POST with input + model");
println!("  [RES#1] completed: text response\n");

println!("--- Production Considerations ---");
println!("• First consideration");
println!("• Second consideration");

Ok(())
```

## Rationale

- Consistent user experience across all examples
- Self-documenting wire protocol behavior
- Actionable guidance for production use
- Examples serve as both demos and documentation
