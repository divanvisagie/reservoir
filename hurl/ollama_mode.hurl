POST http://127.0.0.1:11434/v1/chat/completions
Content-Type: application/json

{
  "model": "gpt-4.1",
  "messages": [
    {
      "role": "user",
      "content": "what is 1 - 1"
    }
  ]
}

HTTP/1.1 200