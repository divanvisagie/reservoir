use crate::repos::message::{MessageRepository, Neo4jMessageRepository};
use anyhow::Error;
use std::fs;
use crate::models::message_node::MessageNode;
use serde_json;

pub async fn run(repo: &Neo4jMessageRepository, file: &str) -> Result<(), Error> {
    let file_content = fs::read_to_string(file)?;
    let messages: Vec<MessageNode> = serde_json::from_str(&file_content)?;
    for message in &messages {
        repo.save_message_node(message).await?;
    }
    println!("Imported {} message nodes from {}", messages.len(), file);
    Ok(())
} 