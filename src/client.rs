use tokio::io::{AsyncBufRead, AsyncWrite};

use crate::req::{parse_request, Request};

pub struct Connection<S: AsyncBufRead + AsyncWrite + Unpin> {
    pub stream: S,
}

impl<S: AsyncBufRead + AsyncWrite + Unpin> Connection<S> {
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    pub async fn next_request(&mut self) -> anyhow::Result<Request> {
        parse_request(&mut self.stream).await
    }
}
