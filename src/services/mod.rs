use anyhow::Error;
use crate::Neo4jMessageRepository;
use crate::repos::message::MessageRepository;

use crate::{clients::openai::{embeddings::get_embeddings_for_text, types::ChatRequest}, models::message_node::MessageNode};

pub struct ChatRequestService <'a>{
    repo: &'a Neo4jMessageRepository,
}

impl <'a> ChatRequestService<'a> {
    pub fn new(repo: &'a Neo4jMessageRepository) -> Self {
        ChatRequestService { repo }
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
            self.repo.save_message_node(&node).await?;
        }
        Ok(())
    }

    pub async fn find_similar_messages(
        &self,
        embedding: Vec<f32>,
        trace_id: &str,
        partition: &str,
        instance: &str,
        top_k: usize
    ) -> Result<Vec<MessageNode>, Error> {
        self.repo.find_similar_messages(embedding, trace_id, partition, instance, top_k).await
    }
}
