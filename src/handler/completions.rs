use anyhow::Error;
use std::{collections::HashSet, env};
use tiktoken_rs::o200k_base;

use crate::{
    clients::embeddings::get_embeddings_for_text,
    models::{ChatResponse, Choice, Message, Usage},
    repos::message::MessageRepository,
};
use bytes::Bytes;
use http::header;
use uuid::Uuid;

use crate::{
    models::{message_node::MessageNode, ChatRequest},
    repos::message::Neo4jMessageRepository,
};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const MAX_TOKENS: usize = 64_000; 

// Helper function to estimate tokens for chat messages
// Based on OpenAI cookbook examples
fn count_chat_tokens(messages: &[Message]) -> usize {
    let bpe = o200k_base().unwrap(); // Or handle error appropriately
    let mut num_tokens = 0;
    for message in messages {
        num_tokens += 4; // Every message follows <|start|>{role/name}\n{content}<|end|>\n
        num_tokens += bpe.encode_with_special_tokens(&message.role).len();
        num_tokens += bpe.encode_with_special_tokens(&message.content).len();
    }
    num_tokens += 3; // Every reply is primed with <|start|>assistant<|message|>
    num_tokens
}

// Helper function to estimate tokens for a single chat message
// Slightly simplified version of count_chat_tokens for one message
fn count_single_message_tokens(message: &Message) -> usize {
    let bpe = o200k_base().unwrap(); // Or handle error appropriately
    let mut num_tokens = 0;
    num_tokens += 4; // Overhead for message structure
    num_tokens += bpe.encode_with_special_tokens(&message.role).len();
    num_tokens += bpe.encode_with_special_tokens(&message.content).len();
    // Note: We don't add the +3 for assistant priming here, just the message itself
    num_tokens
}

