# API

Reservoir provides an OpenAI-compatible API endpoint. Below is an overview of its usage and examples.

## URL Structure

`/v1/partition/{partition}/instance/{instance}/chat/completions`

- `{partition}`: A broad category (e.g., project name, application name).
- `{instance}`: A specific context within the partition (e.g., user ID, session ID, specific feature).

### Example

- **Instead of**:
  `https://api.openai.com/v1/chat/completions`
- **Use**:
  `http://localhost:3017/v1/partition/$USER/instance/my-application/chat/completions`

Here, `$USER` is the system username, and `my-application` is the instance. Context enrichment and history retrieval are scoped to the specific `partition`/`instance` combination.

### Curl Example

```bash
# Ensure OPENAI_API_KEY is set
# Replace 'my-application' with your application name

curl http://localhost:3017/v1/partition/$USER/instance/my-application/chat/completions \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $OPENAI_API_KEY" \
    -d '{
        "model": "gpt-4",
        "messages": [
            {
                "role": "user",
                "content": "Write a one-sentence bedtime story about a brave little toaster."
            }
        ]
    }'
```

### Python Example (using `openai` library)

```python
import os
from openai import OpenAI

# Replace with your application name
INSTANCE = "my-application"
PARTITION = os.getenv("USER")  # System username
RESERVOIR_PORT = os.getenv('RESERVOIR_PORT', '3017')

# Construct the base URL dynamically
RESERVOIR_BASE_URL = f"http://localhost:{RESERVOIR_PORT}/v1/partition/{PARTITION}/instance/{INSTANCE}"

client = OpenAI(
    base_url=RESERVOIR_BASE_URL,
    api_key=os.environ.get("OPENAI_API_KEY")
)

completion = client.chat.completions.create(
    model="gpt-4",
    messages=[
        {
            "role": "user",
            "content": "Write a one-sentence bedtime story about a curious robot."
        }
    ]
)
print(completion.choices[0].message.content)
```

**Note:**
- The request structure (headers, body) remains identical to a direct OpenAI call.
- The **URL** points to Reservoir (`http://localhost:3017`).
- The **path** now includes `/v1/partition/{partition}/instance/{instance}/chat/completions`.
- Context enrichment and history lookups are scoped to the specific `partition` and `instance` provided in the URL.
- Input token limit checks and automatic truncation still apply.

Reservoir forwards the request (including `Authorization`) to OpenAI and stores the conversation tagged with the specified `partition` and `instance`.