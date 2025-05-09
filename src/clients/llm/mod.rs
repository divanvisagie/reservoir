use std::env;

use anyhow::Error;
use http::header;
use utils::compress_system_context;
use tracing::{debug, error, info, warn};

pub mod utils;

use crate::models::{chat_request::ChatRequest, chat_response::ChatResponse};

const RSV_OPENAI_BASE_URL: &str = "RSV_OPENAI_BASE_URL";
const RSV_OLLAMA_BASE_URL: &str = "RSV_OLLAMA_BASE_URL";
const RSV_MISTRAL_BASE_URL: &str = "RSV_MISTRAL_BASE_URL";

fn openai_base_url() -> String {
    env::var(RSV_OPENAI_BASE_URL)
        .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string())
}

fn ollama_base_url() -> String {
    env::var(RSV_OLLAMA_BASE_URL)
        .unwrap_or_else(|_| "http://localhost:11434/v1/chat/completions".to_string())
}

fn mistral_base_url() -> String {
    env::var(RSV_MISTRAL_BASE_URL)
        .unwrap_or_else(|_| "https://api.mistral.ai/v1/chat/completions".to_string())
}

fn gemini_base_url() -> String {
 "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions".to_string()
}

pub struct ModelInfo {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub name: String,
    pub key: String,
    pub base_url: String,
}

pub enum LanguageModel {
    OpenAi(ModelInfo),
    Ollama(ModelInfo),
    Mistral(ModelInfo),
    Gemini(ModelInfo),
}

impl LanguageModel {
    pub fn from_str(model_name: &str) -> Self {
        match model_name {
            "gpt-4.1" => LanguageModel::OpenAi(ModelInfo {
                input_tokens: 128_000,
                output_tokens: 4_096,
                name: "gpt-4.1".to_string(),
                key: env::var("OPENAI_API_KEY").unwrap_or_default(),
                base_url: openai_base_url(),
            }),
            "gpt-4o" => LanguageModel::OpenAi(ModelInfo {
                input_tokens: 128_000,
                output_tokens: 4_096,
                name: "gpt-4o".to_string(),
                key: env::var("OPENAI_API_KEY").unwrap_or_default(),
                base_url: openai_base_url(),
            }),
            //anything that starts with gpt
            "gpt-4o-mini" => LanguageModel::OpenAi(ModelInfo {
                input_tokens: 48_000,
                output_tokens: 4_096,
                name: model_name.to_string(),
                key: env::var("OPENAI_API_KEY").unwrap_or_default(),
                base_url: openai_base_url(),
            }),
            "llama3.2" => LanguageModel::Ollama(ModelInfo {
                input_tokens: 128_000,
                output_tokens: 2048,
                name: "llama3.2".to_string(),
                key: env::var("OPENAI_API_KEY").unwrap_or_default(),
                base_url: ollama_base_url(),
            }),
            "mistral-large-2402" => LanguageModel::Mistral(ModelInfo {
                input_tokens: 128_000,
                output_tokens: 2048,
                name: "mistral-large-2402".to_string(),
                key: env::var("MISTRAL_API_KEY").unwrap_or_default(),
                base_url: mistral_base_url(),
            }),
            "gemini-2.0-flash" => LanguageModel::Gemini(ModelInfo {
                input_tokens: 128_000,
                output_tokens: 2048,
                name: "gemini-2.0-flash".to_string(),
                key: env::var("GEMINI_API_KEY").unwrap_or_default(),
                base_url: gemini_base_url(),
            }),
            name => LanguageModel::Ollama(ModelInfo {
                input_tokens: 128_000,
                output_tokens: 2048,
                name: name.to_string(),
                key: "".to_string(),
                base_url: ollama_base_url(),
            }),
        }
    }
}

pub async fn get_completion_message(
    model: &LanguageModel,
    chat_request: &ChatRequest,
) -> Result<ChatResponse, Error> {
    let client = reqwest::Client::new();

    let model_info = match model {
        LanguageModel::Gemini(model_info) => Clone::clone(&model_info),
        LanguageModel::OpenAi(model_info) => Clone::clone(&model_info),
        LanguageModel::Mistral(model_info) => Clone::clone(&model_info),
        LanguageModel::Ollama(model_info) => Clone::clone(&model_info),
    };

    let context = compress_system_context(&chat_request.messages);
    let chat_request = ChatRequest::new(model_info.name.clone(), context);

    let body = match serde_json::to_string(&chat_request) {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to serialize chat request model: {}", e);
            return Err(Error::msg(format!(
                "Failed to serialize chat request: {}",
                e
            )));
        }
    };

    debug!(
        "Sending request to LLM API: {} -  {}\nbody:\n{}",
        body,
        model_info.name.clone(),
        model_info.base_url.clone(),
    );

    let response = client
        .post(model_info.base_url.clone())
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", model_info.key))
        .body(body)
        .send()
        .await;

    let response = match response {
        Ok(resp) => resp,
        Err(e) => {
            error!("Error sending request to LLM API: {}", e);
            return Err(Error::msg(format!(
                "Failed to send request to LLM API: {}",
                e
            )));
        }
    };

    let status = response.status();
    let response_text = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            error!("Error reading response text: {}", e);
            return Err(Error::msg(format!("Failed to read response text: {}", e)));
        }
    };

    if !status.is_success() {
        error!(
            "LLM API returned error status {}: {}",
            status, response_text
        );
        return Err(Error::msg(format!(
            "LLM API error {}: {}",
            status, response_text
        )));
    }

    match ChatResponse::from_json(&response_text) {
        Ok(r) => Ok(r),
        Err(e) => {
            error!(
                "Error parsing response JSON: {}\nRaw response: {}",
                e, response_text
            );
            Err(Error::msg(format!(
                "Failed to parse response JSON: {}\nRaw response: {}",
                e, response_text
            )))
        }
    }
}
