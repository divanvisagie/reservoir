use crate::{clients::embedding::EmbeddingClient, models::message_node::MessageNode};
use anyhow::Error;
use neo4rs::*;

use super::Neo4jMessageRepository;

pub trait MessageRepository {
    async fn save_message_node(&self, message_node: &MessageNode) -> Result<(), Error>;
    async fn find_similar_messages(
        &self,
        embedding: Vec<f32>,
        trace_id: &str,
        partition: &str,
        instance: &str,
        top_k: usize,
    ) -> Result<Vec<MessageNode>, Error>;

    async fn get_messages_for_embedding_nodes(
        &self,
        embedding_nodes: Vec<i64>,
        embedding_client: &EmbeddingClient,
    ) -> Result<Vec<MessageNode>, Error>;

    #[allow(dead_code)]
    async fn get_message_node(&self, trace_id: &str) -> Result<MessageNode, Error>;

    #[allow(dead_code)]
    async fn get_message_node_by_embedding_id(
        &self,
        embedding_id: &str,
    ) -> Result<MessageNode, Error>;

    async fn get_messages_for_partition(
        &self,
        partition: Option<&str>,
    ) -> Result<Vec<MessageNode>, Error>;
    async fn get_last_messages_for_partition_and_instance(
        &self,
        partition: String,
        instance: String,
        count: usize,
    ) -> Result<Vec<MessageNode>, Error>;

    #[allow(dead_code)]
    async fn delete_message_node(&self, trace_id: &str) -> Result<i32, Error>;

    async fn find_connections_between_nodes(
        &self,
        nodes: &[MessageNode],
    ) -> Result<Vec<MessageNode>, Error>; // Changed return type
    async fn find_nodes_connected_to_node(
        &self,
        node: &MessageNode,
    ) -> Result<Vec<MessageNode>, Error>; // Changed return type
    async fn connect_synapses(&self) -> Result<(), Error>;
    async fn get_messages(&self) -> Result<Vec<MessageNode>, Error>;
}

pub enum AnyMessageRepository {
    Neo4j(Neo4jMessageRepository),
}

impl AnyMessageRepository {
    pub fn new_neo4j() -> Self {
        AnyMessageRepository::Neo4j(Neo4jMessageRepository::default())
    }
}

impl MessageRepository for AnyMessageRepository {
    async fn save_message_node(&self, message_node: &MessageNode) -> Result<(), Error> {
        match self {
            AnyMessageRepository::Neo4j(repo) => repo.save_message_node(message_node).await,
        }
    }

    async fn find_similar_messages(
        &self,
        embedding: Vec<f32>,
        trace_id: &str,
        partition: &str,
        instance: &str,
        top_k: usize,
    ) -> Result<Vec<MessageNode>, Error> {
        match self {
            AnyMessageRepository::Neo4j(repo) => {
                repo.find_similar_messages(embedding, trace_id, partition, instance, top_k)
                    .await
            }
        }
    }

    async fn get_message_node(&self, trace_id: &str) -> Result<MessageNode, Error> {
        match self {
            AnyMessageRepository::Neo4j(repo) => repo.get_message_node(trace_id).await,
        }
    }

    async fn get_message_node_by_embedding_id(
        &self,
        embedding_id: &str,
    ) -> Result<MessageNode, Error> {
        match self {
            AnyMessageRepository::Neo4j(repo) => {
                repo.get_message_node_by_embedding_id(embedding_id).await
            }
        }
    }

    async fn get_messages_for_partition(
        &self,
        partition: Option<&str>,
    ) -> Result<Vec<MessageNode>, Error> {
        match self {
            AnyMessageRepository::Neo4j(repo) => repo.get_messages_for_partition(partition).await,
        }
    }

    async fn get_last_messages_for_partition_and_instance(
        &self,
        partition: String,
        instance: String,
        count: usize,
    ) -> Result<Vec<MessageNode>, Error> {
        match self {
            AnyMessageRepository::Neo4j(repo) => {
                repo.get_last_messages_for_partition_and_instance(partition, instance, count)
                    .await
            }
        }
    }