// Helper function to truncate messages if over token limit, preserving ALL system messages
fn truncate_messages_if_needed(messages: &mut Vec<Message>, limit: usize) {
    let mut current_tokens = count_chat_tokens(messages);

    if current_tokens <= limit {
        return; // No truncation needed
    }

    println!(
        "Token count ({}) exceeds limit ({}), truncating...",
        current_tokens, limit
    );

    // Identify indices of system messages and the last message
    let system_message_indices: HashSet<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role == "system")
        .map(|(i, _)| i)
        .collect();
    let last_message_index = messages.len().saturating_sub(1); // Index of the last message

    // Start checking for removal from the first message
    let mut current_index = 0;

    while current_tokens > limit && current_index < messages.len() {
        // Check if the current index is a system message or the last message
        if system_message_indices.contains(&current_index) || current_index == last_message_index {
            // Skip this message, move to the next index
            current_index += 1;
            continue;
        }

        // If it's safe to remove (not system, not the last message)
        if messages.len() > 1 {
            // Ensure we don't remove the only message left (shouldn't happen here)
            println!(
                "Removing message at index {}: Role='{}', Content='{}...'",
                current_index,
                messages[current_index].role,
                messages[current_index]
                    .content
                    .chars()
                    .take(30)
                    .collect::<String>()
            );
            messages.remove(current_index);
            // Don't increment current_index, as removing shifts subsequent elements down.
            // Recalculate tokens and update system/last indices if needed (though less efficient)
            // For simplicity here, we just recalculate tokens. A more optimized approach
            // might update indices, but given the context size, recalculating tokens is okay.
            current_tokens = count_chat_tokens(messages);
            // Re-evaluate system_message_indices and last_message_index is safer if indices change significantly,
            // but let's stick to the simpler approach for now. If performance becomes an issue, optimize this.
        } else {
            // Safety break: Should not be able to remove the last message due to the check above.
            eprintln!("Warning: Truncation stopped unexpectedly.");
            break;
        }
    }

    println!("Truncated token count: {}", current_tokens);

    if current_tokens > limit {
        eprintln!(
            "Warning: Could not truncate messages enough while preserving system/last messages. Limit: {}, Current: {}",
            limit, current_tokens
        );
    }
}

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

    // ---> START: Check if the last user message is too large <---
    if let Some(last_message) = chat_request_model.messages.last() {
        let last_message_tokens = count_single_message_tokens(last_message);
        if last_message_tokens > MAX_TOKENS {
            println!(
                "Last message token count ({}) exceeds limit ({}), returning error response.",
                last_message_tokens, MAX_TOKENS
            );

            // Construct the error message
            let error_content = format!(
                "Your last message is too long. It contains approximately {} tokens, which exceeds the maximum limit of {}. Please shorten your message.",
                last_message_tokens, MAX_TOKENS
            );
            let error_message = Message {
                role: "assistant".to_string(),
                content: error_content,
            };

            // Create a fake ChatResponse
            let error_choice = Choice {
                index: 0,
                message: error_message,
                finish_reason: "length".to_string(), // Indicate truncation due to length
            };
            let error_response = ChatResponse {
                id: format!("error-{}", trace_id), // Use trace_id for some uniqueness
                object: "chat.completion".to_string(),
                created: chrono::Utc::now().timestamp(), // Safe for recent timestamps
                model: chat_request_model.model.clone(), // Use the requested model name
                choices: vec![error_choice],
                usage: Usage {
                    // Provide dummy usage stats
                    prompt_tokens: last_message_tokens as i64, // Indicate the problematic size
                    completion_tokens: 0,
                    total_tokens: last_message_tokens as i64,
                },
            };

            // Serialize and return the error response
            let response_bytes = serde_json::to_vec(&error_response)?;
            return Ok(Bytes::from(response_bytes));
        }
    }
    // ---> END: Check if the last user message is too large <---

    // --- If the last message is okay, proceed with normal flow ---

    let search_term = get_last_message_in_chat_request(&chat_request_model).unwrap_or("");
    // Handle potential error if embeddings fail
    let embeddings_result = get_embeddings_for_text(search_term).await;
    let embeddings = embeddings_result
        .data[0].embedding.clone();

    let instance = instance.unwrap_or(partition.to_string());

    // Fetch similar messages only if embeddings were successful
    let similar = if !embeddings.is_empty() {
        repo.find_similar_messages(
            embeddings,
            trace_id.as_str(),
            partition,
            instance.as_str(),
            5, // Number of similar messages to fetch
        )
        .await
        .unwrap_or_else(|e| {
            eprintln!("Error finding similar messages: {}", e);
            Vec::new() // Return empty vec on error
        })
    } else {
        Vec::new() // No embeddings, no similar messages
    };

    let last_messages = repo
        .get_last_messages_for_partition_and_instance(
            partition.to_string(),
            instance.to_string(),
            10, // Number of last messages to fetch
        )
        .await
        .unwrap_or_else(|e| {
            eprintln!("Error finding last messages: {}", e);
            Vec::new() // Return empty vec on error
        });

    // Save incoming messages (consider moving this after successful forwarding?)
    for message in &chat_request_model.messages {
        // Skip saving system messages if needed (already handled in repo?)
        let node = MessageNode::from_message(
            message,
            trace_id.as_str(),
            partition,
            Some(instance.clone()),
        )
        .await;
        // Log potential save errors but don't necessarily stop the flow
        if let Err(e) = repo.save_message_node(&node).await {
            eprintln!("Failed to save incoming message node: {}", e);
        }
    }

    // Enrich the request
    enrich_chat_request(similar, last_messages, &mut chat_request_model);

    // Truncate if needed
    truncate_messages_if_needed(&mut chat_request_model.messages, MAX_TOKENS);

    // Serialize the potentially enriched and truncated request
    let whole_body_str =
        serde_json::to_string(&chat_request_model).expect("Failed to serialize chat request model");

    // forward the request with reqwest
    let client = reqwest::Client::new();
    let response = client
        .post(OPENAI_API_URL)
        .header("Content-Type", "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
        .body(whole_body_str) // Use the serialized string
        .send()
        .await;

    // Handle response... (rest of the function)
    // ... existing response handling and saving logic ...

    // Ensure response_text is defined correctly based on response handling
    let response_text = match response {
        Ok(resp) => resp.text().await.unwrap_or_else(|e| {
            eprintln!("Error reading response text: {}", e);
            // Return a default error JSON structure maybe?
            r#"{"error": "Failed to read response text"}"#.to_string()
        }),
        Err(e) => {
            eprintln!("Error sending request to OpenAI: {}", e);
            // Return a default error JSON structure
            r#"{"error": "Failed to send request to OpenAI"}"#.to_string()
        }
    };

    // Attempt to parse the actual response or the error string
    if let Ok(chat_response) = ChatResponse::from_json(&response_text) {
        if let Some(choice) = chat_response.choices.first() {
            let message = &choice.message;
            let message_node = MessageNode::from_message(
                message,
                trace_id.as_str(),
                partition,
                Some(instance.clone()),
            )
            .await;
            if let Err(e) = repo.save_message_node(&message_node).await {
                eprintln!("Failed to save response message node: {}", e);
            }
        }
    } else {
        eprintln!("Failed to parse OpenAI response: {}", response_text);
        // Optionally save the raw error response?
    }

    Ok(Bytes::from(response_text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{message_node::MessageNode, ChatRequest, Message};

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

        enrich_chat_request(similar, last, &mut chat_request);

        assert_eq!(chat_request.messages.len(), 1 + 2 + 2 + 2); // Original + 2 system prompts + 2 similar + 2 last
        assert_eq!(chat_request.messages[0].role, "system");
        assert_eq!(chat_request.messages[0].content, "The following is the result of a semantic search of the most related messages by cosine similarity to previous conversations");
        assert_eq!(chat_request.messages[1].role, "user");
        assert_eq!(chat_request.messages[1].content, "similar user 1");
        assert_eq!(chat_request.messages[2].role, "assistant");
        assert_eq!(chat_request.messages[2].content, "similar assistant 1");
        assert_eq!(chat_request.messages[3].role, "system");
        assert_eq!(
            chat_request.messages[3].content,
            "The following are the most recent messages in the conversation"
        );
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
        assert_eq!(chat_request.messages[2].role, "user"); // Similar message
        assert_eq!(chat_request.messages[3].role, "system"); // Recent prompt
        assert_eq!(chat_request.messages[4].role, "user"); // Last message
        assert_eq!(chat_request.messages[5].role, "user"); // Original user message
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
        assert_eq!(chat_request.messages[3].role, "user"); // Last message
        assert_eq!(chat_request.messages[4].role, "user"); // "already exists"
        assert_eq!(chat_request.messages[5].role, "user"); // "current user message"

        // Check that "already exists" from similar was indeed removed before insertion
        let similar_contents: Vec<&str> = chat_request.messages[1..4]
            .iter()
            .map(|m| m.content.as_str())
            .collect();
        assert!(!similar_contents.contains(&"already exists"));
        assert!(similar_contents.contains(&"new similar"));
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
        enrich_chat_request(similar, last, &mut chat_request);

        // Only the two system prompts should be added
        assert_eq!(chat_request.messages.len(), original_len + 2);
        assert_eq!(chat_request.messages[0].role, "system"); // Semantic prompt
        assert_eq!(chat_request.messages[1].role, "system"); // Recent prompt
        assert_eq!(chat_request.messages[2].role, "user"); // Original user message
    }
}
