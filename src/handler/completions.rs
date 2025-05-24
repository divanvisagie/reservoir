use anyhow::Error;

use crate::clients::embedding::{get_embeddings_for_txt, EmbeddingClient};
use crate::clients::openai::chat_completions::get_completion_message;
use crate::clients::openai::model_info::ModelInfo;
use crate::clients::openai::types::{
    enrich_chat_request, ChatRequest, ChatResponse, Choice, Message,
};
use crate::models::message_node::MessageNode;
use crate::repos::embedding::AnyEmbeddingRepository;
use crate::repos::message::AnyMessageRepository;
use crate::services::ChatRequestService;
use crate::utils::{
    count_single_message_tokens, deduplicate_message_nodes, get_last_message_in_chat_request,
    truncate_messages_if_needed,
};
use crate::{
    clients::openai::embeddings::get_embeddings_for_text, repos::message::MessageRepository,
};
use bytes::Bytes;
use uuid::Uuid;

use tracing::{error, info};

const SIMILAR_MESSAGES_LIMIT: usize = 7;
const LAST_MESSAGES_LIMIT: usize = 15;

pub async fn is_last_message_too_big(last_message: &Message, model: &ModelInfo) -> Option<Bytes> {
    let input_token_limit = model.input_tokens;
    let last_message_tokens = count_single_message_tokens(last_message);
    if last_message_tokens > input_token_limit {
        info!(
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
            id: None,
            object: None,
            created: None,
            model: None,
            choices: vec![error_choice],
            usage: None,
        };

        // Serialize and return the error response
        let response_bytes = serde_json::to_vec(&error_response).unwrap();
        return Some(Bytes::from(response_bytes));
    } else {
        info!(
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
    let model = ModelInfo::new(chat_request_model.model.clone());

    let trace_id = Uuid::new_v4().to_string();
    let message_repo = AnyMessageRepository::new_neo4j();
    let embeddings_repo = AnyEmbeddingRepository::new_neo4j();
    let service = ChatRequestService::new(&message_repo, &embeddings_repo);

    let last_message = chat_request_model
        .messages
        .last()
        .ok_or_else(|| anyhow::anyhow!("There are no messages in the request"))?;

    let too_big = is_last_message_too_big(last_message, &model).await;
    if let Some(bytes) = too_big {
        return Ok(bytes);
    }

    let search_term = last_message.content.as_str();
    get_last_message_in_chat_request(&chat_request_model)?;

    info!("Using search term: {}", search_term);
    let client = EmbeddingClient::with_fastembed("bge-large-en-v15");
    let embeddings = get_embeddings_for_txt(search_term, client.clone()).await?;

    let mut similar = if !embeddings.is_empty() {
        service
            .find_similar_messages(
                embeddings.clone(),
                &client,
                trace_id.as_str(),
                partition,
                instance,
                SIMILAR_MESSAGES_LIMIT,
            )
            .await
            .unwrap_or_else(|e| {
                error!("Error finding similar messages: {}", e);
                Vec::new()
            })
    } else {
        Vec::new()
    };
    similar = deduplicate_message_nodes(similar);

    let similar_pairs = message_repo
        .find_connections_between_nodes(&similar)
        .await?;
    similar.extend(similar_pairs);
    let first = similar.first().clone();
    let similar = match first {
        Some(first) => {
            let nodes = message_repo.find_nodes_connected_to_node(first).await?;
            let nodes = deduplicate_message_nodes(nodes);

            if nodes.len() > 2 {
                nodes
            } else {
                similar
            }
        }
        None => similar,
    };

    let last_messages = message_repo
        .get_last_messages_for_partition_and_instance(
            partition.to_string(),
            instance.to_string(),
            LAST_MESSAGES_LIMIT,
        )
        .await
        .unwrap_or_else(|e| {
            error!("Error finding last messages: {}", e);
            Vec::new()
        });
    service
        .save_chat_request(&chat_request_model, trace_id.as_str(), partition, instance)
        .await
        .expect("Could not save the request");

    let mut enriched_chat_request =
        enrich_chat_request(similar, last_messages, &mut chat_request_model);
    truncate_messages_if_needed(&mut enriched_chat_request.messages, model.input_tokens);

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
        embedding,
    );
    message_repo
        .save_message_node(&message_node)
        .await
        .expect("Failed to save message node");

    message_repo
        .connect_synapses()
        .await
        .expect("Failed to connect synapses");

    let response_text =
        serde_json::to_string(&chat_response).expect("Failed to serialize chat response");
    Ok(Bytes::from(response_text))
}
