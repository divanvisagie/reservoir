use crate::args::ViewSubCommand;
use crate::clients::openai::types::Message;
use crate::repos::message::{MessageRepository, Neo4jMessageRepository};
use anyhow::Error;
use tracing::{info, error};

pub async fn execute(
    repo: &Neo4jMessageRepository,
    partition: String,
    instance: String,
    count: usize,
) -> Result<Vec<Message>, Error> {
    let mut messages = repo
        .get_last_messages_for_partition_and_instance(partition, instance, count)
        .await?;
    messages.sort_by(|a, b| {
        let a_time = a.timestamp;
        let b_time = b.timestamp;
        a_time.cmp(&b_time)
    });

    let messages: Vec<Message> = messages.iter().map(|m| {
        m.to_message()
    }).collect();
    Ok(messages)
}

pub async fn run(repo: &Neo4jMessageRepository, view_cmd: &ViewSubCommand) -> Result<(), Error> {
    let partition = view_cmd
        .partition
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let instance = view_cmd
        .instance
        .clone()
        .unwrap_or_else(|| partition.clone());

    match execute(repo, partition, instance, view_cmd.count).await {
        Ok(output) => {
            // pretty print
            for message in output {
                info!("{}: - {}", message.role, message.content);
            }
            Ok(())
        }
        Err(e) => {
            error!("Error executing command: {:?}", e);
            Err(e)
        }
    }
}
