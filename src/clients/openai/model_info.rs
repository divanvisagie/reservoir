use std::env;

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
    /// The maximum number of input tokens for the model
    pub input_tokens: usize,
    #[allow(dead_code)]
    pub output_tokens: usize,

    /// Name of the model
    pub name: String,
    pub key: String,

    /// Base URL for the model API
    pub base_url: String,
}

impl ModelInfo {
    pub fn new(name: String) -> Self {
        match name.as_str() {
            "gpt-4.1" => Self::new_gpt_4_1(),
            "gpt-4o" => Self::new_gpt_4o(),
            "gpt-4o-mini" => Self::new_gpt_4o_mini(),
            "llama3.2" => Self::new_llama3_2(),
            "mistral-large-2402" => Self::new_mistral_large_2402(),
            "gemini-2.0-flash" => Self::new_gemini_2_0_flash(),
            _ => Self::default(name),
        }
    }

    pub fn new_gpt_4_1() -> Self {
        ModelInfo {
            input_tokens: 128_000,
            output_tokens: 4_096,
            name: "gpt-4.1".to_string(),
            key: env::var("OPENAI_API_KEY").unwrap_or_default(),
            base_url: openai_base_url(),
        }
    }

    pub fn new_gpt_4o() -> Self {
        ModelInfo {
            input_tokens: 128_000,
            output_tokens: 4_096,
            name: "gpt-4o".to_string(),
            key: env::var("OPENAI_API_KEY").unwrap_or_default(),
            base_url: openai_base_url(),
        }
    }

    fn new_gpt_4o_mini() -> ModelInfo {
        ModelInfo {
            input_tokens: 48_000,
            output_tokens: 4_096,
            name: "gpt-4o-mini".to_string(),
            key: env::var("OPENAI_API_KEY").unwrap_or_default(),
            base_url: openai_base_url(),
        }
    }

    fn new_llama3_2() -> ModelInfo {
        ModelInfo {
            input_tokens: 128_000,
            output_tokens: 2048,
            name: "llama3.2".to_string(),
            key: "".to_string(),
            base_url: ollama_base_url(),
        }
    }

    fn new_mistral_large_2402() -> ModelInfo {
        ModelInfo {
            input_tokens: 128_000,
            output_tokens: 2048,
            name: "mistral-large-2402".to_string(),
            key: env::var("MISTRAL_API_KEY").unwrap_or_default(),
            base_url: mistral_base_url(),
        }
    }

    fn new_gemini_2_0_flash() -> ModelInfo {
        ModelInfo {
            input_tokens: 128_000,
            output_tokens: 2048,
            name: "gemini-2.0-flash".to_string(),
            key: env::var("GEMINI_API_KEY").unwrap_or_default(),
            base_url: gemini_base_url(),
        }
    }

    fn default(name: String) -> ModelInfo {
        let ollama_base_url_from_env =
            env::var("OLLAMA_BASE_URL").unwrap_or("http://localhost:11434".to_string());
        let base_url = format!("{}/v1/chat/completions", ollama_base_url_from_env);

        ModelInfo {
            input_tokens: 128_000,
            output_tokens: 2048,
            name,
            key: env::var("OLLAMA_API_KEY").unwrap_or_default(),
            base_url,
        }
    }
}
