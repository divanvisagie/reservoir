use clap::Parser;
use crate::repos::message::{MessageRepository, Neo4jMessageRepository};
use anyhow::Error;
use crate::clients::embeddings::get_embeddings_for_text;
use crate::models::message_node::MessageNode;

#[derive(Parser, Debug)]
#[command(author, version, about = "Search messages by keyword or semantic similarity", long_about = None)]
pub struct SearchSubCommand {
    /// The search term (keyword or semantic)
    pub term: String,
    /// Use semantic search instead of keyword search
    #[arg(long)]
    pub semantic: bool,
    /// Partition to search (defaults to "default")
    #[arg(short, long)]
    pub partition: Option<String>,
    /// Instance to search (defaults to partition)
    #[arg(short, long)]
    pub instance: Option<String>,
}

pub async fn run(repo: &Neo4jMessageRepository, cmd: &SearchSubCommand) -> Result<(), Error> {
    if cmd.semantic {
        let partition = cmd.partition.clone().unwrap_or_else(|| "default".to_string());
        let instance = cmd.instance.clone().unwrap_or_else(|| partition.clone());
        // Semantic search: get embedding for the term, then use find_similar_messages
        let embeddings = get_embeddings_for_text(&cmd.term).await?;
        let embedding = embeddings.first().map(|e| e.embedding.clone()).unwrap_or_default();
        let results = repo.find_similar_messages(
            embedding,
            "search-trace-id",
            &partition,
            &instance,
            10,
        ).await?;
        for (i, msg) in results.iter().enumerate() {
            // If MessageNode had a score, print it; otherwise, just print the message
            // Here, we don't have score, so just print index as a placeholder
            println!("{}. [{}] {}: {}", i + 1, msg.trace_id, msg.role, msg.content.as_deref().unwrap_or(""));
        }
    } else {
        // Keyword search: fetch all messages and filter by keyword
        let messages = repo.get_messages_for_partition(None).await?;
        let filtered: Vec<&MessageNode> = messages.iter()
            .filter(|m| m.content.as_deref().unwrap_or("").to_lowercase().contains(&cmd.term.to_lowercase()))
            .collect();
        for msg in filtered {
            println!("[{}] {}: {}", msg.trace_id, msg.role, msg.content.as_deref().unwrap_or(""));
        }
    }
    Ok(())
} 