# Features

Reservoir offers the following features:

- 📖 **Logging**: Logs all request/response traffic (user & assistant messages) to Neo4j.
- 🔌 **Compatibility**: OpenAI-compatible API endpoint.
  - **Note**: Currently, **streaming responses are not supported**. All requests are handled in a non-streaming manner.
  - Tested primarily with `curl`, the `openai` Python library, and [Chat Gipitty](https://github.com/divanvisagie/chat-gipitty) (for which Reservoir was initially designed as a memory system). Compatibility with other clients may vary.
- 🏷️ **Partitioning & Instancing**: Organize conversations via URL path using `partition` and `instance` (e.g., `/v1/partition/{partition}/instance/{instance}/chat/completions`).
- 🔗 **Traceability**: Unique trace ID for each request/response cycle.
- 🧠 **Context Enrichment**: Automatically injects relevant past messages (semantically similar and recent within the same partition/instance) into the prompt context.
- ✂️ **Token Management**:
  - Checks if the user's input message exceeds the token limit and returns an error.
  - Automatically truncates the enriched message history (preserving system prompts and the latest user message) if it exceeds the model's context window limit.
- 💾 **Graph Storage**: Uses Neo4j, enabling rich querying and future relationship analysis.
- 💡 **Future**: Plans to refine context enrichment using advanced graph algorithms and vector search.