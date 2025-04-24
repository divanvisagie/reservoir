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
use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod clients;
mod handler;
mod models;
mod repos;

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
    println!("Received request: {} {}", req.method(), req.uri().path());

    match (req.method(), req.uri().path()) {
        (&Method::POST, path) if path.starts_with("/v1/") => {
            if !is_chat_request(path) {
                let mut not_found = Response::new(Full::new(Bytes::from("Not Found")));
                *not_found.status_mut() = StatusCode::NOT_FOUND;
                return Ok(not_found);
            }
            let partition = get_partition_from_path(path);
            println!("Partition: {}", partition);
            let instance = get_instance_from_path(path);
            println!("Instance: {:?}", instance);

            let whole_body = req.into_body().collect().await.unwrap().to_bytes();
            let response_bytes = handle_with_partition(partition.as_str(), instance, whole_body).await;
            let response_bytes = match response_bytes {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("Error handling request: {}", e);
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

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Get port from environment variable or use default
    let port = env::var("RESERVOIR_PORT")
        .unwrap_or_else(|_| "3017".to_string())
        .parse::<u16>()
        .unwrap_or(3017);

    // Create a proper SocketAddr with configurable port
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
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
