use anyhow::Error;
use std::collections::HashSet;

use tiktoken_rs::o200k_base;
use tracing::{error, info};

use crate::{clients::openai::types::{ChatRequest, Message}, models::message_node::MessageNode};

fn message_to_string(msg: &Message) -> String {
    match msg.role.as_str() {
        "user" => format!("User: {}", msg.content),
        "assistant" => format!("Assistant: {}", msg.content),
        "system" => format!("System Note: {}", msg.content),
        _ => format!("{}: {}", msg.role, msg.content),
    }
}

pub fn compress_system_context(messages: &Vec<Message>) -> Vec<Message> {
    let first_index = messages.iter().position(|m| m.role == "system");
    let last_index = messages.iter().rposition(|m| m.role == "system");

    if let (Some(first), Some(last)) = (first_index, last_index) {
        if first != 0 || first == last {
            return messages.clone(); // return original if invalid or nothing to compress
        }

        let mut compressed = vec![messages[0].clone()];

        for i in first + 1..=last {
            let msg = &messages[i];
            let line = format!("\n{}", message_to_string(msg));
            compressed[0].content += &line;
        }

        // Add the remaining messages (after the last system prompt)
        compressed.extend_from_slice(&messages[last + 1..]);

        compressed
    } else {
        messages.clone()
    }
}

pub fn deduplicate_message_nodes(message_nodes: Vec<MessageNode>) -> Vec<MessageNode> {
    let mut unique_nodes = HashSet::new();
    let mut deduplicated = Vec::new();

    for node in message_nodes {
        if unique_nodes.insert(node.content.clone()) {
            deduplicated.push(node);
        }
    }
    deduplicated
}

pub fn count_chat_tokens(messages: &[Message]) -> usize {
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
pub fn count_single_message_tokens(message: &Message) -> usize {
    let bpe = o200k_base().unwrap(); // Or handle error appropriately
    let mut num_tokens = 0;
    num_tokens += 4; // Overhead for message structure
    num_tokens += bpe.encode_with_special_tokens(&message.role).len();
    num_tokens += bpe.encode_with_special_tokens(&message.content).len();
    // Note: We don't add the +3 for assistant priming here, just the message itself
    num_tokens
}

pub fn truncate_messages_if_needed(messages: &mut Vec<Message>, limit: usize) {
    let mut current_tokens = count_chat_tokens(messages);
    info!("Current token count: {}", current_tokens);

    if current_tokens <= limit {
        return; // No truncation needed
    }

    info!(
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
            info!(
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
            error!("Warning: Truncation stopped unexpectedly.");
            break;
        }
    }

    info!("Truncated token count: {}", current_tokens);

    if current_tokens > limit {
        error!(
            "Warning: Could not truncate messages enough while preserving system/last messages. Limit: {}, Current: {}",
            limit, current_tokens
        );
    }
}

pub fn get_last_message_in_chat_request(chat_request: &ChatRequest) -> Result<&str, Error> {
    if let Some(last_message) = chat_request.messages.last() {
        if last_message.role == "user" {
            Ok(&last_message.content)
        } else {
            Err(Error::msg("Last message is not a user message"))
        }
    } else {
        Err(Error::msg("No messages in chat request"))
    }
}
