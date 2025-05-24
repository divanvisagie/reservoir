use anyhow::Error;
use neo4rs::{query, ConfigBuilder, Graph};
use tracing::{error, info};

use crate::{
    models::message_node::MessageNode,
    repos::config::{get_neo4j_password, get_neo4j_uri, get_neo4j_user},
};

use super::MessageRepository;

pub struct Neo4jMessageRepository {
    pub uri: String,
    pub user: String,
    pub pass: String,
}

impl Neo4jMessageRepository {
    pub fn default() -> Self {
        let instance = Neo4jMessageRepository {
            uri: get_neo4j_uri(),
            user: get_neo4j_user(),
            pass: get_neo4j_password(),
        };
        instance.init_vector_index();
        instance
    }

    pub async fn init_vector_index(&self) -> Result<(), Error> {
        let index_name = "messageEmbeddings";
        let emneddings_index_name = "embeddingEmbeddings";
        let graph = self.connect().await?;
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
            );
            CALL db.index.vector.createNodeIndex(
                '{}',
                'Embedding',
                'embedding',
                1536,
                'cosine'
            )",
            index_name, emneddings_index_name
        );
        let result = graph.execute(query(&create_query)).await;
        match result {
            Ok(mut rows) => {
                while let Ok(Some(row)) = rows.next().await {
                    let name: String = row.get("name")?;
                    if name == index_name {
                        info!("Index {} created successfully", index_name);
                    }
                }
            }
            Err(e) => {
                // Check if it's the "equivalent index already exists" error and suppress it
                if format!("{:?}", e).contains("EquivalentSchemaRuleAlreadyExistsException") {
                    info!("Index '{}' already exists, skipping creation", index_name);
                } else {
                    Err(e)?;
                }
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
        let create_q = query(
            r#"
            CREATE (m:MessageNode {
                trace_id: $trace_id,
                content: $content,
                role: $role,
                timestamp: $timestamp,
                partition: $partition,
                instance: $instance,
                embedding: $embedding,
                url: $url
            })
            CREATE (e:Embedding {
                model: 'text-embedding-ada-002',
                embedding: $embedding,
                partition: $partition,
                instance: $instance
            })
            CREATE (m)-[:HAS_EMBEDDING]->(e)
            RETURN id(m) AS nodeId, id(e) AS embeddingId
            "#,
        )
        .param("trace_id", message_node.trace_id.clone())
        .param("content", message_node.content.clone())
        .param("timestamp", message_node.timestamp.clone())
        .param("role", message_node.role.clone())
        .param("partition", message_node.partition.clone())
        .param("instance", message_node.instance.clone())
        .param("embedding", message_node.embedding.clone())
        .param("url", message_node.url.clone());

        // Execute the CREATE query
        let mut create_result = graph.execute(create_q).await?;
        // Consume the result to ensure the node is created before potentially linking it
        let _ = create_result.next().await?;

        // If the saved message is an assistant message, try to link it to the corresponding user message
        if message_node.role.eq_ignore_ascii_case("assistant") {
            let link_q = query(
                r#"MATCH (u:MessageNode {role: 'user', trace_id: $trace_id})
                   MATCH (a:MessageNode {role: 'assistant', trace_id: $trace_id})
                   MERGE (u)-[:RESPONDED_WITH]->(a)
                   RETURN count(*)"#,
            )
            .param("trace_id", message_node.trace_id.clone());

            // Execute the MERGE query
            let mut link_result = graph.execute(link_q).await?;
            // Consume the result
            let _ = link_result.next().await?;
        }

        Ok(())
    }

    async fn find_similar_messages(
        &self,
        embedding: Vec<f32>,
        trace_id: &str,
        partition: &str,
        instance: &str,
        top_k: usize,
    ) -> Result<Vec<MessageNode>, Error> {
        let graph = self.connect().await?;
        let top_k_extended = (top_k * 3) as i64;
        let query_text = "
        CALL db.index.vector.queryNodes(
            'messageEmbeddings',
            $topKExtended,
            $embedding
        ) YIELD node, score
        WITH node, score
        WHERE node.partition = $partition
          AND node.instance = $instance
          AND node.role = $role
        RETURN node.trace_id AS trace_id,
               node.partition AS partition,
               node.instance AS instance,
               node.role AS role,
               node.content AS content,
               node.embedding AS embedding,
               node.url AS url,
               node.timestamp AS timestamp,
               score
        ORDER BY score DESC
    ";
        let mut result = graph
            .execute(
                query(query_text)
                    .param("embedding", embedding)
                    .param("topKExtended", top_k_extended)
                    .param("traceId", trace_id)
                    .param("partition", partition)
                    .param("instance", instance)
                    .param("role", "user"),
            )
            .await?;
        let mut messages: Vec<(MessageNode, f64)> = Vec::new();
        while let Ok(Some(row)) = result.next().await {
            let message = MessageNode {
                id: None,
                trace_id: row.get("trace_id")?,
                partition: row.get("partition")?,
                instance: row.get("instance")?,
                role: row.get("role")?,
                content: row.get("content")?,
                embedding: row.get("embedding")?,
                url: row.get("url")?,
                timestamp: row.get("timestamp")?,
            };
            let score: f64 = row.get("score")?;
            messages.push((message, score));
        }
        messages.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let messages: Vec<MessageNode> = messages
            .into_iter()
            .take(top_k)
            .map(|(m, _score)| m)
            .collect();
        Ok(messages)
    }

    async fn get_message_node(&self, trace_id: &str) -> Result<MessageNode, Error> {
        let graph = self.connect().await?;
        let q = format!(
            "MATCH (m:MessageNode {{trace_id: '{}'}}) RETURN m",
            trace_id
        );
        let mut result = graph.execute(query(q.as_str())).await?;
        if let Some(row) = result.next().await? {
            let node: MessageNode = row.get("m")?;
            Ok(node)
        } else {
            Err(Error::msg("MessageNode not found"))
        }
    }

    async fn get_message_node_by_embedding_id(
        &self,
        embedding_id: &str,
    ) -> Result<MessageNode, Error> {
        let graph = self.connect().await?;

        // Query to find the MessageNode connected to an embedding with the given ID
        let q = query(
            r#"
            MATCH (m:MessageNode)-[:HAS_EMBEDDING]->(e:Embedding)
            WHERE id(e) = toInteger($embedding_id)
            RETURN m
            "#,
        )
        .param("embedding_id", embedding_id);

        let mut result = graph.execute(q).await?;

        match result.next().await? {
            Some(row) => {
                let node: MessageNode = row.get("m")?;
                Ok(node)
            }
            None => Err(Error::msg(format!(
                "No message found for embedding ID {}",
                embedding_id
            ))),
        }
    }

    async fn get_messages_for_partition(
        &self,
        partition: Option<&str>,
    ) -> Result<Vec<MessageNode>, Error> {
        let graph = self.connect().await?;
        let q = if let Some(p) = partition {
            query("MATCH (m:MessageNode {partition: $partition}) RETURN id(m) AS id, m")
                .param("partition", p)
        } else {
            query("MATCH (m:MessageNode) RETURN id(m) AS id, m")
        };

        let mut result = graph.execute(q).await?;
        let mut messages = Vec::new();

        while let Some(row) = result.next().await? {
            // First, extract the MessageNode
            let mut node: MessageNode = row.get("m")?;
            // Then, override its id field with the database id
            node.id = Some(row.get::<i64>("id")?);
            info!("Found message node: {:?}", node);
            messages.push(node);
        }

        Ok(messages)
    }

    async fn get_last_messages_for_partition_and_instance(
        &self,
        partition: String,
        instance: String,
        count: usize,
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

    async fn find_connections_between_nodes(
        &self,
        nodes: &[MessageNode],
    ) -> Result<Vec<MessageNode>, Error> {
        if nodes.is_empty() {
            return Ok(Vec::new());
        }

        let trace_ids: Vec<String> = nodes.iter().map(|n| n.trace_id.clone()).collect();

        let graph = self.connect().await?;
        // Query to find pairs of connected nodes within the input list,
        // then unwind the pairs and collect the distinct nodes involved.
        let query_text = r#"
            UNWIND $trace_ids AS traceId1
            UNWIND $trace_ids AS traceId2
            WITH traceId1, traceId2 // Introduce WITH clause
            WHERE traceId1 < traceId2 // Apply WHERE after WITH
            MATCH (n1:MessageNode {trace_id: traceId1})-[r:RESPONDED_WITH]-(n2:MessageNode {trace_id: traceId2})
            // Unwind the pair of nodes found
            WITH n1, n2 // Carry forward the matched nodes
            UNWIND [n1, n2] AS connected_node
            // Return distinct nodes involved in any connection
            RETURN DISTINCT connected_node
        "#;

        let mut result = graph
            .execute(query(query_text).param("trace_ids", trace_ids))
            .await?;

        let mut connected_nodes = Vec::new();
        while let Ok(Some(row)) = result.next().await {
            // Each row now contains one distinct connected node
            let node: MessageNode = row.get("connected_node")?;
            connected_nodes.push(node);
        }

        Ok(connected_nodes) // Return the vector of MessageNode
    }

    /// Finds nodes connected to a given node within a distance of 10 hops.
    /// Returns a vector of `MessageNode` instances representing the connected nodes.
    /// The distance is defined by the number of hops in the graph.
    async fn find_nodes_connected_to_node(
        &self,
        node: &MessageNode,
    ) -> Result<Vec<MessageNode>, Error> {
        let graph = self.connect().await?;
        let q = r#"
            MATCH p=(m:MessageNode {trace_id: $trace_id})-[:SYNAPSE*1..10]-(n:MessageNode)
            RETURN nodes(p) AS allNodes
        "#;
        let mut result = graph
            .execute(query(q).param("trace_id", node.trace_id.clone()))
            .await?;
        let mut connected_nodes = Vec::new();
        while let Ok(Some(row)) = result.next().await {
            let nodes: Vec<MessageNode> = row.get("allNodes")?;
            connected_nodes.extend(nodes);
        }
        Ok(connected_nodes)
    }

    async fn connect_synapses(&self) -> Result<(), Error> {
        let graph = self.connect().await?;
        let q = r#"
            MATCH (m:MessageNode)
            WHERE m.embedding IS NOT NULL AND size(m.embedding) = 1536
            WITH m
            ORDER BY m.timestamp ASC
            WITH collect(m) AS messages
            UNWIND range(0, size(messages) - 2) AS i
            WITH messages[i] AS m1, messages[i+1] AS m2
            WHERE m1.embedding IS NOT NULL AND m2.embedding IS NOT NULL AND size(m1.embedding) = 1536 AND size(m2.embedding) = 1536
            MERGE (m1)-[:SYNAPSE {score: vector.similarity.cosine(m1.embedding, m2.embedding)}]-(m2);
        "#;
        let mut result = graph.execute(query(q)).await?;
        while let Ok(Some(row)) = result.next().await {
            let node: MessageNode = row.get("m")?;
            info!("Connected nodes: {:?}", node);
        }
        let q = r#"
            MATCH (m1:MessageNode)-[r:SYNAPSE]->(m2:MessageNode)
            WHERE r.score < 0.85
            DELETE r
        "#;
        let mut result = graph.execute(query(q)).await?;
        while let Ok(Some(row)) = result.next().await {
            let node: MessageNode = row.get("m")?;
            error!("Deleted synapse: {:?}", node);
        }
        Ok(())
    }

    async fn get_messages_for_embedding_nodes(
        &self,
        embedding_nodes: Vec<i64>,
    ) -> Result<Vec<MessageNode>, Error> {
        let graph = self.connect().await?;
        let q = query(
            r#"
            MATCH (e:Embedding)-[:HAS_EMBEDDING]-(m:MessageNode)
            WHERE id(e) IN $embedding_nodes
            RETURN m
            "#,
        )
        .param("embedding_nodes", embedding_nodes);

        let mut result = graph.execute(q).await?;
        let mut messages = Vec::new();
        while let Some(row) = result.next().await? {
            let node: MessageNode = row.get("m")?;
            messages.push(node);
        }
        Ok(messages)
    }
}
