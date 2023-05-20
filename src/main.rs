use std::net::SocketAddr;

use clap::Parser;
use tokio::{
    io::{AsyncWrite, AsyncWriteExt, BufStream},
    net::{TcpListener, TcpStream},
    signal,
    sync::broadcast,
};
use tracing::{error, info};

mod args;
mod handler;
mod req;
mod resp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the default tracing subscriber.
    tracing_subscriber::fmt::init();

    let args = args::Args::parse();
    let port = args.port;
    let handler = args
        .root
        .map(handler::StaticFileHandler::with_root)
        .unwrap_or_else(|| {
            handler::StaticFileHandler::in_current_dir().expect("failed to get current dir")
        });

    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();

    info!("listening on: {}", listener.local_addr()?);

    let (exit_sender, _) = broadcast::channel::<bool>(1);

    tokio::spawn({
        let exit_sender = exit_sender.clone();
        async move {
            if let Ok(()) = signal::ctrl_c().await {
                info!("received Ctrl-C, shutting down");
                exit_sender.send(true).unwrap();
            }
        }
    });

    loop {
        let mut exit_receiver = exit_sender.subscribe();

        tokio::select! {
            Ok((stream, addr)) = listener.accept() => {
                let handler = handler.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, addr, &handler).await {
                        error!(?e, "failed to handle client");
                    }
                });
            },
            _ = exit_receiver.recv() => {
                info!("shutting down");
                break;
            }
        }
    }

    Ok(())
}

async fn handle_client(
    stream: TcpStream,
    addr: SocketAddr,
    handler: &handler::StaticFileHandler,
) -> anyhow::Result<()> {
    let mut stream = BufStream::new(stream);

    info!(?addr, "new connection");

    loop {
        let req = req::parse_request(&mut stream).await;

        match req {
            Ok(req) => {
                info!(?req, "incoming request");
                handle_req(req, &handler, &mut stream).await?;
            }
            Err(e) => {
                error!(?e, "failed to parse request");
                break;
            }
        }
    }

    info!(?addr, "closing connection");

    Ok(())
}

async fn handle_req<S: AsyncWrite + Unpin>(
    req: req::Request,
    handler: &handler::StaticFileHandler,
    stream: &mut S,
) -> anyhow::Result<bool> {
    let keep_alive = req.headers.get("Connection") == Some(&"keep-alive".to_string());

    match handler.handle(req).await {
        Ok(resp) => {
            resp.write(stream).await.unwrap();
        }
        Err(e) => {
            error!(?e, "failed to handle request");
            return Ok(false);
        }
    };

    stream.flush().await.unwrap();

    Ok(keep_alive)
}
