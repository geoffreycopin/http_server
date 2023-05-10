# Build a web server with Rust and tokio - Part 0: the simplest possible GET handler

Welcome to this series of blog posts where we will be exploring how to 
build a web server from scratch using the Rust programming language.
We will be taking a hands-on approach, maximizing our learning experience 
by using as few dependencies as possible and implementing as much logic as we can. 
This will enable us to understand the inner workings of a web server and the underlying 
protocols that it uses.

By the end of this tutorial, you will have a solid understanding of how to build a 
web server from scratch using Rust and the tokio library. So, let's dive in and 
get started on our journey!

In this first part, we'll be building a barebones web server that can only 
anwser GET requests with a static Not Found response. This will give us a
good starting point to build upon in the following tutorial.


## Setting up our project

First, we need to create a new Rust project. We'll use the following crates:
* [tokio](https://docs.rs/tokio/1.28.0/tokio/): async runtime 
* [anyhow](https://docs.rs/anyhow/1.0.44/anyhow/): easy error handling
* [maplit](https://docs.rs/maplit/1.0.2/maplit/): macro for creating HashMaps
* [tracing](https://docs.rs/tracing/0.1.27/tracing/): structured logging
* [tracing-subscriber](https://docs.rs/tracing-subscriber/0.2.19/tracing_subscriber/): instrumentation
```bash
cargo new webserver
cargo add tokio --features full
cargo add anyhow maplit tracing tracing-subscriber
```

## Anatomy of a simple GET request
In order to actually see what a GET request looks like, we'll set up a simple server 
listening on port 8080 that will print the incoming requests to the console.
This can be done with `netcat`:
```bash
nc -l 8080
```
Now, if we open a new terminal and use `curl` send a simple GET request to 
our server, we should see the following output:

<img src="https://github.com/geoffreycopin/http_server/blob/65207d0188ecd7ef8dbc39a83ebd4ac31289b56f/blog/img/coloured-get.png">

Let's break down the request parts:
* <span style="background-color: #F8676A">the method:</span>
    indicates the action to be performed on the resource. In this case, we are
    performing a GET request, which means we want to retrieve the resource.
* <span style="background-color: #D36FEB">the path:</span>
    uniquely identifies the resource. In this case, we are requesting
    the root path `/`. 
* <span style="background-color: #6D88FD">the protocol:</span>
    the protocol version. At this stage, we will always asume HTTP/1.1.
* <span style="background-color: #0F8D9F">the headers:</span>
    a set of key-value pairs that provide additional information about the request. 
    Our request contains the `Host` header, which indicates the host name of the server,
    the `User-Agent` header, which describes the client software that is making the request
    and the `Accept` header, which indicates the media types that are acceptable 
    for the response.
    We'll go into more details about headers in a later tutorial.

We'll use the following `struct` to represent requests in our code:
```rust
// req.rs

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Method {
    Get,
}
```

Parsing the request is just a matter of splitting the request string into
lines. The first line contains the method, path and protocol separated by spaces.
The following lines contain the headers, followed by an empty line.
```rust
// req.rs
use std::{collections::HashMap, hash::Hash};

use tokio::io::{AsyncBufRead, AsyncBufReadExt};

// [...]

impl TryFrom<&str> for Method {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "GET" => Ok(Method::Get),
            m => Err(anyhow::anyhow!("unsupported method: {m}")),
        }
    }
}

pub async fn parse_request(mut stream: impl AsyncBufRead + Unpin) -> anyhow::Result<Request> {
    let mut line_buffer = String::new();
    stream.read_line(&mut line_buffer).await?;

    let mut parts = line_buffer.split_whitespace();

    let method: Method = parts
        .next()
        .ok_or(anyhow::anyhow!("missing method"))
        .and_then(TryInto::try_into)?;

    let path: String = parts
        .next()
        .ok_or(anyhow::anyhow!("missing path"))
        .map(Into::into)?;

    let mut headers = HashMap::new();

    loop {
        line_buffer.clear();
        stream.read_line(&mut line_buffer).await?;

        if line_buffer.is_empty() || line_buffer == "\n" || line_buffer == "\r\n" {
            break;
        }

        let mut comps = line_buffer.split(":");
        let key = comps.next().ok_or(anyhow::anyhow!("missing header name"))?;
        let value = comps
            .next()
            .ok_or(anyhow::anyhow!("missing header value"))?
            .trim();

        headers.insert(key.to_string(), value.to_string());
    }

    Ok(Request {
        method,
        path,
        headers,
    })
}
```

## Accepting connections
Now that we know how to parse a request, we can start accepting connections.
Each time a new connection is established, we'll spawn a new task to handle it
in order to keep the main thread free to accept new connections.
```rust
// main.rs
use tokio::{io::BufStream, net::TcpListener};
use tracing::info;

mod req;

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
        });
    }
}
```

We can now run our server on port `8081`with the following command: 
`cargo run -- 8081`.
Sending a GET request to `localhost:8081` should print the following output:
```
INFO http_server: listening on: 0.0.0.0:8081
INFO http_server: new connection addr=127.0.0.1:49351
INFO http_server: incoming request req=Request { method: Get, path: "/", headers: {"Host": "localhost", "User-Agent": "curl/7.87.0", "Accept": "*/*"} }
```

## Sending a response