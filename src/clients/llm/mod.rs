pub struct ModelInfo {
    input_tokens: usize,
    output_tokens: usize,
    name: String,
    pub base_url: String
}

pub enum LanguageModel {
    GPT4_1(ModelInfo),
    GTP4o(ModelInfo),
    Llama3_2(ModelInfo),
    Unknown(ModelInfo)
}

impl LanguageModel {
    pub fn from_str(model_name: &str) -> Self {
        match model_name {
            "gpt-4.1" => LanguageModel::GPT4_1(ModelInfo {
                input_tokens: 128_000,
                output_tokens: 4_096,
                name: "gpt-4-1".to_string(),
                base_url: "https://api.openai.com/v1/chat/completions".to_string()
            }),
            "gpt-4o" => LanguageModel::GTP4o(ModelInfo {
                input_tokens: 128_000,
                output_tokens: 4_096,
                name: "gpt-4o".to_string(),
                base_url: "https://api.openai.com/v1/chat/completions".to_string()
            }),
            "llama3-2" => LanguageModel::Llama3_2(ModelInfo {
                input_tokens: 128_000,
                output_tokens: 2048,
                name: "llama3.2".to_string(),
                // use ollama
                base_url:  "http://localhost:11434/v1/chat/completions".to_string()
            }),
            name => LanguageModel::Unknown(ModelInfo {
                input_tokens: 0,
                output_tokens: 0,
                name: name.to_string(),
                base_url: "http://localhost:11434/v1/chat/completions".to_string()
            })
        }
    }
}
