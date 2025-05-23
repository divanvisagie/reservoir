use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct EmbeddingNode {
    pub id: Option<i64>,
    pub model: String,
    pub embedding: Vec<f32>,
    pub partition: Option<String>,
    pub instance: Option<String>,
}
