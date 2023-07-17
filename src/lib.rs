#![forbid(unsafe_code)]
#![cfg_attr(windows, feature(abi_vectorcall))]

use std::collections::HashMap;
use std::error::Error;

use ext_php_rs::prelude::*;
use ext_php_rs::types::ZendClassObject;
use http::request::Builder;
use http_body_util::{BodyExt, Empty};
use hyper::body::{Bytes, Incoming};
use hyper::header::HeaderName;
use hyper::http::response::Parts;
use hyper::{HeaderMap, Method};
use once_cell::sync::OnceCell;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;

static RUNTIME: OnceCell<Runtime> = OnceCell::new();

fn get_runtime() -> &'static Runtime {
    RUNTIME.get().expect("Unitizialied Spidroin Runtime")
}

pub struct SpidroinError {
    pub description: String,
}

impl SpidroinError {
    pub fn new(description: String) -> Self {
        Self { description }
    }

    pub fn from<T: Error>(context: &str, error: &T) -> Self {
        Self {
            description: format!("{context}: {}", error),
        }
    }
}

impl From<SpidroinError> for PhpException {
    fn from(value: SpidroinError) -> PhpException {
        PhpException::default(format!("Spidroin Exception: {}", value.description))
    }
}

/// HTTP client
#[php_class(name = "Spidroin\\HttpClient")]
pub struct HttpClient;

impl HttpClient {
    fn request(&self, method: Method, uri: &str) -> PhpResult<RequestBuilder> {
        let uri = uri
            .parse::<hyper::Uri>()
            .map_err(|err| SpidroinError::from("Unable to parse URI", &err))?;

        // Get the host and the port
        let host = uri.host().expect("uri has no host");
        let port = uri.port_u16().unwrap_or(80);

        let address = format!("{}:{}", host, port);

        // The authority of our URL will be the hostname with port
        let authority = uri
            .authority()
            .ok_or_else(|| SpidroinError::new("Unable to extract authority".to_string()))?
            .clone();

        // Create an HTTP request with an empty body and a HOST header
        let builder = hyper::Request::builder()
            .method(method)
            .uri(uri)
            .header(hyper::header::HOST, authority.as_str());
        Ok(RequestBuilder { address, builder })
    }
}

#[php_impl]
impl HttpClient {
    pub fn __construct() -> Self {
        Self {}
    }

    /// Execute a HEAD request to the specified URL. Returns a Response object.
    pub fn head(&self, uri: &str) -> PhpResult<RequestBuilder> {
        self.request(Method::HEAD, uri)
    }

    /// Execute a GET request to the specified URL. Returns a Response object.
    pub fn get(&self, uri: &str) -> PhpResult<RequestBuilder> {
        self.request(Method::GET, uri)
    }

    /// Execute a POST request to the specified URL. Returns a Response object.
    pub fn post(&self, uri: &str) -> PhpResult<RequestBuilder> {
        self.request(Method::POST, uri)
    }

    /// Execute a PUT request to the specified URL. Returns a Response object.
    pub fn put(&self, uri: &str) -> PhpResult<RequestBuilder> {
        self.request(Method::PUT, uri)
    }

    /// Execute a PATCH request to the specified URL. Returns a Response object.
    pub fn patch(&self, uri: &str) -> PhpResult<RequestBuilder> {
        self.request(Method::PATCH, uri)
    }

    /// Execute a DELETE request to the specified URL. Returns a Response object.
    pub fn delete(&self, uri: &str) -> PhpResult<RequestBuilder> {
        self.request(Method::DELETE, uri)
    }

    /// Execute a CONNECT request to the specified URL. Returns a Response object.
    pub fn connect(&self, uri: &str) -> PhpResult<RequestBuilder> {
        self.request(Method::CONNECT, uri)
    }

    /// Execute a OPTIONS request to the specified URL. Returns a Response object.
    pub fn options(&self, uri: &str) -> PhpResult<RequestBuilder> {
        self.request(Method::OPTIONS, uri)
    }

