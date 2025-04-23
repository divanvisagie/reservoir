use std::env;
use anyhow::Error;

use http::header;
use bytes::Bytes;
use crate::repos::message::MessageRepository;
use uuid::Uuid;

use crate::{models::{message_node::MessageNode, ChatRequest}, repos::message::Neo4jMessageRepository};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

pub async fn handle_with_partition(partition: &str, whole_body: Bytes) -> Result<Bytes, Error> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let json_string = String::from_utf8_lossy(&whole_body).to_string();
    let chat_request_model = ChatRequest::from_json(json_string.as_str()).expect("Valid JSON");
    let trace_id = Uuid::new_v4().to_string();
    let repo = Neo4jMessageRepository::default();
    for message in &chat_request_model.messages {
        let node = MessageNode::from_message(message, trace_id.as_str(), partition);
        let _save_result = repo.save_message_node(&node).await;
    }

    // forward the request with reqwest
    let client = reqwest::Client::new();
    let response = client
        .post(OPENAI_API_URL)
        .header("Content-Type", "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
        .body(whole_body.to_vec())
        .send()
        .await
        .map_err(|e| {
            eprintln!("Error sending request to OpenAI: {}", e);
            e
        });

    let response = match response {
        Ok(response) => response.text().await.unwrap(),
        Err(e) => {
            if e.is_timeout() {
                eprintln!("The request timed out.");
            } else if e.is_connect() {
                eprintln!("Failed to connect to the server: {}", e);
            } else if e.is_status() {
                if let Some(status) = e.status() {
                    eprintln!("Received HTTP status code: {}", status);
                }
            }

            if let Some(url) = e.url() {
                eprintln!("URL: {}", url);
            }
            "".to_string()
        }
    };

    println!("Response from OpenAI: {:?}", response);

    Ok(Bytes::from(response))
}
