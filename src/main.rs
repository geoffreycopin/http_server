use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;
use tokio::{
    io::{AsyncWrite, BufStream},
    net::{TcpListener, TcpStream},
    signal,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

mod handler;
mod req;
mod resp;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,
    #[arg(short, long)]
    pub root: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the default tracing subscriber.
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let port = args.port;
    let handler = args
        .root
        .map(handler::StaticFileHandler::with_root)
        .unwrap_or_else(|| {
            handler::StaticFileHandler::in_current_dir().expect("failed to get current dir")
        });

    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();

    info!("listening on: {}", listener.local_addr()?);

    let cancel_token = CancellationToken::new();

    tokio::spawn({
        let cancel_token = cancel_token.clone();
        async move {
            if let Ok(()) = signal::ctrl_c().await {
                info!("received Ctrl-C, shutting down");
                cancel_token.cancel();
            }
        }
    });

    let mut tasks = Vec::new();

    loop {
        let cancel_token = cancel_token.clone();

        tokio::select! {
            Ok((stream, addr)) = listener.accept() => {
                let handler = handler.clone();
                let client_task = tokio::spawn(async move {
                    if let Err(e) = handle_client(cancel_token, stream, addr, &handler).await {
                        error!(?e, "failed to handle client");
                    }
                });
                tasks.push(client_task);
            },
            _ = cancel_token.cancelled() => {
                info!("stop listening");
                break;
            }
        }
    }

    futures::future::join_all(tasks).await;

    Ok(())
}

async fn handle_client(
    cancel_token: CancellationToken,
    stream: TcpStream,
    addr: SocketAddr,
    handler: &handler::StaticFileHandler,
) -> anyhow::Result<()> {
    let mut stream = BufStream::new(stream);

    info!(?addr, "new connection");

    loop {
        tokio::select! {
            req = req::parse_request(&mut stream) => {
                match req {
                    Ok(req) => {
                        info!(?req, "incoming request");
                        let close_conn = handle_req(req, &handler, &mut stream).await?;
                        if close_conn {
                            break;
                        }
                    }
                    Err(e) => {
                        error!(?e, "failed to parse request");
                        break;
                    }
                }
            }
            _ = cancel_token.cancelled() => {
                info!(?addr, "closing connection");
                break;
            }
        }
    }

    Ok(())
}

async fn handle_req<S: AsyncWrite + Unpin>(
    req: req::Request,
    handler: &handler::StaticFileHandler,
    stream: &mut S,
) -> anyhow::Result<bool> {
    let close_connection = req.headers.get("Connection") == Some(&"close".to_string());

    match handler.handle(req).await {
        Ok(resp) => {
            resp.write(stream).await.unwrap();
        }
        Err(e) => {
            error!(?e, "failed to handle request");
            return Ok(false);
        }
    };

    Ok(close_connection)
}
