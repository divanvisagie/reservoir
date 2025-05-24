use anyhow::Error;

use crate::{
    args::ReplaySubCommand,
    clients::embedding::{get_embeddings_for_txt, EmbeddingClient},
    services::ChatRequestService,
};

pub async fn execute<'a>(service: &'a ChatRequestService<'a>, model: &str) -> Result<(), Error> {
    let messages = service.get_messages_for_partition("default").await?;

    // Spawn tasks for each message
    for message in messages {
        let ec: EmbeddingClient = EmbeddingClient::with_fastembed(model);
        println!("message id : {:?}", message.id);

        match message.content {
            Some(content) => match get_embeddings_for_txt(content.as_str(), ec).await {
                Ok(_embeddings) => {
                    // println!("Embeddings: {:?}", embeddings)
                }
                Err(e) => eprintln!("Error fetching embeddings: {}", e),
            },
            None => {
                println!(
                    "No content found for message with trace ID: {}",
                    message.trace_id
                );
            }
        }
    }

    Ok(())
}

pub async fn run<'a>(
    service: &'a ChatRequestService<'a>,
    replay_sub_command: &ReplaySubCommand,
) -> Result<(), Error> {
    let model = "bge-large-en-v15";
    execute(service, model).await
}
