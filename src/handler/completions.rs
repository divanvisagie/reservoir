use anyhow::Error;
use std::{collections::HashSet, env};

use crate::{clients::embeddings::get_embeddings_for_text, models::{message_node, ChatResponse, Message}, repos::message::MessageRepository};
use bytes::Bytes;
use http::header;
use uuid::Uuid;

use crate::{
    models::{message_node::MessageNode, ChatRequest},
    repos::message::Neo4jMessageRepository,
};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

fn get_last_message_in_chat_request(chat_request: &ChatRequest) -> Option<&str> {
    if let Some(last_message) = chat_request.messages.last() {
        return Some(&last_message.content);
    }
    None
}

fn enrich_chat_request(
    mut similar_messages: Vec<MessageNode>,
    mut last_messages: Vec<MessageNode>, // Add `mut` here
    chat_request: &mut ChatRequest,
) {
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
}

pub async fn handle_with_partition(
    partition: &str,
    instance: Option<String>,
    whole_body: Bytes,
) -> Result<Bytes, Error> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let json_string = String::from_utf8_lossy(&whole_body).to_string();
    let mut chat_request_model = ChatRequest::from_json(json_string.as_str()).expect("Valid JSON");
    let trace_id = Uuid::new_v4().to_string();
    let repo = Neo4jMessageRepository::default();

    let search_term = get_last_message_in_chat_request(&chat_request_model).unwrap_or("");
    let embeddings = get_embeddings_for_text(search_term).await.data[0]
        .embedding
        .clone();

    let instance = instance.unwrap_or(partition.to_string());
    let similar = repo
        .find_similar_messages(
            embeddings,
            trace_id.as_str(),
            partition,
            instance.as_str(),
            5,
        )
        .await
        .expect("Error while finding similar messages");
    let last_messages = repo
        .get_last_messages_for_partition_and_instance(
            partition.to_string(),
            instance.to_string(),
            15,
        )
        .await
        .expect("Error while finding last messages");
    for message in &chat_request_model.messages {
        let node =
            MessageNode::from_message(message, trace_id.as_str(), partition, Some(instance.clone()))
                .await;
        let _save_result = repo.save_message_node(&node).await;
    }
    enrich_chat_request(similar,last_messages, &mut chat_request_model); 

    let whole_body = serde_json::to_string(&chat_request_model)
        .expect("Failed to serialize chat request model");

    // forward the request with reqwest
    let client = reqwest::Client::new();
    let response = client
        .post(OPENAI_API_URL)
        .header("Content-Type", "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
        .body(whole_body)
        .send()
        .await
        .map_err(|e| {
            eprintln!("Error sending request to OpenAI: {}", e);
            e
        });

    let response_text = match response {
        Ok(response) => response.text().await.unwrap(),
        Err(e) => {
            if e.is_timeout() {
                eprintln!("The request timed out.");
            } else if e.is_connect() {
                eprintln!("Failed to connect to the server: {}", e);
            } else if e.is_status() {
                if let Some(status) = e.status() {
                    eprintln!("Received HTTP status code: {}", status);
                }
            }

            if let Some(url) = e.url() {
                eprintln!("URL: {}", url);
            }
            "".to_string()
        }
    };

    let chat_response = ChatResponse::from_json(response_text.as_str())?;
    let message = chat_response.choices[0]
        .message.clone();
    let message_node = MessageNode::from_message(
        &message,
        trace_id.as_str(),
        partition,
        Some(instance.clone()),
    ).await;
    repo.save_message_node(&message_node).await?;

    println!("Response from OpenAI: {:?}", response_text);

    Ok(Bytes::from(response_text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ChatRequest, Message, message_node::MessageNode};
    use std::collections::HashSet;

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
            messages: vec![
                create_dummy_message("user", "current user message"),
            ],
        };

        enrich_chat_request(similar, last, &mut chat_request);

        assert_eq!(chat_request.messages.len(), 1 + 2 + 2 + 2); // Original + 2 system prompts + 2 similar + 2 last
        assert_eq!(chat_request.messages[0].role, "system");
        assert_eq!(chat_request.messages[0].content, "The following is the result of a semantic search of the most related messages by cosine similarity to previous conversations");
        assert_eq!(chat_request.messages[1].role, "user");
        assert_eq!(chat_request.messages[1].content, "similar user 1");
        assert_eq!(chat_request.messages[2].role, "assistant");
        assert_eq!(chat_request.messages[2].content, "similar assistant 1");
        assert_eq!(chat_request.messages[3].role, "system");
        assert_eq!(chat_request.messages[3].content, "The following are the most recent messages in the conversation");
        assert_eq!(chat_request.messages[4].role, "user");
        assert_eq!(chat_request.messages[4].content, "last user 1");
        assert_eq!(chat_request.messages[5].role, "assistant");
        assert_eq!(chat_request.messages[5].content, "last assistant 1");
        assert_eq!(chat_request.messages[6].role, "user");
        assert_eq!(chat_request.messages[6].content, "current user message");
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

        enrich_chat_request(similar, last, &mut chat_request);

        assert_eq!(chat_request.messages.len(), 2 + 2 + 1 + 1); // Original + 2 system prompts + 1 similar + 1 last
        assert_eq!(chat_request.messages[0].role, "system");
        assert_eq!(chat_request.messages[0].content, "initial system prompt");
        assert_eq!(chat_request.messages[1].role, "system"); // Semantic prompt
        assert_eq!(chat_request.messages[2].role, "user");   // Similar message
        assert_eq!(chat_request.messages[3].role, "system"); // Recent prompt
        assert_eq!(chat_request.messages[4].role, "user");   // Last message
        assert_eq!(chat_request.messages[5].role, "user");   // Original user message
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

        enrich_chat_request(similar, last, &mut chat_request);

        assert_eq!(chat_request.messages.len(), 2 + 2 + 1 + 1); // Original + 2 system prompts + 1 similar (deduplicated) + 1 last
        assert_eq!(chat_request.messages[0].role, "system"); // Semantic prompt
        assert_eq!(chat_request.messages[1].role, "assistant"); // "new similar"
        assert_eq!(chat_request.messages[2].role, "system"); // Recent prompt
        assert_eq!(chat_request.messages[3].role, "user");   // Last message
        assert_eq!(chat_request.messages[4].role, "user");   // "already exists"
        assert_eq!(chat_request.messages[5].role, "user");   // "current user message"

        // Check that "already exists" from similar was indeed removed before insertion
        let similar_contents: Vec<&str> = chat_request.messages[1..4].iter().map(|m| m.content.as_str()).collect();
        assert!(!similar_contents.contains(&"already exists"));
        assert!(similar_contents.contains(&"new similar"));
    }

    #[test]
    fn test_enrich_empty_enrichment() {
        let similar = Vec::new();
        let last = Vec::new();
        let mut chat_request = ChatRequest {
            model: "test-model".to_string(),
            messages: vec![
                create_dummy_message("user", "current user message"),
            ],
        };

        let original_len = chat_request.messages.len();
        enrich_chat_request(similar, last, &mut chat_request);

        // Only the two system prompts should be added
        assert_eq!(chat_request.messages.len(), original_len + 2);
        assert_eq!(chat_request.messages[0].role, "system"); // Semantic prompt
        assert_eq!(chat_request.messages[1].role, "system"); // Recent prompt
        assert_eq!(chat_request.messages[2].role, "user");   // Original user message
    }
}
