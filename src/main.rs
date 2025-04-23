use http::header;
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use models::message_node::MessageNode;
use models::ChatRequest;
use repos::message::MessageRepository;
use repos::message::Neo4jMessageRepository;
use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use uuid::Uuid;

mod models;
mod repos;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

async fn handle(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    println!("Received request: {} {}", req.method(), req.uri().path());
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

    match (req.method(), req.uri().path()) {
        (&Method::POST, "/v1/chat/completions") => {
            let whole_body = req.into_body().collect().await.unwrap().to_bytes();

            let json_string = String::from_utf8_lossy(&whole_body).to_string();
            let chat_request_model =
                ChatRequest::from_json(json_string.as_str()).expect("Valid JSON");
            let trace_id = Uuid::new_v4().to_string();
            let partition = Some("test_partition".to_string());
            let repo = Neo4jMessageRepository::new(
                "bolt://localhost:7687".to_string(),
                "neo4j".to_string(),
                "password".to_string(),
            );
            for message in &chat_request_model.messages {
                let node = MessageNode::from_message(message, trace_id.as_str(), partition.clone());
                let _save_result = repo.save_message_node(&node).await;
            }

            // forward the request with reqwest
            let client = reqwest::Client::new();
            let response = client
                .post(OPENAI_API_URL)
                .header("Content-Type", "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
                .body(whole_body.to_vec())
                .send()
                .await
                .map_err(|e| {
                    eprintln!("Error sending request to OpenAI: {}", e);
                    e
                });

            let response = match response {
                Ok(response) => response.text().await.unwrap(),
                Err(e) => {
                    if e.is_timeout() {
                        eprintln!("The request timed out.");
                    } else if e.is_connect() {
                        eprintln!("Failed to connect to the server: {}", e);
                    } else if e.is_status() {
                        if let Some(status) = e.status() {
                            eprintln!("Received HTTP status code: {}", status);
                        }
                    }

                    if let Some(url) = e.url() {
                        eprintln!("URL: {}", url);
                    }
                    "".to_string()
                }
            };

            println!("Response from OpenAI: {:?}", response);

            Ok(Response::new(Full::new(Bytes::from(response))))
        }
        (&Method::POST, "/echo") => {
            // Use collect() from BodyExt instead of to_bytes
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create a proper SocketAddr
    let addr = SocketAddr::from(([127, 0, 0, 1], 3017));
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            // Use the hyper_util service_fn
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(handle))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
