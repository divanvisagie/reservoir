use std::path::PathBuf;

use super::openai::embeddings::get_embeddings_for_text as openai_get_embeddings_for_text;
use anyhow::Error;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use tracing::info;

#[derive(Clone, Debug)]
pub enum EmbeddingClient {
    OpenAI { model: String, length: i32 },
    FastEmbed { model: String, length: i32 },
}

#[allow(dead_code)]
impl EmbeddingClient {
    pub fn new_openai(model: String) -> Self {
        EmbeddingClient::OpenAI {
            model,
            length: 1536,
        }
    }

    pub fn with_fastembed(model: &str) -> Self {
        EmbeddingClient::FastEmbed {
            model: model.to_string(),
            length: 1024,
        }
    }

    pub fn default() -> Self {
        let model = "text-embedding-ada-002".to_string();
        EmbeddingClient::OpenAI {
            model,
            length: 1536,
        }
    }

    pub fn get_node_name(&self) -> String {
        match self {
            EmbeddingClient::OpenAI { model, .. } => format!("Embedding1536"),
            EmbeddingClient::FastEmbed { model, .. } => format!("Embedding1024"),
        }
    }

    pub fn get_index_name(&self) -> String {
        match self {
            EmbeddingClient::OpenAI { model, .. } => format!("embedding1536"),
            EmbeddingClient::FastEmbed { model, .. } => format!("embedding1024"),
        }
    }
}

pub fn get_cache_path() -> PathBuf {
    let tmp_dir = dirs_next::data_dir().unwrap();
    tmp_dir.join("reservoir").join("models")
}

pub async fn get_embeddings_for_txt(
    text: &str,
    client: EmbeddingClient,
) -> Result<Vec<f32>, Error> {
    match client {
        EmbeddingClient::OpenAI { model, length } => {
            let result = openai_get_embeddings_for_text(text).await;
            match result {
                Ok(embeddings) => {
                    if embeddings.is_empty() {
                        Err(Error::msg("No embeddings found"))
                    } else {
                        Ok(embeddings[0].embedding.clone())
                    }
                }
                Err(e) => Err(e),
            }
        }
        EmbeddingClient::FastEmbed { model, length } => {
            info!("Using FastEmbed for embedding");
            let init_options = InitOptions::new(EmbeddingModel::BGELargeENV15)
                .with_show_download_progress(true)
                .with_cache_dir(get_cache_path());

            let model = TextEmbedding::try_new(init_options);
            let texts = vec![text];
            let model = model?;
            let embeddings = model.embed(texts, None)?;

            if let Some(embedding) = embeddings.first() {
                Ok(embedding.clone())
            } else {
                Err(Error::msg("No embeddings found"))
            }
        }
    }
}
