use crate::models::Message;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct MessageNode {
    pub trace_id: String,
    pub partition: String,
    pub instance: String,
    pub content: Option<String>,
    pub role: String,
    pub embedding: Vec<f32>,
    pub url: Option<String>,
    pub timestamp: i64,
}

#[allow(dead_code)]
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

    pub fn from_message(
        message: &Message,
        trace_id: &str,
        partition: &str,
        instance: &str,
        embedding: Vec<f32>,
    ) -> Self {
        MessageNode {
            trace_id: trace_id.to_string(),
            partition: partition.to_string(),
            instance: instance.to_string(),
            role: message.role.clone(),
            embedding,
            content: Some(message.content.clone()),
            url: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}
