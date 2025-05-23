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
    
    async fn get_embedding_node(&self, id: &str) -> Result<EmbeddingNode, Error>;
}

pub enum AnyEmbeddingRepository {
    Neo4j(Neo4jEmbeddingRepository),
}

impl AnyEmbeddingRepository {
    pub fn new_neo4j(uri: String, user: String, pass: String) -> Self {
        AnyEmbeddingRepository::Neo4j(Neo4jEmbeddingRepository::new(uri, user, pass))
    }
    
    pub fn clone_from_neo4j(repo: &Neo4jEmbeddingRepository) -> Self {
        AnyEmbeddingRepository::Neo4j(repo.clone())
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
    
    async fn get_embedding_node(&self, id: &str) -> Result<EmbeddingNode, Error> {
        match self {
            AnyEmbeddingRepository::Neo4j(repo) => {
                repo.get_embedding_node(id).await
            }
        }
    }
}

pub struct Neo4jEmbeddingRepository {
    uri: String,
    user: String,
    pass: String,
}

impl Clone for Neo4jEmbeddingRepository {
    fn clone(&self) -> Self {
        Neo4jEmbeddingRepository {
            uri: self.uri.clone(),
            user: self.user.clone(),
            pass: self.pass.clone(),
        }
    }
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
    
    async fn get_embedding_node(&self, id: &str) -> Result<EmbeddingNode, Error> {
        let graph = self.connect().await?;
        let q = query(
            r#"
            MATCH (e:Embedding) 
            WHERE id(e) = toInteger($id)
            RETURN id(e) AS id, e.model AS model, e.embedding AS embedding, 
                   e.partition AS partition, e.instance AS instance
            "#
        )
        .param("id", id);

        let mut result = graph.execute(q).await?;
        
        if let Some(row) = result.next().await? {
            let id = row.get::<i64>("id")?.to_string();
            let model = row.get::<String>("model")?;
            let embedding = row.get::<Vec<f32>>("embedding")?;
            let partition = row.get::<String>("partition").ok();
            let instance = row.get::<String>("instance").ok();
            
            Ok(EmbeddingNode {
                id: Some(id),
                model,
                embedding,
                partition,
                instance,
            })
        } else {
            Err(Error::msg(format!("No embedding found with id {}", id)))
        }
    }
}

impl EmbeddingRepository for Neo4jEmbeddingRepository {
    async fn get_embedding_node(&self, id: &str) -> Result<EmbeddingNode, Error> {
        // Call the instance method with the same name
        Neo4jEmbeddingRepository::get_embedding_node(self, id).await
    }
    
    async fn find_similar_embeddings(
        &self,
        embedding: Vec<f32>,
        partition: &str,
        instance: &str,
        top_k: usize,
    ) -> Result<Vec<EmbeddingNode>, Error> {
        let graph = self.connect().await?;
        let q = query(
            &format!(
                r#"
                MATCH (e:Embedding) 
                WHERE e.partition = $partition AND e.instance = $instance
                WITH e, algo.similarity.cosine(e.embedding, $embedding) 
                AS similarity 
                RETURN id(e) AS id, e.model AS model, e.embedding AS embedding, 
                       e.partition AS partition, e.instance AS instance, similarity
                ORDER BY similarity DESC LIMIT {}
                "#,
                top_k
            )
        )
        .param("embedding", embedding)
        .param("partition", partition)
        .param("instance", instance);

        let mut result = graph.execute(q).await?;

        let mut similar_embeddings = Vec::new();
        while let Some(row) = result.next().await? {
            let id = row.get::<i64>("id")?.to_string();
            let model = row.get::<String>("model")?;
            let embedding_vec = row.get::<Vec<f32>>("embedding")?;
            let similarity = row.get::<f64>("similarity")?;
            
            let node = EmbeddingNode {
                id: Some(id),
                model,
                embedding: embedding_vec,
                partition: Some(partition.to_string()),
                instance: Some(instance.to_string()),
            };
            
            similar_embeddings.push(node);
        }

        Ok(similar_embeddings)
    }
}
