use std::{
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    io::Cursor,
    path::Path,
};

use maplit::hashmap;
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
};

pub struct Response {
    pub status: Status,
    pub headers: HashMap<String, String>,
    pub data: Box<dyn AsyncRead + Unpin + Send>,
}

impl Response {
    pub fn status_and_headers(&self) -> String {
        let headers = self
            .headers
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\r\n");

        format!("HTTP/1.1 {}\r\n{headers}\r\n\r\n", self.status)
    }

    pub async fn write<O: AsyncWrite + Unpin>(mut self, stream: &mut O) -> anyhow::Result<()> {
        let bytes = self.status_and_headers().into_bytes();

        stream.write_all(&bytes).await?;

        tokio::io::copy(&mut self.data, stream).await?;

        Ok(())
    }

    pub fn from_html(status: Status, data: impl ToString) -> Self {
        let bytes = data.to_string().into_bytes();

        let headers = hashmap! {
            "Content-Type".to_string() => "text/html".to_string(),
            "Content-Length".to_string() => bytes.len().to_string(),
        };

        Self {
            status,
            headers,
            data: Box::new(Cursor::new(bytes)),
        }
    }

    pub async fn from_file(path: &Path, file: File) -> anyhow::Result<Response> {
        let headers = hashmap! {
            "Content-Length".to_string() => file.metadata().await?.len().to_string(),
            "Content-Type".to_string() => mime_type(path).to_string(),
        };

        Ok(Response {
            headers,
            status: Status::Ok,
            data: Box::new(file),
        })
    }
}

fn mime_type(path: &Path) -> &str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "text/javascript",
        Some("png") => "image/png",
        Some("jpg") => "image/jpeg",
        Some("gif") => "image/gif",
        _ => "application/octet-stream",
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Status {
    NotFound,
    Ok,
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::NotFound => write!(f, "404 Not Found"),
            Status::Ok => write!(f, "200 OK"),
        }
    }
}
