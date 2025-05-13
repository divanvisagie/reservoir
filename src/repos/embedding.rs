use anyhow::Error;
use neo4rs::{query, ConfigBuilder, Graph};

use crate::models::embedding_node::EmbeddingNode;

use super::config::{get_neo4j_password, get_neo4j_uri, get_neo4j_user};

pub trait EmbeddingRepository {
    async fn find_similar_embeddings(
        &self,
        embedding: Vec<f32>,
        partition: &str,
        instance: &str,
        top_k: usize,
    ) -> Result<Vec<EmbeddingNode>, Error>;
}

pub enum AnyEmbeddingRepository {
    Neo4j(Neo4jEmbeddingRepository),
}

impl AnyEmbeddingRepository {
    pub fn new_neo4j(uri: String, user: String, pass: String) -> Self {
        AnyEmbeddingRepository::Neo4j(Neo4jEmbeddingRepository::new(uri, user, pass))
    }
}

impl EmbeddingRepository for AnyEmbeddingRepository {
    async fn find_similar_embeddings(
        &self,
        embedding: Vec<f32>,
        partition: &str,
        instance: &str,
        top_k: usize,
    ) -> Result<Vec<EmbeddingNode>, Error> {
        match self {
            AnyEmbeddingRepository::Neo4j(repo) => {
                repo.find_similar_embeddings(embedding, partition, instance, top_k)
                    .await
            }
        }
    }
}

pub struct Neo4jEmbeddingRepository {
    uri: String,
    user: String,
    pass: String,
}

impl Neo4jEmbeddingRepository {
    pub fn new(uri: String, user: String, pass: String) -> Self {
        Neo4jEmbeddingRepository {
            uri: get_neo4j_uri(),
            user: get_neo4j_user(),
            pass: get_neo4j_password(),
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

impl EmbeddingRepository for Neo4jEmbeddingRepository {
    async fn find_similar_embeddings(
        &self,
        embedding: Vec<f32>,
        partition: &str,
        instance: &str,
        top_k: usize,
    ) -> Result<Vec<EmbeddingNode>, Error> {
        let graph = self.connect().await?;
        let q = query(
            r#"
            MATCH (e:EmbeddingNode) 
            WHERE e.partition = $partition AND e.instance = $instance
            WITH e, algo.similarity.cosine(e.embedding, $embedding) 
            AS similarity 
            RETURN e ORDER BY similarity DESC LIMIT {}
            "#,
        )
        .param("embedding", embedding)
        .param("partition", partition)
        .param("instance", instance);

        let mut result = graph.execute(q).await?;

        let mut similar_embeddings = Vec::new();
        while let Some(row) = result.next().await? {
            let node: EmbeddingNode = row.get("e")?;
            similar_embeddings.push(node);
        }

        Ok(similar_embeddings)
    }
}
