use std::path::PathBuf;

use super::openai::embeddings::get_embeddings_for_text as openai_get_embeddings_for_text;
use anyhow::Error;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use tracing::info;

pub enum EmbeddingClient {
    OpenAI(String),
    FastEmbed(String),
}

#[allow(dead_code)]
impl EmbeddingClient {
    pub fn new_openai(model: String) -> Self {
        EmbeddingClient::OpenAI(model)
    }

    pub fn with_fastembed(model: &str) -> Self {
        EmbeddingClient::FastEmbed(model.to_string())
    }

    pub fn default() -> Self {
        EmbeddingClient::OpenAI("text-embedding-ada-002".to_string())
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
        EmbeddingClient::OpenAI(_model) => {
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
        EmbeddingClient::FastEmbed(_model) => {
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
