# FAQ

## Troubleshooting

### Neo4j Connection Issues

- **Problem**: Unable to connect to Neo4j.
- **Solution**: Ensure Neo4j is running and the connection details in your `.env` file are correct:
  ```env
  NEO4J_URI=bolt://localhost:7687
  NEO4J_USER=neo4j
  NEO4J_PASSWORD=password
  ```

### OpenAI API Key Issues

- **Problem**: Requests fail due to missing or invalid API key.
- **Solution**: Ensure the `OPENAI_API_KEY` environment variable is set correctly.

### Token Limit Errors

- **Problem**: Requests fail due to exceeding the token limit.
- **Solution**: Reduce the size of the input message or the context history.

## Common Questions

### Does Reservoir support streaming responses?

No, streaming responses are not currently supported. All requests are handled in a non-streaming manner.

### Can I use Reservoir with clients other than OpenAI's Python library?

Yes, Reservoir is designed to be OpenAI-compatible. However, compatibility with other clients may vary.