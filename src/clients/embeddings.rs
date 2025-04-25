use anyhow::Error;
use reqwest::header;
use serde::{Deserialize, Serialize};
use std::env;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/embeddings"; // Assuming you meant the embeddings endpoint

#[derive(Deserialize, Debug)]
pub struct Embedding {
    object: String,
    index: i32,
    pub embedding: Vec<f32>,
}

#[derive(Deserialize, Debug)]
pub struct EmbeddingResponse {
    // Define this struct according to the API response structure for embeddings
    object: String,
    pub data: Vec<Embedding>,
}

#[derive(Serialize)]
struct EmbeddingRequest {
    input: String,
    model: String,
}

pub async fn get_embeddings_for_text(text: &str) -> Result<EmbeddingResponse, Error> {
    let client = reqwest::Client::new();
    let api_key = env::var("OPENAI_API_KEY")?;

    // Set up the request payload
    let request_body = EmbeddingRequest {
        input: text.to_string(),
        model: "text-embedding-ada-002".to_string(), // Replace with the appropriate model name
    };

    // Set up the request headers
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("Bearer {}", api_key))
            .expect("Invalid API key format"),
    );

    // Send the request and handle response
    let response = client
        .post(OPENAI_API_URL)
        .headers(headers)
        .json(&request_body)
        .send()
        .await;

    match response {
        Ok(res) => {
            if res.status().is_success() {
                let embeddings: EmbeddingResponse =
                    res.json().await?;
                Ok(embeddings)
            } else {
                eprintln!("Error: Received non-success status code {}", res.status());
                let error_response: serde_json::Value =
                    res.json().await.expect("Failed to parse error response");         
                eprintln!("Error response: {:?}", error_response);
                panic!("Failed to get embeddings");
            }
        }
        Err(e) => {
            eprintln!("Error sending request: {}", e);
            panic!("Failed to send request to OpenAI API");
        }
    }
}
