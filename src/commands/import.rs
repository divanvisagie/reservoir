use crate::models::message_node::MessageNode;
use crate::repos::message::{AnyMessageRepository, MessageRepository};
use anyhow::Error;
use serde_json;
use std::fs;

pub async fn run(repo: &AnyMessageRepository, file: &str) -> Result<(), Error> {
    let file_content = fs::read_to_string(file)?;
    let messages: Vec<MessageNode> = serde_json::from_str(&file_content)?;
    for message in &messages {
        repo.save_message_node(message).await?;
    }
    println!("Imported {} message nodes from {}", messages.len(), file);
    Ok(())
}

