use std::io::Cursor;
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use maplit::hashmap;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct Response<S: AsyncRead + Unpin> {
    pub status: Status,
    pub headers: HashMap<String, String>,
    pub data: S,
}

impl<S: AsyncRead + Unpin> Response<S> {
    pub fn status_and_headers(&self) -> String {
        let headers = self
            .headers
            .iter()
            .map(|(k, v)| format!("{}: {}\r\n", k, v))
            .collect::<Vec<_>>()
            .join("");

        format!("HTTP/1.1 {}\r\n{headers}\r\n\r\n", self.status)
    }

    pub async fn write<O: AsyncWrite + Unpin>(mut self, stream: &mut O) -> anyhow::Result<()> {
        stream
            .write_all(self.status_and_headers().as_bytes())
            .await?;

        tokio::io::copy(&mut self.data, stream).await?;

        Ok(())
    }
}

impl Response<Cursor<Vec<u8>>> {
    pub fn from_html(status: Status, data: impl ToString) -> Self {
        let bytes = data.to_string().into_bytes();

        let headers = hashmap! {
            "Content-Type".to_string() => "text/html".to_string(),
            "Content-Length".to_string() => (bytes.len() + 2).to_string(),
        };

        Self {
            status,
            headers,
            data: Cursor::new(bytes),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Status {
    NotFound,
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::NotFound => write!(f, "404 Not Found"),
        }
    }
}
