use serde::{Deserialize, Serialize};

use super::{Choice, Usage};


// response
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub usage: Usage,
    pub choices: Vec<Choice>,
}

impl ChatResponse {
    pub fn new(
        id: String,
        object: String,
        created: i64,
        model: String,
        usage: Usage,
        choices: Vec<Choice>,
    ) -> Self {
        ChatResponse {
            id,
            object,
            created,
            model,
            usage,
            choices,
        }
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
