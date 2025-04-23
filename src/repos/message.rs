use crate::models::message_node::MessageNode;
use anyhow::Error;
use neo4rs::*;

pub trait MessageRepository {
    async fn save_message_node(&self, message_node: &MessageNode) -> Result<(), Error>;
    // async fn save_message_with_partition(
    //     &self,
    //     message_node: &MessageNode,
    //     partition: &str,
    // ) -> Result<String, Error>;
    async fn get_message_node(&self, trace_id: &str) -> Result<MessageNode, Error>;
    // async fn get_message_node_by_partition(&self, partition: &str) -> Result<Vec<MessageNode>, Error>;
    async fn get_messages_for_partition(
        &self,
        partition: Option<&str>,
    ) -> Result<Vec<MessageNode>, Error>;
    async fn delete_message_node(&self, trace_id: &str) -> Result<i32, Error>;
}

pub struct Neo4jMessageRepository {
    uri: String,
    user: String,
    pass: String,
}

impl Neo4jMessageRepository {
    pub fn new(uri: String, user: String, pass: String) -> Self {
        Neo4jMessageRepository { uri, user, pass }
    }
    pub fn default() -> Self {
        Neo4jMessageRepository {
            uri: "bolt://localhost:7687".to_string(),
            user: "neo4j".to_string(),
            pass: "password".to_string(),
        }
    }
    async fn connect(&self) -> Result<Graph, Error> {
        let config = ConfigBuilder::new()
            .uri(self.uri.clone())
            .user(self.user.clone())
            .password(self.pass.clone())
            .build()?;

        let graph = Graph::connect(config).await?;
        Ok(graph)
    }
}

impl MessageRepository for Neo4jMessageRepository {
    async fn save_message_node(&self, message_node: &MessageNode) -> Result<(), Error> {
        let graph = self.connect().await?;
        let q = query(
            r#"CREATE (m:MessageNode {
                trace_id: $trace_id, 
                content: $content, 
                role: $role, 
                timestamp: $timestamp, 
                partition: $partition
            }) RETURN m"#,
        )
        .param("trace_id", message_node.trace_id.clone())
        .param("content", message_node.content.clone())
        .param("timestamp", message_node.timestamp.clone())
        .param("role", message_node.role.clone())
        .param("partition", message_node.partition.clone());

        let mut result = graph.execute(q).await?;
        let row = result.next().await.unwrap().unwrap();
        let _: MessageNode = row.get("m")?;
        Ok(())
    }

    async fn get_message_node(&self, trace_id: &str) -> Result<MessageNode, Error> {
        let graph = self.connect().await?;
        let q = format!(
            "MATCH (m:MessageNode {{trace_id: '{}'}}) RETURN m",
            trace_id
        );
        let mut result = graph.execute(query(q.as_str())).await?;
        let row = result.next().await.unwrap().unwrap();
        let node: MessageNode = row.get("m")?;
        Ok(node)
    }

    async fn get_messages_for_partition(
        &self,
        partition: Option<&str>,
    ) -> Result<Vec<MessageNode>, Error> {
        let graph = self.connect().await?;

        let q = if let Some(p) = partition {
            query("MATCH (m:MessageNode {partition: $partition}) RETURN m").param("partition", p)
        } else {
            query("MATCH (m:MessageNode) RETURN m")
        };

        let mut result = graph.execute(q).await?;
        let mut messages = Vec::new();

        while let Some(row) = result.next().await? {
            let node: MessageNode = row.get("m")?;
            messages.push(node);
        }

        Ok(messages)
    }

    async fn delete_message_node(&self, trace_id: &str) -> Result<i32, Error> {
        let graph = self.connect().await?;
        let q = format!(
            "MATCH (m:MessageNode {{trace_id: '{}'}}) DELETE m RETURN COUNT(m)",
            trace_id
        );
        let mut result = graph.execute(query(q.as_str())).await?;
        let row = result.next().await.unwrap().unwrap();
        let count: i32 = row.get("COUNT(m)")?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::message_node::MessageNode;

    #[tokio::test]
    async fn test_save_message_node() {
        let repo = Neo4jMessageRepository::new(
            "bolt://localhost:7687".to_string(),
            "neo4j".to_string(),
            "password".to_string(),
        );

        let message_node = MessageNode {
            trace_id: "12345".to_string(),
            partition: "default".to_string(),
            role: "user".to_string(),
            content: Some("Hello, world!".to_string()),
            url: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };
        let result = repo.save_message_node(&message_node).await;
        if result.is_err() {
            println!("Error saving message node: {:?}", result);
        }
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_messages_for_trace_id() {
        let repo = Neo4jMessageRepository::new(
            "bolt://localhost:7687".to_string(),
            "neo4j".to_string(),
            "password".to_string(),
        );

        // add some messages to get
        let first = MessageNode::default()
            .with_trace_id("test-first")
            .with_partition("test");
        let second = MessageNode::default().with_trace_id("test-second");
        repo.save_message_node(&first).await.unwrap();
        repo.save_message_node(&second).await.unwrap();

        let partition = None;
        let result = repo.get_messages_for_partition(partition).await;
        // should be
        if result.is_err() {
            println!("Error getting messages for partition: {:?}", result);
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
        let repo = Neo4jMessageRepository::new(
            "bolt://localhost:7687".to_string(),
            "neo4j".to_string(),
            "password".to_string(),
        );

        let trace_id = "12345";
        let result = repo.delete_message_node(trace_id).await;
        if result.is_err() {
            println!("Error deleting message node: {:?}", result);
        }
        assert!(result.is_ok());
        let result = repo.get_message_node(trace_id).await;
        if result.is_err() {
            println!("Error saving message node: {:?}", result);
        }
    }
}
