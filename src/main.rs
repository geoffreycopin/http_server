use tokio::{io::BufStream, net::TcpListener};
use tracing::info;

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

    info!("listening on: {}", listener.local_addr()?);

    loop {
        let (stream, addr) = listener.accept().await?;
        let mut stream = BufStream::new(stream);

        // do not block the main thread, spawn a new task
        tokio::spawn(async move {
            info!(?addr, "new connection");

            match req::parse_request(&mut stream).await {
                Ok(req) => info!(?req, "incoming request"),
                Err(e) => {
                    info!(?e, "failed to parse request");
                }
            }

            let resp = resp::Response::from_html(
                resp::Status::NotFound,
                include_str!("../static/404.html"),
            );

            resp.write(&mut stream).await.unwrap();
        });
    }
}
