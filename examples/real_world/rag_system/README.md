# RAG System Example

A Retrieval-Augmented Generation (RAG) system demonstrating document Q&A with context retrieval.

## Overview

This example shows how to build a production-style RAG system that:

1. **Document Storage**: Maintains a knowledge base with chunked documents
2. **Retrieval**: Finds relevant context based on user queries
3. **Augmentation**: Injects retrieved context into prompts
4. **Generation**: Uses Gemini to answer questions with cited sources

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  User Query │────▶│  Retriever  │────▶│   Context   │
└─────────────┘     └─────────────┘     └─────────────┘
                           │                    │
                           ▼                    ▼
                    ┌─────────────┐     ┌─────────────┐
                    │  Doc Store  │     │  Augmented  │
                    │  (Chunks)   │     │   Prompt    │
                    └─────────────┘     └─────────────┘
                                               │
                                               ▼
                                        ┌─────────────┐
                                        │   Gemini    │
                                        │   Model     │
                                        └─────────────┘
                                               │
                                               ▼
                                        ┌─────────────┐
                                        │  Response   │
                                        │  + Sources  │
                                        └─────────────┘
```

## Key Concepts

### Document Chunking

Documents are split into manageable chunks with metadata:

```rust
struct DocumentChunk {
    id: String,
    content: String,
    source: String,
    keywords: Vec<String>, // For simulated retrieval
}
```

### Retrieval Strategy

This example uses keyword-based retrieval to simulate semantic search. In production, replace with:

- **Vector Embeddings**: Use Gemini's embedding API or sentence-transformers
- **Vector Database**: Pinecone, Qdrant, Weaviate, Milvus
- **Hybrid Search**: Combine keyword (BM25) with semantic similarity

### Context Injection

Retrieved documents are formatted with source attribution:

```
[Source 1: HR Policy Manual - Chapter 4]
Our company offers 25 days of paid time off...

[Source 2: Benefits Guide 2024]
The company provides health insurance...
```

## Running

```bash
export GEMINI_API_KEY=your_api_key
cargo run --example rag_system
```

## Sample Output

```
📝 User Query: How many vacation days do I get per year?

🔍 Retrieved 1 relevant document(s):
   - [policy-001] HR Policy Manual - Chapter 4: Time Off

🤖 Assistant:
According to the HR Policy Manual (Chapter 4: Time Off), full-time employees
receive 25 days of paid time off (PTO) per year. This accrues at approximately
2.08 days per month. You can carry over up to 10 unused days into the next year.
```

## Production Enhancements

### Vector Embeddings

```rust
// Replace keyword matching with actual embeddings
async fn get_embedding(text: &str, client: &Client) -> Vec<f32> {
    // Use embedding API to get vector representation
    // ...
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    // Calculate similarity between vectors
    // ...
}
```

### Caching Layer

```rust
// Cache frequently accessed queries
struct CachedRetriever {
    store: DocumentStore,
    cache: LruCache<String, Vec<DocumentChunk>>,
}
```

### Quality Monitoring

- Track retrieval precision/recall
- Log query-context pairs for evaluation
- A/B test different chunking strategies
- Monitor response quality scores

## Error Handling

The example demonstrates graceful handling of:

- Missing documents (no relevant context found)
- Empty knowledge base queries
- API errors during generation

## Limitations

This example uses simplified retrieval for demonstration. Production systems should:

- Use actual vector embeddings for semantic search
- Implement chunking strategies (sliding window, semantic boundaries)
- Add reranking for improved relevance
- Handle document updates and versioning
