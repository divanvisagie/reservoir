use std::env;

use anyhow::Error;
use http::header;

use crate::models::{
    chat_request::ChatRequest, chat_response::ChatResponse,
};

const RSV_OPENAI_BASE_URL: &str = "RSV_OPENAI_BASE_URL";
const RSV_OLLAMA_BASE_URL: &str = "RSV_OLLAMA_BASE_URL";

fn openai_base_url() -> String {
    env::var(RSV_OPENAI_BASE_URL)
        .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string())
}

fn ollama_base_url() -> String {
    env::var(RSV_OLLAMA_BASE_URL)
        .unwrap_or_else(|_| "http://localhost:11434/v1/chat/completions".to_string())
}

pub struct ModelInfo {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub name: String,
    pub base_url: String,
}

pub enum LanguageModel {
    GPT4_1(ModelInfo),
    GTP4o(ModelInfo),
    Llama3_2(ModelInfo),
    Unknown(ModelInfo),
}

impl LanguageModel {
    pub fn from_str(model_name: &str) -> Self {
        match model_name {
            "gpt-4.1" => LanguageModel::GPT4_1(ModelInfo {
                input_tokens: 128_000,
                output_tokens: 4_096,
                name: "gpt-4-1".to_string(),
                base_url: openai_base_url(),
            }),
            "gpt-4o" => LanguageModel::GTP4o(ModelInfo {
                input_tokens: 128_000,
                output_tokens: 4_096,
                name: "gpt-4o".to_string(),
                base_url: openai_base_url(),
            }),
            "llama3.2" => LanguageModel::Llama3_2(ModelInfo {
                input_tokens: 128_000,
                output_tokens: 2048,
                name: "llama3.2".to_string(),
                base_url: ollama_base_url(),
            }),
            name => LanguageModel::Unknown(ModelInfo {
                input_tokens: 0,
                output_tokens: 0,
                name: name.to_string(),
                base_url: ollama_base_url(),
            }),
        }
    }
}

pub async fn get_completion_message(
    model: &LanguageModel,
    chat_request: &ChatRequest,
) -> Result<ChatResponse, Error> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let client = reqwest::Client::new();

    let model_url = match model {
        LanguageModel::GPT4_1(model_info) => model_info.base_url.clone(),
        LanguageModel::GTP4o(model_info) => model_info.base_url.clone(),
        LanguageModel::Llama3_2(model_info) => model_info.base_url.clone(),
        LanguageModel::Unknown(model_info) => model_info.base_url.clone(),
    };

    let body =
        serde_json::to_string(&chat_request).expect("Failed to serialize chat request model");
    let response = client
        .post(model_url)
        .header("Content-Type", "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
        .body(body)
        .send()
        .await;

    let response_text = match response {
        Ok(resp) => resp.text().await.unwrap_or_else(|e| {
            eprintln!("Error reading response text: {}", e);
            r#"{"error": "Failed to read response text"}"#.to_string()
        }),
        Err(e) => {
            eprintln!("Error sending request to OpenAI: {}", e);
            r#"{"error": "Failed to send request to OpenAI"}"#.to_string()
        }
    };

    let r = ChatResponse::from_json(&response_text)?;
    Ok(r)
}
