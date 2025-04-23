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
use anyhow::Error;

mod handler;
mod models;
mod repos;

async fn handle(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    println!("Received request: {} {}", req.method(), req.uri().path());

    match (req.method(), req.uri().path()) {
        (&Method::POST, path) if path.starts_with("/v1/chat/completions/") => {
            let partition = path.trim_start_matches("/v1/chat/completions/").to_string();

            let whole_body = req.into_body().collect().await.unwrap().to_bytes();
            let response_bytes = handle_with_partition(&partition, whole_body).await;
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
