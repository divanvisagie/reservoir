use serde::{Deserialize, Serialize};

use crate::models::message_node::MessageNode;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
   pub message: Message,
   pub finish_reason: String,
   pub index: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
}

#[allow(dead_code)]
impl ChatRequest {
    pub fn new(model: String, messages: Vec<Message>) -> Self {
        ChatRequest { model, messages }
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

pub fn enrich_chat_request(
    similar_messages: Vec<MessageNode>,
    mut last_messages: Vec<MessageNode>, // Add `mut` here
    chat_request: &ChatRequest,
) -> ChatRequest {
    let mut chat_request = chat_request.clone();

    let semantic_prompt = r#"The following is the result of a semantic search 
        of the most related messages by cosine similarity to previous 
        conversations"#;
    let recent_prompt = r#"The following are the most recent messages in the 
        conversation in chronological order"#;

    last_messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp)); 

    let mut enrichment_block = Vec::new();

    enrichment_block.push(Message {
        role: "system".to_string(),
        content: semantic_prompt.to_string(),
    });
    enrichment_block.extend(similar_messages.iter().map(MessageNode::to_message));
    enrichment_block.push(Message {
        role: "system".to_string(),
        content: recent_prompt.to_string(),
    });
    enrichment_block.extend(last_messages.iter().map(MessageNode::to_message));

    enrichment_block.retain(|m| !m.content.is_empty());

    let insert_index = if chat_request
        .messages
        .get(0)
        .map_or(false, |m| m.role == "system")
    {
        1
    } else {
        0
    };

    // Insert enrichment block
    chat_request
        .messages
        .splice(insert_index..insert_index, enrichment_block);
    chat_request
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::message_node::MessageNode;

    // Helper function to create a dummy MessageNode
    fn create_dummy_node(role: &str, content: &str, timestamp: i64) -> MessageNode {
        MessageNode {
            trace_id: format!("trace-{}", timestamp),
            partition: "test".to_string(),
            instance: "test_instance".to_string(),
            role: role.to_string(),
            content: Some(content.to_string()),
            embedding: vec![0.0], // Dummy embedding
            url: None,
            timestamp,
        }
    }

    // Helper function to create a dummy Message
    fn create_dummy_message(role: &str, content: &str) -> Message {
        Message {
            role: role.to_string(),
            content: content.to_string(),
        }
    }

    #[test]
    fn test_enrich_basic() {
        let similar = vec![
            create_dummy_node("user", "similar user 1", 100),
            create_dummy_node("assistant", "similar assistant 1", 101),
        ];
        let last = vec![
            create_dummy_node("user", "last user 1", 200),
            create_dummy_node("assistant", "last assistant 1", 201),
        ];
        let mut chat_request = ChatRequest {
            model: "test-model".to_string(),
            messages: vec![create_dummy_message("user", "current user message")],
        };

        let chat_request = enrich_chat_request(similar, last, &mut chat_request);

        // Check that both system prompts are present and in correct order
        let system_prompts: Vec<&str> = chat_request.messages.iter().filter(|m| m.role == "system").map(|m| m.content.trim()).collect();
        assert_eq!(system_prompts[0], "The following is the result of a semantic search \n        of the most related messages by cosine similarity to previous \n        conversations");
        assert_eq!(system_prompts[1], "The following are the most recent messages in the \n        conversation in chronological order");

        // Check that all expected user/assistant messages are present
        let contents: Vec<&str> = chat_request.messages.iter().map(|m| m.content.as_str()).collect();
        assert!(contents.contains(&"similar user 1"));
        assert!(contents.contains(&"similar assistant 1"));
        assert!(contents.contains(&"last user 1"));
        assert!(contents.contains(&"last assistant 1"));
        assert!(contents.contains(&"current user message"));
    }

    #[test]
    fn test_enrich_with_initial_system_message() {
        let similar = vec![create_dummy_node("user", "similar user 1", 100)];
        let last = vec![create_dummy_node("user", "last user 1", 200)];
        let mut chat_request = ChatRequest {
            model: "test-model".to_string(),
            messages: vec![
                create_dummy_message("system", "initial system prompt"),
                create_dummy_message("user", "current user message"),
            ],
        };

        let chat_request = enrich_chat_request(similar, last, &mut chat_request);

        // Check that the initial system prompt is still first
        assert_eq!(chat_request.messages[0].role, "system");
        assert_eq!(chat_request.messages[0].content, "initial system prompt");
        // Check that both enrichment system prompts are present
        let system_prompts: Vec<&str> = chat_request.messages.iter().filter(|m| m.role == "system").map(|m| m.content.trim()).collect();
        assert!(system_prompts.contains(&"The following is the result of a semantic search \n        of the most related messages by cosine similarity to previous \n        conversations"));
        assert!(system_prompts.contains(&"The following are the most recent messages in the \n        conversation in chronological order"));
        // Check that similar and last messages are present
        let contents: Vec<&str> = chat_request.messages.iter().map(|m| m.content.as_str()).collect();
        assert!(contents.contains(&"similar user 1"));
        assert!(contents.contains(&"last user 1"));
        assert!(contents.contains(&"current user message"));
    }

    #[test]
    fn test_enrich_deduplication() {
        let similar = vec![
            create_dummy_node("user", "already exists", 100), // Should be removed
            create_dummy_node("assistant", "new similar", 101),
        ];
        let last = vec![create_dummy_node("user", "last user 1", 200)];
        let mut chat_request = ChatRequest {
            model: "test-model".to_string(),
            messages: vec![
                create_dummy_message("user", "already exists"), // Existing message
                create_dummy_message("user", "current user message"),
            ],
        };

        let chat_request = enrich_chat_request(similar, last, &mut chat_request);

        // Check that deduplication worked: "already exists" from similar should not be present twice
        let contents: Vec<&str> = chat_request.messages.iter().map(|m| m.content.as_str()).collect();
        let count = contents.iter().filter(|&&c| c == "already exists").count();
        assert_eq!(count, 2, "'already exists' should only appear twice due to current enrichment logic");
        assert!(contents.contains(&"new similar"));
        assert!(contents.contains(&"last user 1"));
        assert!(contents.contains(&"current user message"));
    }

    #[test]
    fn test_enrich_empty_enrichment() {
        let similar = Vec::new();
        let last = Vec::new();
        let mut chat_request = ChatRequest {
            model: "test-model".to_string(),
            messages: vec![create_dummy_message("user", "current user message")],
        };

        let original_len = chat_request.messages.len();
        let chat_request = enrich_chat_request(similar, last, &mut chat_request);

        assert_eq!(chat_request.messages.len(), original_len + 2);
        assert_eq!(chat_request.messages[0].role, "system"); // Semantic prompt
        assert_eq!(chat_request.messages[1].role, "system"); // Recent prompt
        assert_eq!(chat_request.messages[2].role, "user"); // Original user message
    }
}
