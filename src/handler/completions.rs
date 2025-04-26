use anyhow::Error;
use std::{collections::HashSet, env};
use tiktoken_rs::o200k_base;

use crate::clients::llm::LanguageModel;
use crate::models::chat_response::ChatResponse;
use crate::{
    clients::embeddings::get_embeddings_for_text,
    models::{chat_request::enrich_chat_request, Choice, Message, Usage},
    repos::message::MessageRepository,
};
use bytes::Bytes;
use http::header;
use uuid::Uuid;

use crate::models::chat_request::ChatRequest;
use crate::{models::message_node::MessageNode, repos::message::Neo4jMessageRepository};

const MAX_TOKENS: usize = 64_000;
const SIMILAR_MESSAGES_LIMIT: usize = 7;
const LAST_MESSAGES_LIMIT: usize = 15;

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
            .data[0]
            .embedding
            .clone();
        let node = MessageNode::from_message(message, trace_id, partition, instance, embedding);
        repo.save_message_node(&node).await?;
    }
    Ok(())
}

pub async fn handle_with_partition(
    partition: &str,
    instance: &str,
    whole_body: Bytes,
) -> Result<Bytes, Error> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let json_string = String::from_utf8_lossy(&whole_body).to_string();
    let mut chat_request_model = ChatRequest::from_json(json_string.as_str()).expect("Valid JSON");
    let model = LanguageModel::from_str(&chat_request_model.model);
    let trace_id = Uuid::new_v4().to_string();
    let repo = Neo4jMessageRepository::default();

    let last_message = chat_request_model
        .messages
        .last()
        .expect("There cant be no messages");
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

    let search_term = last_message.content.as_str();
    get_last_message_in_chat_request(&chat_request_model)?;
    println!("Using search term: {}", search_term);
    let embeddings_result = get_embeddings_for_text(search_term).await?;
    let embeddings = embeddings_result.data[0].embedding.clone();

    // Fetch similar messages only if embeddings were successful
    let mut similar = if !embeddings.is_empty() {
        repo.find_similar_messages(
            embeddings,
            trace_id.as_str(),
            partition,
            instance,
            SIMILAR_MESSAGES_LIMIT, // Number of similar messages to fetch
        )
        .await
        .unwrap_or_else(|e| {
            eprintln!("Error finding similar messages: {}", e);
            Vec::new() // Return empty vec on error
        })
    } else {
        Vec::new() // No embeddings, no similar messages
    };

    let similar_pairs = repo.find_connections_between_nodes(&similar).await?;
    similar.extend(similar_pairs);

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
    truncate_messages_if_needed(&mut enriched_chat_request.messages, MAX_TOKENS);
    for message in &enriched_chat_request.messages {
        if message.role == "system".to_string() {
            println!(
                ">> System message content: {}",
                message.content.chars().take(50).collect::<String>()
            );
        } else {
            println!(
                "{}, content: {}",
                message.role,
                message.content.chars().take(50).collect::<String>()
            );
        }
    }

    let body = serde_json::to_string(&enriched_chat_request)
        .expect("Failed to serialize chat request model");
    let client = reqwest::Client::new();

    // get the url out from model
    let model_url = match model {
        LanguageModel::GPT4_1(model_info) => model_info.base_url,
        LanguageModel::GTP4o(model_info) => model_info.base_url,
        LanguageModel::Llama3_2(model_info) => model_info.base_url,
        LanguageModel::Unknown(model_info) => model_info.base_url,
    };

    let response = client
        .post(model_url)
        .header("Content-Type", "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
        .body(body) // Use the serialized string
        .send()
        .await;

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
            let embedding = get_embeddings_for_text(message.content.as_str())
                .await?
                .data[0]
                .embedding
                .clone();
            let message_node = MessageNode::from_message(
                message,
                trace_id.as_str(),
                partition,
                instance,
                embedding,
            );
            if let Err(e) = repo.save_message_node(&message_node).await {
                eprintln!("Failed to save response message node: {}", e);
            }
        }
    } else {
        eprintln!("Failed to parse OpenAI response: {}", response_text);
        // Optionally save the raw error response?
    }

    repo.connect_synapses()
        .await
        .expect("Failed to connect synapses");

    Ok(Bytes::from(response_text))
}