    /// Execute a TRACE request to the specified URL. Returns a Response object.
    pub fn trace(&self, uri: &str) -> PhpResult<RequestBuilder> {
        self.request(Method::TRACE, uri)
    }
}

#[php_class(name = "Spidroin\\RequestBuilder")]
pub struct RequestBuilder {
    address: String,
    builder: Builder,
}

impl RequestBuilder {
    fn get_mut_headers(&mut self) -> PhpResult<&mut HeaderMap> {
        self.builder
            .headers_mut()
            .ok_or_else(|| SpidroinError::new("Unable to get headers".to_string()).into())
    }

    fn set_cookies(&mut self, cookies: HashMap<String, String>) -> PhpResult<()> {
        let request_headers = self.get_mut_headers()?;
        let mut encoded_cookies = String::new();
        for (i, (key, value)) in cookies.iter().enumerate() {
            if i != 0 {
                encoded_cookies.push_str("; ");
            }
            encoded_cookies.push_str(key);
            encoded_cookies.push('=');
            encoded_cookies.push_str(value);
        }
        request_headers.append(
            "Cookie",
            encoded_cookies.try_into().map_err(|err| {
                SpidroinError::from("Unable to convert cookies to header value", &err)
            })?,
        );
        Ok(())
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
    ) -> PhpResult<&mut ZendClassObject<Self>> {
        let request_headers = this.get_mut_headers()?;
        let update = update.unwrap_or(false);
        for (key, value) in headers.iter() {
            let key: HeaderName = key
                .try_into()
                .map_err(|err| SpidroinError::from("Unable to parse header name", &err))?;
            let value = value
                .try_into()
                .map_err(|err| SpidroinError::from("Unable to parse header value", &err))?;
            if update {
                request_headers.insert(key, value);
            } else {
                request_headers.append(key, value);
            }
        }
        Ok(this)
    }

    /// Add given cookies to the request as-is.
    ///
    /// @param cookies array<string, string>
    /// @return RequestBuilder
    pub fn with_raw_cookies(
        #[this] this: &mut ZendClassObject<Self>,
        cookies: HashMap<String, String>,
    ) -> PhpResult<&mut ZendClassObject<Self>> {
        this.set_cookies(cookies)?;
        Ok(this)
    }

    /// Add given cookies to the request with URL encoding.
    ///
    /// @param cookies array<string, string>
    /// @return RequestBuilder
    pub fn with_safe_cookies(
        #[this] this: &mut ZendClassObject<Self>,
        cookies: HashMap<String, String>,
    ) -> PhpResult<&mut ZendClassObject<Self>> {
        let safe_cookies = cookies
            .into_iter()
            .map(|(key, value)| (key, urlencoding::encode(&value).into_owned()))
            .collect::<HashMap<_, _>>();
        this.set_cookies(safe_cookies)?;
        Ok(this)
    }

    /// Send the request and return response.
    ///
    /// @return Response
    pub fn send(&mut self) -> PhpResult<Response> {
        let rt = get_runtime();

        let address = self.address.clone();
        let builder = std::mem::replace(&mut self.builder, Builder::new());
        let req = builder
            .body(Empty::<Bytes>::new())
            .map_err(|err| SpidroinError::from("Unable to build body", &err))?;

        let res = rt.block_on(async move {
            // Open a TCP connection to the remote host
            let stream = TcpStream::connect(address)
                .await
                .map_err(|err| SpidroinError::from("Unable to establish connection", &err))?;

            // Perform a TCP handshake
            let (mut sender, conn) = hyper::client::conn::http1::handshake(stream)
                .await
                .map_err(|err| SpidroinError::from("Unable to run handshake", &err))?;

            // spawn a task to poll the connection and drive the HTTP state
            tokio::task::spawn(async move {
                conn.await
                    .map_err(|err| SpidroinError::from("Unable to poll connection", &err))
            });

            // Await the response...
            sender
                .send_request(req)
                .await
                .map_err(|err| SpidroinError::from("Unable to send request", &err))
        })?;

        let (parts, body) = res.into_parts();

        Ok(Response {
            parts,
            body: Some(body),
        })
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
