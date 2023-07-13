#![forbid(unsafe_code)]
#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::prelude::*;
use ext_php_rs::types::ZendClassObject;
use http::request::Builder;
use http_body_util::{BodyExt, Empty};
use hyper::body::{Bytes, Incoming};
use hyper::header::HeaderName;
use hyper::http::response::Parts;
use hyper::{HeaderMap, Method};
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;

static RUNTIME: OnceCell<Runtime> = OnceCell::new();

fn get_runtime() -> &'static Runtime {
    RUNTIME.get().expect("Unitizialied Spidroin Runtime")
}

/// HTTP client
#[php_class(name = "Spidroin\\HttpClient")]
pub struct HttpClient;

#[php_impl]
impl HttpClient {
    pub fn __construct() -> Self {
        Self {}
    }

    /// Execute a GET request to the specified URL. Returns a Response object.
    pub fn get(&self, uri: &str) -> RequestBuilder {
        let uri = uri.parse::<hyper::Uri>().unwrap();

        // Get the host and the port
        let host = uri.host().expect("uri has no host");
        let port = uri.port_u16().unwrap_or(80);

        let address = format!("{}:{}", host, port);

        // The authority of our URL will be the hostname with port
        let authority = uri.authority().unwrap().clone();

        // Create an HTTP request with an empty body and a HOST header
        let builder = hyper::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header(hyper::header::HOST, authority.as_str());
        println!("YEAH");
        RequestBuilder { address, builder }
    }
}

#[php_class(name = "Spidroin\\RequestBuilder")]
pub struct RequestBuilder {
    address: String,
    builder: Builder,
}

impl RequestBuilder {
    fn get_mut_headers(&mut self) -> &mut HeaderMap {
        self.builder.headers_mut().unwrap()
    }
}

#[php_impl]
impl RequestBuilder {
    /// Add given headers to the request.
    ///
    /// @param headers array<string, string>
    /// @param update bool [default: false] Whether update value of preexisting headers.
    /// @return RequestBuilder
    pub fn with_headers(
        #[this] this: &mut ZendClassObject<Self>,
        headers: HashMap<String, String>,
        update: Option<bool>,
    ) -> &mut ZendClassObject<Self> {
        let request_headers = this.get_mut_headers();
        let update = update.unwrap_or(false);
        for (key, value) in headers.iter() {
            let key: HeaderName = key.try_into().unwrap();
            let value = value.try_into().unwrap();
            if update {
                request_headers.insert(key, value);
            } else {
                request_headers.append(key, value);
            }
        }
        this
    }

    /// Send the request and return response.
    ///
    /// @return Response
    pub fn send(&mut self) -> Response {
        let rt = get_runtime();

        let address = self.address.clone();
        let builder = std::mem::replace(&mut self.builder, Builder::new());
        let req = builder.body(Empty::<Bytes>::new()).unwrap();

        let res = rt.block_on(async move {
            // Open a TCP connection to the remote host
            let stream = TcpStream::connect(address).await.unwrap();

            // Perform a TCP handshake
            let (mut sender, conn) = hyper::client::conn::http1::handshake(stream).await.unwrap();

            // spawn a task to poll the connection and drive the HTTP state
            tokio::task::spawn(async move {
                conn.await.unwrap();
            });

            // Await the response...
            sender.send_request(req).await.unwrap()
        });

        let (parts, body) = res.into_parts();

        Response {
            parts,
            body: Some(body),
        }
    }
}

/// HTTP Response
#[php_class(name = "Spidroin\\Response")]
pub struct Response {
    parts: Parts,
    body: Option<Incoming>,
}

#[php_impl]
impl Response {
    /// Returns the HTTP status code
    pub fn get_status_code(&self) -> u16 {
        self.parts.status.as_u16()
    }

    /// Download body as utf-8 string.
    pub fn get_text(&mut self) -> String {
        let rt = get_runtime();
        let res = &mut self.body.take().unwrap();
        rt.block_on(async move {
            let mut body = String::new();
            while let Some(next) = res.frame().await {
                let frame = next.unwrap();
                if let Some(chunk) = frame.data_ref() {
                    body.push_str(&String::from_utf8(chunk.to_vec()).unwrap());
                }
            }
            body
        })
    }
}

#[php_startup]
fn startup() {
    RUNTIME
        .set(Runtime::new().expect("Unable to create async runtime"))
        .expect("Unable to set global runtime");
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
}
