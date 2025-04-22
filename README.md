# reservoir

Reservoir acts as a transparent proxy in front of any OpenAI-compatible API endpoint. Every conversation you have with language models is captured and stored, transforming your 
Reservoir into a living knowledge base that automatically grows with each interaction you make, whether you’re chatting, prompting, or coding.

<image width="256" src="./docs/logo.png" />

## Why?

Think of Reservoir as your personal neural lake:  
- 🗂️ Capture: Every query and response is archived for future use.
- 🔎 Search & Fetch: Effortlessly retrieve past prompts and answers by topic, keyword, or context.
- ⚡ Self-building: The more you interact, the smarter and richer your Reservoir becomes.
- 🔩 Plug-n-play: Drop it in as a gateway and make any OpenAI-powered app on your machine start building knowledge, automagically.

## How Does It Work?

Reservoir sits between your client app and the actual OpenAI API:
```mermaid
sequenceDiagram
    participant A as Your App
    participant B as Reservoir
    participant C as OpenAI API/Ollama
    
    A->>B: API Request
    B->>C: Forward Request
    C->>B: Return Response
    B->>A: Return Response
    Note over B: Stores conversation data
    
```

- **Proxy**: Behaves just like the real API to your apps.
- **Recorder**: Stores all inbound/outbound traffic—questions, answers, metadata, you name it!
- **Knowledge Base**: Over time, Reservoir becomes your own private ChatGPT history, supercharged with search and retrieval.

## Features

- 📖 Full logging & history search
- 🔌 Compatible with OpenAI API clients (Both OpenAI and Ollama)
