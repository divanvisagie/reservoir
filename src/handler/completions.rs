use anyhow::Error;
use std::collections::HashSet;
use tiktoken_rs::o200k_base;

use crate::clients::llm::{get_completion_message, LanguageModel};
use crate::models::chat_response::ChatResponse;
use crate::{
    clients::embeddings::get_embeddings_for_text,
    models::{chat_request::enrich_chat_request, Choice, Message, Usage},
    repos::message::MessageRepository,
};
use bytes::Bytes;
use uuid::Uuid;

use crate::models::chat_request::ChatRequest;
use crate::{models::message_node::MessageNode, repos::message::Neo4jMessageRepository};

const SIMILAR_MESSAGES_LIMIT: usize = 7;
const LAST_MESSAGES_LIMIT: usize = 15;

fn deduplicate_message_nodes(message_nodes: Vec<MessageNode>) -> Vec<MessageNode> {
    let mut unique_nodes = HashSet::new();
    let mut deduplicated = Vec::new();

    for node in message_nodes {
        if unique_nodes.insert(node.content.clone()) {
            deduplicated.push(node);
        }
    }
    deduplicated
}

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

fn truncate_messages_if_needed(messages: &mut Vec<Message>, limit: usize) {
    let mut current_tokens = count_chat_tokens(messages);
    println!("Current token count: {}", current_tokens);

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

fn get_last_message_in_chat_request(chat_request: &ChatRequest) -> Result<&str, Error> {
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

async fn save_chat_request(
    chat_request: &ChatRequest,
    trace_id: &str,
    partition: &str,
    instance: &str,
) -> Result<(), Error> {
    let repo = Neo4jMessageRepository::default();
    for message in &chat_request.messages {
        let embedding = get_embeddings_for_text(message.content.as_str())
            .await?
            .first()
            .unwrap()
            .embedding
            .clone();
        let node = MessageNode::from_message(message, trace_id, partition, instance, embedding);
        repo.save_message_node(&node).await?;
    }
    Ok(())
}

pub async fn is_last_message_too_big(
    last_message: &Message,
    model: &LanguageModel,
    trace_id: &str,
) -> Option<Bytes> {
    let model = match model {
        LanguageModel::GPT4_1(model_info) => model_info,
        LanguageModel::GTP4o(model_info) => model_info,
        LanguageModel::Llama3_2(model_info) => model_info,
        LanguageModel::MistralLarge2402(model_info) => model_info,
        LanguageModel::Unknown(model_info) => model_info,
    };
    let input_token_limit = model.input_tokens;
    let last_message_tokens = count_single_message_tokens(last_message);
    if last_message_tokens > input_token_limit {
        println!(
            "Last message token count ({}) exceeds limit ({}), returning error response.",
            last_message_tokens, input_token_limit
        );

        let error_content = format!(
                "Your last message is too long. It contains approximately {} tokens, which exceeds the maximum limit of {}. Please shorten your message.",
                last_message_tokens, input_token_limit
            );
        let error_message = Message {
            role: "assistant".to_string(),
            content: error_content,
        };

        let error_choice = Choice {
            index: 0,
            message: error_message,
            finish_reason: "length".to_string(), // Indicate truncation due to length
        };
        let error_response = ChatResponse {
            id: format!("error-{}", trace_id),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.name.clone(),
            choices: vec![error_choice],
            usage: Usage {
                prompt_tokens: last_message_tokens as i64, // Indicate the problematic size
                completion_tokens: 0,
                total_tokens: last_message_tokens as i64,
            },
        };

        // Serialize and return the error response
        let response_bytes = serde_json::to_vec(&error_response).unwrap();
        return Some(Bytes::from(response_bytes));
    } else {
        println!(
            "Last message token count ({}) is within limit ({}).",
            last_message_tokens, input_token_limit
        );
        return None;
    }
}
pub async fn handle_with_partition(
    partition: &str,
    instance: &str,
    whole_body: Bytes,
) -> Result<Bytes, Error> {
    let json_string = String::from_utf8_lossy(&whole_body).to_string();
    let mut chat_request_model = ChatRequest::from_json(json_string.as_str()).expect("Valid JSON");
    let model = LanguageModel::from_str(&chat_request_model.model);

    let (input_token_limit, _output_token_limit) = match &model {
        LanguageModel::GPT4_1(info) => (info.input_tokens, info.output_tokens),
        LanguageModel::GTP4o(info) => (info.input_tokens, info.output_tokens),
        LanguageModel::Llama3_2(info) => (info.input_tokens, info.output_tokens),
        LanguageModel::MistralLarge2402(info) => (info.input_tokens, info.output_tokens),
        LanguageModel::Unknown(info) => (info.input_tokens, info.output_tokens),
    };
    let trace_id = Uuid::new_v4().to_string();
    let repo = Neo4jMessageRepository::default();

    let last_message = chat_request_model
        .messages
        .last()
        .ok_or_else(|| anyhow::anyhow!("There are no messages in the request"))?;

    let too_big = is_last_message_too_big(last_message, &model, &trace_id).await;
    if let Some(bytes) = too_big {
        return Ok(bytes);
    }

    let search_term = last_message.content.as_str();
    get_last_message_in_chat_request(&chat_request_model)?;

    println!("Using search term: {}", search_term);
    let embeddings = get_embeddings_for_text(search_term)
        .await?
        .first()
        .unwrap()
        .embedding
        .clone();

    let mut similar = if !embeddings.is_empty() {
        repo.find_similar_messages(
            embeddings,
            trace_id.as_str(),
            partition,
            instance,
            SIMILAR_MESSAGES_LIMIT,
        )
        .await
        .unwrap_or_else(|e| {
            eprintln!("Error finding similar messages: {}", e);
            Vec::new()
        })
    } else {
        Vec::new() 
    };

    let similar_pairs = repo.find_connections_between_nodes(&similar).await?;
    similar.extend(similar_pairs);
    let first = similar.first().clone();
    let similar = match first {
        Some(first) => {
            let r = repo.find_nodes_connected_to_node(first).await?;
            let r = deduplicate_message_nodes(r);

            if r.len() > 2 {
                r
            } else {
                similar
            }
        }
        None => similar,
    };

    let last_messages = repo
        .get_last_messages_for_partition_and_instance(
            partition.to_string(),
            instance.to_string(),
            LAST_MESSAGES_LIMIT,
        )
        .await
        .unwrap_or_else(|e| {
            eprintln!("Error finding last messages: {}", e);
            Vec::new() // Return empty vec on error
        });
    save_chat_request(&chat_request_model, trace_id.as_str(), partition, instance)
        .await
        .expect("Could not save the request");

    let mut enriched_chat_request =
        enrich_chat_request(similar, last_messages, &mut chat_request_model);
    truncate_messages_if_needed(&mut enriched_chat_request.messages, input_token_limit);

    let chat_response = get_completion_message(&model, &enriched_chat_request)
        .await
        .expect("Failed to get completion message");
    
    let message_node = chat_response.choices.first().unwrap().message.clone();
    let embedding = get_embeddings_for_text(message_node.content.as_str())
        .await?
        .first()
        .unwrap()
        .embedding
        .clone();
    let message_node = MessageNode::from_message(
        &message_node,
        trace_id.as_str(),
        partition,
        instance,
        embedding
    );
    repo.save_message_node(&message_node)
        .await
        .expect("Failed to save message node");

    repo.connect_synapses()
        .await
        .expect("Failed to connect synapses");

    let response_text = serde_json::to_string(&chat_response)
        .expect("Failed to serialize chat response");
    Ok(Bytes::from(response_text))
}
