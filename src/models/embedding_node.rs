use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct EmbeddingNode {
    pub model: String,
    pub embedding: Vec<f32>
}
