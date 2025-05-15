use anyhow::Error;
use args::{Args, SubCommands};
use clap::Parser;
use commands::search::execute as search_execute;
use commands::view::execute;
use handler::completions::handle_with_partition;
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use repos::message::AnyMessageRepository;
use repos::message::Neo4jMessageRepository;
use std::convert::Infallible;
use tracing::{error, info};
use std::fs;
use std::path::Path;

mod args;
mod clients;
mod commands;
mod handler;
mod models;
mod repos;
mod services;
mod utils;

fn get_partition_from_path(path: &str) -> String {
    path.strip_prefix("/partition/")
        .and_then(|rest| rest.split('/').next())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "default".to_string())
}

fn get_instance_from_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.strip_prefix("/partition/")?.split('/').collect();
    if parts.len() >= 3 && parts[1] == "instance" {
        Some(parts[2].to_string())
    } else {
        None
    }
}

fn is_chat_request(path: &str) -> bool {
    path.contains("/chat/completions")
}

fn is_search_request(path: &str) -> bool {
    path.contains("/command/search")
}

fn is_view_request(path: &str) -> bool {
    path.contains("/command/view")
}

async fn handle(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    info!("Received request: {} {}", req.method(), req.uri().path());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/api/tags") => {
            let body = include_str!("static/ollama_tags.json");
            let response = Response::builder()
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(body)))
                .unwrap();
            Ok(response)
        }

        (&Method::POST, path) if is_chat_request(path) => {
            info!("Chat request: {}", path);
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

        (&Method::GET, path) if is_search_request(path) => {
            info!("Search request: {}", path);
            let partition = get_partition_from_path(path);
            info!("Partition: {}", partition);
            let instance = get_instance_from_path(path).unwrap_or(partition.clone());
            info!("Instance: {}", instance);

            // Extract count from the path (last segment)
            let count = path
                .split('/')
                .last()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(5) as usize;

            // Parse query parameters for term and semantic
            let query = req.uri().query().unwrap_or("");
            let mut term = "".to_string();
            let mut semantic = false;
            for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
                if key == "term" {
                    term = value.into_owned();
                } else if key == "semantic" {
                    semantic = value == "true" || value == "1";
                }
            }

            if term.is_empty() {
                let response =
                    Response::new(Full::new(Bytes::from("Missing 'term' query parameter")));
                return Ok(response);
            }

            let repo = AnyMessageRepository::new_neo4j();
            let result = search_execute(
                &repo, partition, instance, count, term, semantic, false, false,
            )
            .await;
            match result {
                Ok(output) => {
                    let json = serde_json::to_string(&output).unwrap();
                    let response = Response::new(Full::new(Bytes::from(json)));
                    Ok(response)
                }
                Err(e) => {
                    error!("Error executing search: {}", e);
                    let response = Response::new(Full::new(Bytes::from(format!("Error: {}", e))));
                    Ok(response)
                }
            }
        }

        (&Method::GET, path) if is_view_request(path) => {
            let partition = get_partition_from_path(path);
            info!("Partition: {}", partition);
            let instance = get_instance_from_path(path).unwrap_or(partition.clone());
            info!("Instance: {}", instance);

            // the last part of the path should be the number, lets get it
            let count = path
                .split('/')
                .last()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(5);
            // convert to usize
            let count = count as usize;

            let repo = AnyMessageRepository::new_neo4j();

            let result = execute(&repo, partition, instance, count).await;

            match result {
                Ok(output) => {
                    let json = serde_json::to_string(&output).unwrap();
                    let response = Response::new(Full::new(Bytes::from(json)));
                    Ok(response)
                }
                Err(e) => {
                    error!("Error executing command: {}", e);
                    let response = Response::new(Full::new(Bytes::from(format!("Error: {}", e))));
                    Ok(response)
                }
            }
        }

        (&Method::POST, "/api/show") => {
            let body = include_str!("static/ollama_show.json");
            let response = Response::builder()
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(body)))
                .unwrap();
            Ok(response)
        }

        _ => {
            let mut not_found = Response::new(Full::new(Bytes::from("Not Found")));
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "reservoir=info".to_string()))
        .init();
    let args = Args::parse();
    let repo = AnyMessageRepository::new_neo4j();
    match args.subcmd {
        Some(SubCommands::Start(ref start_cmd)) => {
            commands::start::run(&repo, start_cmd.ollama).await?;
        }
        Some(SubCommands::Config(_config_subcmd)) => {
            commands::config::run().await?;
        }
        Some(SubCommands::Export) => {
            commands::export::run(&repo).await?;
        }
        Some(SubCommands::Import(import_cmd)) => {
            commands::import::run(&repo, &import_cmd.file).await?;
        }
        Some(SubCommands::View(ref view_cmd)) => {
            commands::view::run(&repo, view_cmd).await?;
        }
        Some(SubCommands::Search(ref search_cmd)) => {
            commands::search::run(&repo, search_cmd).await?;
        }
        Some(SubCommands::Ingest(ref ingest_cmd)) => {
            commands::ingest::run(&repo, ingest_cmd).await?;
        }
        None => {}
    };
    Ok(())
}
