use crate::models::message_node::MessageNode;
use anyhow::Error;
use neo4rs::*;

pub trait MessageRepository {
    async fn save_message_node(&self, message_node: &MessageNode) -> Result<(), Error>;
    async fn find_similar_messages(
        &self,
        embedding: Vec<f32>,
        trace_id: &str,
        partition: &str,
        instance: &str,
        top_k: i64,
    ) -> Result<Vec<MessageNode>, Error>;
    async fn get_message_node(&self, trace_id: &str) -> Result<MessageNode, Error>;
    async fn get_messages_for_partition(
        &self,
        partition: Option<&str>,
    ) -> Result<Vec<MessageNode>, Error>;
    async fn get_last_messages_for_partition_and_instance(
        &self,
        partition: String,
        instance: String,
        count: i32,
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

    async fn init_vector_index(&self, graph: &Graph, index_name: &str) -> Result<(), Error> {
        // Check if index already exists
        let check_query = query("SHOW INDEXES YIELD name RETURN name");
        let mut result = graph.execute(check_query).await?;

        while let Ok(Some(row)) = result.next().await {
            let name: String = row.get("name")?;
            if name == index_name {
                // Index already exists, nothing to do
                return Ok(());
            }
        }

        // Create the index if it doesn't exist
        let create_query = format!(
            "CALL db.index.vector.createNodeIndex(
                '{}',
                'MessageNode',
                'embedding',
                1536,
                'cosine'
            )",
            index_name
        );
        let mut result = graph.execute(query(&create_query)).await?;
        while let Ok(Some(row)) = result.next().await {
            let name: String = row.get("name")?;
            if name == index_name {
                println!("Index {} created successfully", index_name);
            }
        }
        Ok(())
    }
    async fn connect(&self) -> Result<Graph, Error> {
        let config = ConfigBuilder::new()
            .uri(self.uri.clone())
            .user(self.user.clone())
            .password(self.pass.clone())
            .build()?;

        let graph = Graph::connect(config).await?;
        self.init_vector_index(&graph, "messageEmbeddings").await?;
        Ok(graph)
    }
}

impl MessageRepository for Neo4jMessageRepository {
    async fn save_message_node(&self, message_node: &MessageNode) -> Result<(), Error> {
        // Skip saving system messages
        if message_node.role.eq_ignore_ascii_case("system") {
            return Ok(());
        }

        let graph = self.connect().await?;
        let q = query(
            r#"CREATE (m:MessageNode {
                trace_id: $trace_id, 
                content: $content, 
                role: $role, 
                timestamp: $timestamp, 
                partition: $partition,
                instance: $instance,
                embedding: $embedding
            }) RETURN m"#,
        )
        .param("trace_id", message_node.trace_id.clone())
        .param("content", message_node.content.clone())
        .param("timestamp", message_node.timestamp.clone())
        .param("role", message_node.role.clone())
        .param("partition", message_node.partition.clone())
        .param("instance", message_node.instance.clone())
        .param("embedding", message_node.embedding.clone());

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

    async fn find_similar_messages(
        &self,
        embedding: Vec<f32>,
        trace_id: &str,
        partition: &str,
        instance: &str,
        top_k: i64,
    ) -> Result<Vec<MessageNode>, Error> {
        let graph = self.connect().await?;

        let query_text = "
            CALL db.index.vector.queryNodes(
                'messageEmbeddings',
                $topK,
                $embedding
            ) YIELD node, score
            WITH node, score
            WHERE node.partition = $partition
              AND node.instance = $instance
            RETURN node.trace_id AS trace_id,
                   node.partition AS partition,
                   node.instance AS instance,
                   node.role AS role,
                   node.content AS content,
                   node.embedding AS embedding,
                   node.url AS url,
                   node.timestamp AS timestamp
            ORDER BY score DESC
        ";

        let mut result = graph
            .execute(
                query(query_text)
                    .param("embedding", embedding)
                    .param("topK", top_k)
                    .param("traceId", trace_id)
                    .param("partition", partition)
                    .param("instance", instance),
            )
            .await?;

        let mut messages = Vec::new();
        while let Ok(Some(row)) = result.next().await {
            messages.push(MessageNode {
                trace_id: row.get("trace_id")?,
                partition: row.get("partition")?,
                instance: row.get("instance")?,
                role: row.get("role")?,
                content: row.get("content")?,
                embedding: row.get("embedding")?,
                url: row.get("url")?,
                timestamp: row.get("timestamp")?,
            });
        }

        Ok(messages)
    }

    async fn get_last_messages_for_partition_and_instance(
        &self,
        partition: String,
        instance: String,
        count: i32,
    ) -> Result<Vec<MessageNode>, Error> {
        let graph = self.connect().await?;
        let q = format!(
            "MATCH (m:MessageNode {{partition: '{}', instance: '{}'}}) RETURN m ORDER BY m.timestamp DESC LIMIT {}",
            partition, instance, count
        );
        let mut result = graph.execute(query(q.as_str())).await?;
        let mut messages = Vec::new();

        while let Some(row) = result.next().await? {
            let node: MessageNode = row.get("m")?;
            messages.push(node);
        }

        Ok(messages)
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

    #[tokio::test]
    async fn test_find_similar_messages() {
        let repo = Neo4jMessageRepository::new(
            "bolt://localhost:7687".to_string(),
            "neo4j".to_string(),
            "password".to_string(),
        );

        //insert some test messages
        let message_node = MessageNode {
            embedding: vec![0.0; 1536],
            trace_id: "12345".to_string(),
            partition: "default".to_string(),
            instance: "default".to_string(),
            role: "user".to_string(),
            content: Some("Hello, world!".to_string()),
            url: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };
        let no_match_node = MessageNode {
            embedding: vec![0.0; 1234],
            trace_id: "12346".to_string(),
            partition: "default".to_string(),
            instance: "default".to_string(),
            role: "user".to_string(),
            content: Some("Hello, world!".to_string()),
            url: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };
        repo.save_message_node(&message_node).await.unwrap();
        repo.save_message_node(&no_match_node).await.unwrap();

        let embedding = vec![0.0; 1536];
        let partition = "default".to_string();
        let instance = "default".to_string();
        let result = repo
            .find_similar_messages(embedding, "12345", partition.as_str(), instance.as_str(), 5)
            .await;
        if result.is_err() {
            println!("Error finding similar messages: {:?}", result);
        }
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len() > 0, true);
    }
}
