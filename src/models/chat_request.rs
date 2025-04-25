use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{message_node::MessageNode, Message};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
}

impl ChatRequest {
    pub fn new(model: String, messages: Vec<Message>) -> Self {
        ChatRequest { model, messages }
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

pub fn enrich_chat_request(
    mut similar_messages: Vec<MessageNode>,
    mut last_messages: Vec<MessageNode>, // Add `mut` here
    chat_request: &ChatRequest,
) -> ChatRequest {
    let mut chat_request = chat_request.clone();
    // Define enrichment prompts
    let semantic_prompt = "The following is the result of a semantic search of the most related messages by cosine similarity to previous conversations";
    let recent_prompt = "The following are the most recent messages in the conversation";

    // Prepare set of (role, content) for deduplication
    let existing: HashSet<(String, String)> = chat_request
        .messages
        .iter()
        .map(|m| (m.role.clone(), m.content.clone()))
        .collect();

    // Remove any similar messages that already exist in the chat
    similar_messages.retain(|m| {
        let msg = MessageNode::to_message(m);
        !existing.contains(&(msg.role.clone(), msg.content.clone()))
    });

    // Sort similar messages chronologically
    similar_messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    // Sort last messages chronologically
    last_messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp)); // Added this line

    // Construct enrichment messages
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

    // Find insertion point: after first system message (if exists), else start
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
