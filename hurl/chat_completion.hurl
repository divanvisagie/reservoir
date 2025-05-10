POST http://127.0.0.1:3017/v1/partition/{{USER}}/instance/reservoir/chat/completions
Content-Type: application/json

{
  "model": "gpt-4.1",
  "messages": [
    {
      "role": "user",
      "content": "Write a one-sentence bedtime story about a brave little toaster."
    }
  ]
}

HTTP/1.1 200