    async fn delete_message_node(&self, trace_id: &str) -> Result<i32, Error> {
        match self {
            AnyMessageRepository::Neo4j(repo) => repo.delete_message_node(trace_id).await,
        }
    }

    async fn find_connections_between_nodes(
        &self,
        nodes: &[MessageNode],
    ) -> Result<Vec<MessageNode>, Error> {
        match self {
            AnyMessageRepository::Neo4j(repo) => repo.find_connections_between_nodes(nodes).await,
        }
    }

    async fn find_nodes_connected_to_node(
        &self,
        node: &MessageNode,
    ) -> Result<Vec<MessageNode>, Error> {
        match self {
            AnyMessageRepository::Neo4j(repo) => repo.find_nodes_connected_to_node(node).await,
        }
    }

    async fn connect_synapses(&self) -> Result<(), Error> {
        match self {
            AnyMessageRepository::Neo4j(repo) => repo.connect_synapses().await,
        }
    }

    async fn get_messages_for_embedding_nodes(
        &self,
        embedding_nodes: Vec<i64>,
        embedding_client: &EmbeddingClient,
    ) -> Result<Vec<MessageNode>, Error> {
        match self {
            AnyMessageRepository::Neo4j(repo) => {
                repo.get_messages_for_embedding_nodes(embedding_nodes, embedding_client)
                    .await
            }
        }
    }

    async fn get_messages(&self) -> Result<Vec<MessageNode>, Error> {
        match self {
            AnyMessageRepository::Neo4j(repo) => repo.get_messages().await,
        }
    }
}

#[cfg(test)] // Ignoring tests as requested
mod tests {
    use super::*;
    use crate::models::message_node::MessageNode;
    use tracing::error;

    #[tokio::test]
    async fn test_save_message_node() {
        let repo = Neo4jMessageRepository::default();

        let message_node = MessageNode {
            id: None,
            embedding: vec![],
            trace_id: "12345".to_string(),
            partition: "default".to_string(),
            instance: "default".to_string(),
            role: "user".to_string(),
            content: Some("Hello, world!".to_string()),
            url: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };
        let result = repo.save_message_node(&message_node).await;
        if result.is_err() {
            error!("Error saving message node: {:?}", result);
        }
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_messages_for_trace_id() {
        let repo = Neo4jMessageRepository::default();

        // add some messages to get
        let mut first = MessageNode::default();
        first.trace_id = "test-first".to_string();
        first.partition = "test".to_string();
        let mut second = MessageNode::default();
        second.trace_id = "test-second".to_string();
        repo.save_message_node(&first).await.unwrap();
        repo.save_message_node(&second).await.unwrap();

        let partition = None;
        let result = repo.get_messages_for_partition(partition).await;
        // should be
        if result.is_err() {
            error!("Error getting messages for partition: {:?}", result);
        }

        // we should find a result with the test-first traceId
        let messages = result.unwrap();

        assert_eq!(messages.len() > 2, true);

        let first_message = messages
            .iter()
            .find(|m| m.trace_id == "test-first")
            .unwrap();

        let second_message = messages
            .iter()
            .find(|m| m.trace_id == "test-second")
            .unwrap();

        assert_eq!(first_message.trace_id, "test-first");
        assert_eq!(first_message.partition, "test".to_string());

        assert_eq!(second_message.trace_id, "test-second");
        assert_eq!(second_message.partition, "default");
    }

    #[tokio::test]
    async fn test_delete_message_node() {
        let repo = Neo4jMessageRepository::default();

        let trace_id = "test-delete-node";
        // Ensure the node exists before deleting
        let message_node = MessageNode {
            id: None,
            embedding: vec![],
            trace_id: trace_id.to_string(),
            partition: "default".to_string(),
            instance: "default".to_string(),
            role: "user".to_string(),
            content: Some("To be deleted".to_string()),
            url: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };
        let _ = repo.save_message_node(&message_node).await;

        let result = repo.delete_message_node(trace_id).await;
        if result.is_err() {
            error!("Error deleting message node: {:?}", result);
        }
        assert!(result.is_ok());
        let result = repo.get_message_node(trace_id).await;
        // Should return an error or None, but should not panic
        assert!(
            result.is_err(),
            "Message node should not be found after deletion"
        );
    }
}
