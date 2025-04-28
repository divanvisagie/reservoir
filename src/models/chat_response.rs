use serde::{Deserialize, Serialize};

use super::{Choice, Usage};


// response
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<i64>,
    pub model: Option<String>,
    pub usage: Option<Usage>,
    pub choices: Vec<Choice>,
}

#[allow(dead_code)]
impl ChatResponse {
    pub fn new(
        id: Option<String>,
        object: Option<String>,
        created: Option<i64>,
        model: Option<String>,
        usage: Option<Usage>,
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
