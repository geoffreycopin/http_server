use tokio::{
    io::{AsyncWriteExt, BufStream},
    net::TcpListener,
};
use tracing::info;

mod client;
mod req;
mod resp;

static DEFAULT_PORT: &str = "8080";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the default tracing subscriber.
    tracing_subscriber::fmt::init();

    let port: u16 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_PORT.to_string())
        .parse()?;

    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();

    info!("Listening on: {}", listener.local_addr()?);

    loop {
        let (stream, addr) = listener.accept().await?;
        let mut conn = client::Connection::new(BufStream::new(stream));

        info!(?addr, "new connection");

        while let Ok(req) = conn.next_request().await {
            info!(?addr, ?req, "incoming request");

            let resp = resp::Response::from_html(
                resp::Status::NotFound,
                include_str!("../static/404.html"),
            );

            resp.write(&mut conn.stream).await?;

            conn.stream.flush().await?;
        }

        conn.stream.write_all("Hello, world!\n".as_bytes()).await?;
    }
}
