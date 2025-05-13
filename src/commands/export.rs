use crate::repos::message::{AnyMessageRepository, MessageRepository};
use anyhow::Error;
use serde_json;

pub async fn run(repo: &AnyMessageRepository) -> Result<(), Error> {
    let messages = repo.get_messages_for_partition(None).await?;
    let json = serde_json::to_string_pretty(&messages)?;
    println!("{}", json);
    Ok(())
}
