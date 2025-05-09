use crate::repos::message::Neo4jMessageRepository;
use anyhow::Error;
use crate::repos::config::get_reservoir_port;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use std::net::SocketAddr;
use tracing::{info, error};

use crate::handle;

pub async fn start_server() -> Result<(), Error> {
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
pub async fn run(repo: &Neo4jMessageRepository) -> Result<(), Error> {
    repo.init_vector_index().await?;
    start_server().await
} 
