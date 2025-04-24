use crate::{clients::embeddings::get_embeddings_for_text, models::Message};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct MessageNode {
    // Internal tracking
    pub trace_id: String,
    pub partition: String,
    pub instance: String,

    // Actual Languuage model stuff
    pub role: String,
    pub content: Option<String>,
    pub embedding: Vec<f32>,
    pub url: Option<String>,
    pub timestamp: i64,
}

impl MessageNode {
    pub fn new(
        trace_id: String,
        partition: String,
        instance: String,
        role: String,
        content: Option<String>,
        url: Option<String>,
    ) -> Self {
        MessageNode {
            trace_id,
            partition,
            instance,
            role,
            content,
            url,
            embedding: vec![],
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn default() -> Self {
        MessageNode {
            trace_id: "test-traceid".to_string(),
            partition: "default".to_string(),
            instance: "default".to_string(),
            role: "user".to_string(),
            embedding: vec![],
            content: None,
            url: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn to_message(&self) -> Message {
        Message {
            role: self.role.clone(),
            content: self.content.clone().unwrap_or_default(),
        }
    }

    pub fn with_trace_id(mut self, trace_id: &str) -> Self {
        self.trace_id = trace_id.to_string();
        self
    }

    pub fn with_partition(mut self, partition: &str) -> Self {
        self.partition = partition.to_string();
        self
    }

    pub fn with_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    pub async fn from_message(message: &Message, trace_id: &str, partition: &str, instance: Option<String>) -> Self {
        let embeddings = get_embeddings_for_text(message.content.as_str()).await;
        MessageNode {
            trace_id: trace_id.to_string(),
            partition: partition.to_string(),
            instance: instance.unwrap_or(partition.to_string()),
            role: message.role.clone(),
            embedding: embeddings.data[0].embedding.clone(),
            content: Some(message.content.clone()),
            url: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}
