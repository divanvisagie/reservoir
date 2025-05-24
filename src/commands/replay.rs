use anyhow::Error;
use tracing::info;

use crate::{
    args::ReplaySubCommand,
    clients::embedding::{get_embeddings_for_txt, EmbeddingClient},
    services::ChatRequestService,
};

pub async fn execute<'a>(service: &'a ChatRequestService<'a>, model: &str) -> Result<(), Error> {
    let messages = service.get_messages().await?;
    info!("Found {} messages to process", messages.len());

    // Spawn tasks for each message
    for message in messages {
        let ec: EmbeddingClient = EmbeddingClient::with_fastembed(model);
        println!("message id : {:?}", message.id);

        match message.content.clone() {
            Some(content) => match get_embeddings_for_txt(content.as_str(), ec.clone()).await {
                Ok(embeddings) => {
                    info!("attaching to message: {:?}", message.id);
                    let r = service
                        .attach_embedding_to_message(&message, embeddings, &ec, model)
                        .await;
                    match r {
                        Ok(_) => {
                            println!(
                                "Successfully attached embeddings to message with trace ID: {}",
                                message.trace_id
                            );
                        }
                        Err(e) => {
                            eprintln!(
                                "Failed to attach embeddings to message with trace ID: {}. Error: {}",
                                message.trace_id, e
                            );
                        }
                    }
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
