use crate::repos::config::get_reservoir_port;
use crate::repos::message::AnyMessageRepository;
use anyhow::Error;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::handle;

pub async fn start_server(port: u16) -> Result<(), Error> {
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

pub async fn run(_repo: &AnyMessageRepository, ollama: bool) -> Result<(), Error> {
    let port = if ollama { 11434 } else { get_reservoir_port() };
    start_server(port).await
}
