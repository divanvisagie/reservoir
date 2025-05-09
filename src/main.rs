use anyhow::Error;
use handler::completions::handle_with_partition;
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use repos::message::{Neo4jMessageRepository, MessageRepository};
use repos::config::get_reservoir_port;
use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use args::{Args, SubCommands};
use clap::Parser;
use serde_json;
use std::fs;
use tracing::{info, warn, error};

mod clients;
mod handler;
mod models;
mod repos;
mod args;

fn get_partition_from_path(path: &str) -> String {
    path.strip_prefix("/v1/partition/")
        .and_then(|rest| rest.split('/').next())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "default".to_string())
}

fn get_instance_from_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.strip_prefix("/v1/partition/")?.split('/').collect();
    if parts.len() >= 3 && parts[1] == "instance" {
        Some(parts[2].to_string())
    } else {
        None
    }
}

fn is_chat_request(path: &str) -> bool {
    path.contains("/chat/completions") || path.starts_with("/v1/partition/")
}

async fn handle(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    info!("Received request: {} {}", req.method(), req.uri().path());

    match (req.method(), req.uri().path()) {
        (&Method::POST, path) if path.starts_with("/v1/") => {
            if !is_chat_request(path) {
                let mut not_found = Response::new(Full::new(Bytes::from("Not Found")));
                *not_found.status_mut() = StatusCode::NOT_FOUND;
                return Ok(not_found);
            }
            let partition = get_partition_from_path(path);
            info!("Partition: {}", partition);
            let instance = get_instance_from_path(path).unwrap_or(partition.clone());
            info!("Instance: {}", instance);

            let whole_body = req.into_body().collect().await.unwrap().to_bytes();
            let response_bytes =
                handle_with_partition(partition.as_str(), instance.as_str(), whole_body).await;
            let response_bytes = match response_bytes {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("Error handling request: {}", e);
                    return Ok(Response::new(Full::new(Bytes::from(
                        "Internal Server Error",
                    ))));
                }
            };
            Ok(Response::new(Full::new(response_bytes)))
        }

        (&Method::POST, "/echo") => {
            let whole_body = req.into_body().collect().await.unwrap().to_bytes();
            let body = String::from_utf8_lossy(&whole_body);
            Ok(Response::new(Full::new(Bytes::from(format!(
                "You said: {}",
                body
            )))))
        }

        _ => {
            let mut not_found = Response::new(Full::new(Bytes::from("Not Found")));
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

async fn start_server() -> Result<(), Error> {
    let port = get_reservoir_port();

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(handle))
                .await
            {
                error!("Error serving connection: {:?}", err);
            }
        });
    }

}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "reservoir=info".to_string())
        )
        .init();
    let args = Args::parse();
    let repo = Neo4jMessageRepository::default();
    match args.subcmd {
        Some(SubCommands::Start(_)) => {
            repo.init_vector_index().await?;
            start_server().await?;
        }
        Some(SubCommands::Config(_config_subcmd)) => {
            info!("Not yet supported");
        }
        Some(SubCommands::Export) => {
            // Export all message nodes as JSON
            let messages = repo.get_messages_for_partition(None).await?;
            let json = serde_json::to_string_pretty(&messages)?;
            println!("{}", json);
        }
        Some(SubCommands::Import(import_cmd)) => {
            // Import message nodes from a JSON file
            let file_content = fs::read_to_string(&import_cmd.file)?;
            let messages: Vec<models::message_node::MessageNode> = serde_json::from_str(&file_content)?;
            for message in &messages {
                repo.save_message_node(message).await?;
            }
            println!("Imported {} message nodes from {}", messages.len(), import_cmd.file);
        }
        None => {
        }
    };
    Ok(())
}
