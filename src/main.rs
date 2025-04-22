use hyper::service::service_fn;
use hyper::{Request, Response, Method, StatusCode};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use models::ChatResponse;
use tokio::net::TcpListener;
use std::convert::Infallible;
use http_body_util::Full;
use hyper::body::Incoming;
use http_body_util::BodyExt;
use std::net::SocketAddr;

mod models;

async fn handle(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    println!("Received request: {} {}", req.method(), req.uri().path());
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/v1/chat/completions") => {
            let cr = ChatResponse {
                id: "chatcmpl-123".to_string(),
                object: "chat.completion".to_string(),
                created: 1677858242,
                model: "gpt-3.5-turbo".to_string(),
                usage: models::Usage {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                },
                choices: vec![models::Choice {
                    message: models::Message {
                        role: "assistant".to_string(),
                        content: "Hello, World!".to_string(),
                    },
                    finish_reason: "stop".to_string(),
                    index: 0,
                }],
            };
            let str = serde_json::to_string(&cr).unwrap();
            println!("Response: {}", str);
            Ok(Response::new(Full::new(Bytes::from(str))))
        }
        (&Method::POST, "/echo") => {
            // Use collect() from BodyExt instead of to_bytes
            let whole_body = req.into_body().collect().await.unwrap().to_bytes();
            let body = String::from_utf8_lossy(&whole_body);
            Ok(Response::new(Full::new(Bytes::from(format!("You said: {}", body)))))
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
