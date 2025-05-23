use crate::repos::embedding::{AnyEmbeddingRepository, EmbeddingRepository};
use crate::repos::message::AnyMessageRepository;
use crate::repos::message::MessageRepository;
use anyhow::Error;
use tracing::info;

use crate::{
    clients::openai::{embeddings::get_embeddings_for_text, types::ChatRequest},
    models::message_node::MessageNode,
};

pub struct ChatRequestService<'a> {
    message_repo: &'a AnyMessageRepository,
    embeddings_repo: &'a AnyEmbeddingRepository,
}

impl<'a> ChatRequestService<'a> {
    pub fn new(
        message_repo: &'a AnyMessageRepository,
        embeddings_repo: &'a AnyEmbeddingRepository,
    ) -> Self {
        ChatRequestService {
            message_repo,
            embeddings_repo,
        }
    }

    pub async fn save_chat_request(
        &self,
        chat_request: &ChatRequest,
        trace_id: &str,
        partition: &str,
        instance: &str,
    ) -> Result<(), Error> {
        for message in &chat_request.messages {
            let embedding = get_embeddings_for_text(message.content.as_str())
                .await?
                .first()
                .unwrap()
                .embedding
                .clone();
            let node = MessageNode::from_message(message, trace_id, partition, instance, embedding);
            self.message_repo.save_message_node(&node).await?;
        }
        Ok(())
    }

    pub async fn find_similar_messages(
        &self,
        embedding: Vec<f32>,
        trace_id: &str,
        partition: &str,
        instance: &str,
        top_k: usize,
    ) -> Result<Vec<MessageNode>, Error> {
        let embedding_result = self
            .embeddings_repo
            .find_similar_embeddings(embedding.clone(), partition, instance, top_k)
            .await;

        match embedding_result {
            Ok(embeddings) => {
                if embeddings.is_empty() {
                    info!("No similar embeddings found");
                    return Ok(vec![]);
                }
                info!("Found similar embeddings: {:?}", embeddings);
            }
            Err(e) => {
                info!("Error finding similar embeddings: {}", e);
                return Err(e);
            }
        }

        self.message_repo
            .find_similar_messages(embedding, trace_id, partition, instance, top_k)
            .await
    }

    pub(crate) async fn find_connections_between_nodes(
        &self,
        similar: &[MessageNode],
    ) -> Result<Vec<MessageNode>, Error> {
        self.message_repo
            .find_connections_between_nodes(similar)
            .await
    }

    pub(crate) async fn find_nodes_connected_to_node(
        &self,
        first: &MessageNode,
    ) -> Result<Vec<MessageNode>, Error> {
        self.message_repo.find_nodes_connected_to_node(first).await
    }

    pub(crate) async fn get_messages_for_partition(
        &self,
        partition: Option<&str>,
    ) -> Result<Vec<MessageNode>, Error> {
        self.message_repo
            .get_messages_for_partition(partition)
            .await
    }
}
