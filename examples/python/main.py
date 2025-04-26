import os
from openai import OpenAI

client = OpenAI(
    base_url=os.environ.get("OPENAI_API_URL"),
    api_key=os.environ.get("OPENAI_API_KEY")
)

completion = client.chat.completions.create(
    model="gpt-4o",
    messages=[
        {
            "role": "user",
            "content": "What is my name?"
        }
    ]
)
print(completion.choices[0].message.content)
