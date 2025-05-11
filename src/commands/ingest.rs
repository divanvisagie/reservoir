use crate::repos::message::{Neo4jMessageRepository, MessageRepository};
use crate::models::message_node::MessageNode;
use crate::clients::openai::embeddings::get_embeddings_for_text;
use crate::clients::openai::types::Message;
use anyhow::Error;
use uuid::Uuid;
use std::io::{self, Read};
use crate::args::IngestSubCommand;

pub async fn run(repo: &Neo4jMessageRepository, cmd: &IngestSubCommand) -> Result<(), Error> {
    // Read stdin
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    let content = buffer.trim().to_string();
    if content.is_empty() {
        println!("No input provided on stdin");
        return Ok(());
    }
    let partition = cmd.partition.clone().unwrap_or_else(|| "default".to_string());
    let instance = cmd.instance.clone().unwrap_or_else(|| partition.clone());
    let trace_id = Uuid::new_v4().to_string();
    let message = Message {
        role: "user".to_string(),
        content: content.clone(),
    };
    let embedding = get_embeddings_for_text(&content).await?.first().unwrap().embedding.clone();
    let node = MessageNode::from_message(&message, &trace_id, &partition, &instance, embedding);
    repo.save_message_node(&node).await?;
    println!("Saved message with trace_id: {}", trace_id);
    Ok(())
} 