#![cfg_attr(windows, feature(abi_vectorcall))]
#![forbid(unsafe_code)]
#![warn(clippy::unwrap_used)]
#![allow(clippy::should_implement_trait)]

mod error;
mod serde;

use std::io::{BufReader, Cursor};
use std::sync::Arc;
use std::{collections::HashMap, mem};

use base64::{engine::general_purpose::STANDARD, Engine};
use ext_php_rs::types::ZendHashTable;
use ext_php_rs::{
    binary::Binary,
    prelude::*,
    types::{ZendClassObject, Zval},
    zend::ce,
};
use http::{request::Builder, uri::Scheme, HeaderMap, HeaderValue, Method};
use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Bytes, Incoming},
    header::{HeaderName, AUTHORIZATION, CONTENT_TYPE},
    http::response::Parts,
};
use hyper_util::rt::TokioIo;
use once_cell::sync::OnceCell;
use rustls_pemfile::{read_one, Item};
use tokio::{net::TcpStream, runtime::Runtime};
use tokio_rustls::rustls::{Certificate, PrivateKey};
use tokio_rustls::{
    rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore, ServerName},
    TlsConnector,
};
use webpki_roots::TLS_SERVER_ROOTS;

use crate::{
    error::SilqError,
    serde::{ZvalDeserializer, ZvalSerializer},
};

static RUNTIME: OnceCell<Runtime> = OnceCell::new();

static CONTENT_TYPE_JSON: HeaderValue = HeaderValue::from_static("application/json");
static CONTENT_TYPE_FORM: HeaderValue =
    HeaderValue::from_static("application/x-www-form-urlencoded");

fn get_runtime() -> &'static Runtime {
    RUNTIME.get().expect("Uninitialized Silq Runtime")
}

/// HTTP client builder
#[php_class(name = "Silq\\HttpClientBuilder")]
pub struct HttpClientBuilder {
    allow_unsecure_http: bool,
    client_identity: Option<ClientIdentity>,
    ca_cert: Option<CertificateAuthority>,
}

#[php_impl]
impl HttpClientBuilder {
    #[constructor]
    pub fn default() -> HttpClientBuilder {
        Self {
            allow_unsecure_http: false,
            client_identity: None,
            ca_cert: None,
        }
    }

    pub fn allow_unsecure_http(
        #[this] this: &mut ZendClassObject<Self>,
        allow: bool,
    ) -> &mut ZendClassObject<Self> {
        this.allow_unsecure_http = allow;
        this
    }

    pub fn with_client_authentication<'a>(
        #[this] this: &'a mut ZendClassObject<Self>,
        client_identity: &ZendClassObject<ClientIdentity>,
    ) -> PhpResult<&'a mut ZendClassObject<Self>> {
        this.client_identity = Some((*client_identity).clone());
        Ok(this)
    }

    pub fn with_server_authentication<'a>(
        #[this] this: &'a mut ZendClassObject<Self>,
        certificate_authority: &ZendClassObject<CertificateAuthority>,
    ) -> PhpResult<&'a mut ZendClassObject<Self>> {
        this.ca_cert = Some((*certificate_authority).clone());
        Ok(this)
    }

    pub fn build(&mut self) -> PhpResult<HttpClient> {
        let transport_security = match self {
            HttpClientBuilder {
                allow_unsecure_http: true,
                client_identity: None,
                ca_cert: None,
            } => TransportSecurity::AllowUnsecure,
            HttpClientBuilder {
                allow_unsecure_http: false,
                client_identity,
                ca_cert,
                ..
            } => TransportSecurity::SecureOnly {
                client_identity: client_identity.clone(),
                ca_cert: ca_cert.clone(),
            },
            _ => Err(SilqError::new(
                "can't allow unsecure HTTP with client authentication".to_string(),
            ))?,
        };

        Ok(HttpClient { transport_security })
    }
}

#[derive(Clone)]
pub enum TransportSecurity {
    AllowUnsecure,
    SecureOnly {
        client_identity: Option<ClientIdentity>,
        ca_cert: Option<CertificateAuthority>,
    },
}

impl TransportSecurity {
    pub fn allow_unsecure(&self) -> bool {
        matches!(self, &TransportSecurity::AllowUnsecure)
    }
}

/// HTTP client
#[php_class(name = "Silq\\HttpClient")]
#[derive(Clone)]
pub struct HttpClient {
    transport_security: TransportSecurity,
}

#[php_impl]
impl HttpClient {
    pub fn default() -> Self {
        HttpClientBuilder::default()
            .build()
            .expect("default configuration must be safe")
    }

    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::default()
    }

    /// Execute a HEAD request to the specified URL. Returns a Response object.
    pub fn head(&self, uri: &str) -> PhpResult<RequestBuilder> {
        RequestBuilder::new(self.clone(), Method::HEAD, uri)
    }

    /// Execute a GET request to the specified URL. Returns a Response object.
    pub fn get(&self, uri: &str) -> PhpResult<RequestBuilder> {
        RequestBuilder::new(self.clone(), Method::GET, uri)
    }

    /// Execute a POST request to the specified URL. Returns a Response object.
    pub fn post(&self, uri: &str) -> PhpResult<RequestBuilder> {
        RequestBuilder::new(self.clone(), Method::POST, uri)
    }

    /// Execute a PUT request to the specified URL. Returns a Response object.
    pub fn put(&self, uri: &str) -> PhpResult<RequestBuilder> {
        RequestBuilder::new(self.clone(), Method::PUT, uri)
    }

    /// Execute a PATCH request to the specified URL. Returns a Response object.
    pub fn patch(&self, uri: &str) -> PhpResult<RequestBuilder> {
        RequestBuilder::new(self.clone(), Method::PATCH, uri)
    }

    /// Execute a DELETE request to the specified URL. Returns a Response object.
    pub fn delete(&self, uri: &str) -> PhpResult<RequestBuilder> {
        RequestBuilder::new(self.clone(), Method::DELETE, uri)
    }

    /// Execute a CONNECT request to the specified URL. Returns a Response object.
    pub fn connect(&self, uri: &str) -> PhpResult<RequestBuilder> {
        RequestBuilder::new(self.clone(), Method::CONNECT, uri)
    }

    /// Execute a OPTIONS request to the specified URL. Returns a Response object.
    pub fn options(&self, uri: &str) -> PhpResult<RequestBuilder> {
        RequestBuilder::new(self.clone(), Method::OPTIONS, uri)
    }

    /// Execute a TRACE request to the specified URL. Returns a Response object.
    pub fn trace(&self, uri: &str) -> PhpResult<RequestBuilder> {
        RequestBuilder::new(self.clone(), Method::TRACE, uri)
    }
}

enum Payload {
    Empty,
    Bytes(Vec<u8>),
}

#[php_class(name = "Silq\\RequestBuilder")]
pub struct RequestBuilder {
    client: HttpClient,
    scheme: Scheme,
    address: String,
    host: String,
    builder: Builder,
    payload: Payload,
}

impl RequestBuilder {
    pub fn new(client: HttpClient, method: Method, uri: &str) -> PhpResult<Self> {
        let uri = uri
            .parse::<hyper::Uri>()
            .map_err(|err| SilqError::from("Unable to parse URI", &err))?;

        let scheme = match uri.scheme() {
            None => return Err(SilqError::new("Missing URI scheme".to_string()).into()),
            Some(scheme) => (*scheme).to_owned(),
        };

        if !client.transport_security.allow_unsecure() && scheme.eq(&Scheme::HTTP) {
            Err(SilqError::new("Unsecure HTTP disabled".to_string()))?
        }

        let default_port = if scheme.eq("https") { 443 } else { 80 };

        let authority = uri
            .authority()
            .ok_or_else(|| SilqError::new("Unable to extract URI's authority".to_string()))?
            .clone();

        if authority.as_str().contains('@') {
            Err(SilqError::new(
                "Reject URI: contains username and password".to_string(),
            ))?
        }

        let host = authority
            .host()
            .trim_start_matches('[')
            .trim_end_matches(']');
        let port = authority.port_u16().unwrap_or(default_port);

        let address = format!("{}:{}", host, port);

        // Create an HTTP request with an empty body and a HOST header
        let builder = hyper::Request::builder()
            .method(method)
            .uri(uri)
            .header(hyper::header::HOST, authority.as_str());

        Ok(Self {
            client,
            scheme,
            address,
            host: host.to_string(),
            builder,
            payload: Payload::Empty,
        })
    }

    fn get_mut_headers(&mut self) -> PhpResult<&mut HeaderMap> {
        self.builder
            .headers_mut()
            .ok_or_else(|| SilqError::new("Unable to get headers".to_string()).into())
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
                SilqError::from("Unable to convert cookies to header value", &err)
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
                .map_err(|err| SilqError::from("Unable to parse header name", &err))?;
            let value = value
                .try_into()
                .map_err(|err| SilqError::from("Unable to parse header value", &err))?;
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

    /// Add given string as request's body.
    ///
    /// @param body string
    /// @return RequestBuilder
    pub fn with_body(
        #[this] this: &mut ZendClassObject<Self>,
        bytes: Binary<u8>,
    ) -> PhpResult<&mut ZendClassObject<Self>> {
        this.payload = Payload::Bytes(bytes.to_vec());
        Ok(this)
    }

    /// Add given array as request's body JSON serialized. Set the content-type header accordingly.
    ///
    /// @param body mixed
    /// @return RequestBuilder
    pub fn with_json<'a>(
        #[this] this: &'a mut ZendClassObject<Self>,
        body: &Zval,
    ) -> PhpResult<&'a mut ZendClassObject<Self>> {
        let request_headers = this.get_mut_headers()?;
        request_headers.insert(CONTENT_TYPE, CONTENT_TYPE_JSON.clone());
        this.payload = Payload::Bytes(
            serde_json::to_string(&ZvalSerializer(body))
                .map_err(|err| SilqError::from("Unable to encode value to JSON", &err))?
                .into_bytes(),
        );
        Ok(this)
    }

    /// Add given array as request's body url encoded as form. Set the content-type header accordingly.
    ///
    /// @param body mixed
    /// @return RequestBuilder
    pub fn with_form<'a>(
        #[this] this: &'a mut ZendClassObject<Self>,
        body: &Zval,
    ) -> PhpResult<&'a mut ZendClassObject<Self>> {
        let request_headers = this.get_mut_headers()?;
        request_headers.insert(CONTENT_TYPE, CONTENT_TYPE_FORM.clone());
        this.payload = Payload::Bytes(
            serde_urlencoded::to_string(ZvalSerializer(body))
                .map_err(|err| SilqError::from("Unable to encode value to url encoded form", &err))?
                .into_bytes(),
        );
        Ok(this)
    }

    /// Add basic authentication header with given user/password.
    ///
    /// @param user string user's name
    /// @param password string password
    /// @return RequestBuilder
    pub fn with_basic_auth<'a>(
        #[this] this: &'a mut ZendClassObject<Self>,
        user: &str,
        password: &str,
    ) -> PhpResult<&'a mut ZendClassObject<Self>> {
        let request_headers = this.get_mut_headers()?;
        let token = format!("Basic {}", STANDARD.encode(format!("{user}:{password}")));
        request_headers.insert(
            AUTHORIZATION,
            token
                .try_into()
                .map_err(|err| SilqError::from("Unable to encode token as header value", &err))?,
        );
        Ok(this)
    }

    /// Send the request and return response.
    ///
    /// @return Response
    pub fn send(&mut self) -> PhpResult<Response> {
        let rt = get_runtime();

        let address = self.address.clone();
        let builder = std::mem::replace(&mut self.builder, Builder::new());

        let req = match mem::replace(&mut self.payload, Payload::Empty) {
            Payload::Empty => builder.body(Full::<Bytes>::from(Bytes::new())),
            Payload::Bytes(bytes) => builder.body(Full::<Bytes>::from(bytes)),
        }
        .map_err(|err| SilqError::from("Unable to build body", &err))?;

        let res = rt.block_on(async move {
            let mut sender = if self.scheme.eq("https") {
                let root_store = {
                    let mut root_store = RootCertStore::empty();
                    if let TransportSecurity::SecureOnly {
                        ca_cert: Some(ca_cert),
                        ..
                    } = &self.client.transport_security
                    {
                        root_store.add(&ca_cert.certificate).map_err(|err| {
                            SilqError::from("Unable to use specified CA Certificate", &err)
                        })?;
                    } else {
                        root_store.add_trust_anchors(TLS_SERVER_ROOTS.0.iter().map(|ta| {
                            OwnedTrustAnchor::from_subject_spki_name_constraints(
                                ta.subject,
                                ta.spki,
                                ta.name_constraints,
                            )
                        }));
                    }
                    root_store
                };

                let tls_config = match &self.client.transport_security {
                    TransportSecurity::SecureOnly {
                        client_identity: Some(identity),
                        ..
                    } => ClientConfig::builder()
                        .with_safe_defaults()
                        .with_root_certificates(root_store)
                        .with_client_auth_cert(
                            vec![identity.certificate.clone()],
                            identity.private_key.clone(),
                        )
                        .map_err(|err| SilqError::from("Unable to use client identity", &err))?,

                    _ => ClientConfig::builder()
                        .with_safe_defaults()
                        .with_root_certificates(root_store)
                        .with_no_client_auth(),
                };

                let rc_tls_config = Arc::new(tls_config);

                let connector = TlsConnector::from(rc_tls_config);
                let name = ServerName::try_from(self.host.as_str())
                    .map_err(|err| SilqError::from("Unable to parse host", &err))?;
                let stream = TcpStream::connect(&address)
                    .await
                    .map_err(|err| SilqError::from("Unable to establish connection", &err))?;
                let stream = connector
                    .connect(name, stream)
                    .await
                    .map_err(|err| SilqError::from("Connection error", &err))?;
                let stream = TokioIo::new(stream);

                // Perform a TCP handshake
                let (sender, conn) = hyper::client::conn::http1::handshake(stream)
                    .await
                    .map_err(|err| SilqError::from("Unable to run handshake", &err))?;

                // spawn a task to poll the connection and drive the HTTP state
                tokio::task::spawn(async move {
                    conn.await
                        .map_err(|err| SilqError::from("Unable to poll connection", &err))
                });

                sender
            } else {
                let stream = TcpStream::connect(address)
                    .await
                    .map_err(|err| SilqError::from("Unable to establish connection", &err))?;
                let stream = TokioIo::new(stream);

                // Perform a TCP handshake
                let (sender, conn) = hyper::client::conn::http1::handshake(stream)
                    .await
                    .map_err(|err| SilqError::from("Unable to run handshake", &err))?;

                // spawn a task to poll the connection and drive the HTTP state
                tokio::task::spawn(async move {
                    conn.await
                        .map_err(|err| SilqError::from("Unable to poll connection", &err))
                });

                sender
            };

            // Await the response...
            sender
                .send_request(req)
                .await
                .map_err(|err| SilqError::from("Unable to send request", &err))
        })?;

        let (parts, body) = res.into_parts();

        Ok(Response {
            parts,
            body: Some(body),
        })
    }
}

/// HTTP Response
#[php_class(name = "Silq\\Response")]
pub struct Response {
    parts: Parts,
    body: Option<Incoming>,
}

#[php_impl]
impl Response {
    /// Returns the HTTP status as string
    pub fn get_status(&self) -> String {
        self.parts.status.to_string()
    }

    /// Returns the HTTP status code
    pub fn get_status_code(&self) -> u16 {
        self.parts.status.as_u16()
    }

    pub fn is_success(&self) -> bool {
        self.parts.status.is_success()
    }

    pub fn is_client_error(&self) -> bool {
        self.parts.status.is_client_error()
    }

    pub fn is_server_error(&self) -> bool {
        self.parts.status.is_server_error()
    }

    pub fn is_redirection(&self) -> bool {
        self.parts.status.is_redirection()
    }

    /// Returns the first header's value, or null.
    /// Ignore the header's name case.
    pub fn get_header_first_value(&self, header_name: &str) -> Option<Binary<u8>> {
        self.parts
            .headers
            .get(header_name)
            .map(|value| Binary::new(value.as_bytes()))
    }

    /// Returns all the values for the given header.
    /// Ignore the header's name case.
    pub fn get_header_all_values(&self, header_name: &str) -> Vec<Binary<u8>> {
        self.parts
            .headers
            .get_all(header_name)
            .iter()
            .map(|value| Binary::new(value.as_bytes()))
            .collect()
    }

    pub fn iter_headers(&self) -> HeaderIterator {
        HeaderIterator::new(self.parts.headers.clone())
    }

    /// Download body as raw bytes.
    pub fn get_bytes(&mut self) -> PhpResult<Binary<u8>> {
        let runtime = get_runtime();
        runtime.block_on(async {
            let mut body = self
                .body
                .take()
                .ok_or_else(|| SilqError::new("Body already consumed".into()))?;
            let mut content = vec![];
            while let Some(next) = body.frame().await {
                let frame =
                    next.map_err(|err| SilqError::from("Unable to fetch next frame", &err))?;
                if let Some(chunk) = frame.data_ref() {
                    content.extend_from_slice(chunk);
                }
            }
            Ok(Binary::from(content))
        })
    }

    /// Download body as utf-8 string.
    pub fn get_text(&mut self) -> PhpResult<String> {
        String::from_utf8(self.get_bytes()?.into())
            .map_err(|err| SilqError::from("Invalid UTF-8 string", &err).into())
    }

    /// Download body and parse it as JSON.
    pub fn get_json(&mut self) -> PhpResult<Zval> {
        match serde_json::from_slice::<ZvalDeserializer>(&self.get_bytes()?) {
            Err(err) => Err(SilqError::from("Invalid JSON", &err).into()),
            Ok(value) => Ok(value.0),
        }
    }

    pub fn iter_frames(&mut self) -> PhpResult<FrameIterator> {
        Ok(FrameIterator::new(self.body.take().ok_or_else(|| {
            SilqError::new("Body already consumed".into())
        })?))
    }
}

enum HeaderIteratorState {
    Uninitialized,
    Header { index: usize, header: Zval },
    Terminated,
}

#[php_class(name = "Silq\\HeaderIterator")]
#[implements(ce::iterator())]
pub struct HeaderIterator {
    headers: http::header::IntoIter<HeaderValue>,
    state: HeaderIteratorState,
}

impl HeaderIterator {
    fn new(headers: HeaderMap<HeaderValue>) -> Self {
        Self {
            headers: headers.into_iter(),
            state: HeaderIteratorState::Uninitialized,
        }
    }
}

#[php_impl]
impl HeaderIterator {
    pub fn current(&self) -> Zval {
        match &self.state {
            HeaderIteratorState::Header { header, .. } => header.shallow_clone(),
            _ => {
                panic!("Invalid call to HeaderIterator::current");
            }
        }
    }

    pub fn key(&self) -> usize {
        match &self.state {
            HeaderIteratorState::Header { index, .. } => *index,
            _ => {
                panic!("Invalid call to HeaderIterator::key");
            }
        }
    }

    pub fn next(&mut self) -> PhpResult<()> {
        match self.state {
            HeaderIteratorState::Header { index, .. } => match self.headers.next() {
                None => self.state = HeaderIteratorState::Terminated,
                Some((name, value)) => {
                    let mut array = ZendHashTable::new();
                    array
                        .push(name.expect("Infallible").to_string())
                        .expect("Infallible");
                    array
                        .push(Binary::new(value.as_bytes()))
                        .expect("Infallible");
                    let mut zval = Zval::new();
                    zval.set_hashtable(array);

                    self.state = HeaderIteratorState::Header {
                        index: index + 1,
                        header: zval,
                    }
                }
            },
            _ => {
                self.state = HeaderIteratorState::Terminated;
            }
        }
        Ok(())
    }

    pub fn rewind(&mut self) -> PhpResult<()> {
        match self.state {
            HeaderIteratorState::Uninitialized => match self.headers.next() {
                None => self.state = HeaderIteratorState::Terminated,
                Some((name, value)) => {
                    let mut array = ZendHashTable::new();
                    array
                        .push(name.expect("Infallible").to_string())
                        .expect("Infallible");
                    array
                        .push(Binary::new(value.as_bytes()))
                        .expect("Infallible");
                    let mut zval = Zval::new();
                    zval.set_hashtable(array);

                    self.state = HeaderIteratorState::Header {
                        index: 0,
                        header: zval,
                    }
                }
            },
            _ => {
                self.state = HeaderIteratorState::Terminated;
            }
        }
        Ok(())
    }

    pub fn valid(&self) -> bool {
        matches!(self.state, HeaderIteratorState::Header { .. })
    }
}

enum FrameIteratorState {
    Uninitialized,
    Frame { index: usize, frame: Vec<u8> },
    Terminated,
}

/// Iterator over body's frames
#[php_class(name = "Silq\\FrameIterator")]
#[implements(ce::iterator())]
pub struct FrameIterator {
    incoming: Incoming,
    state: FrameIteratorState,
}

impl FrameIterator {
    fn new(incoming: Incoming) -> Self {
        Self {
            incoming,
            state: FrameIteratorState::Uninitialized,
        }
    }

    fn fetch_next_frame(&mut self) -> PhpResult<()> {
        let index = match &self.state {
            FrameIteratorState::Terminated => {
                return Ok(());
            }
            FrameIteratorState::Uninitialized => 0,
            FrameIteratorState::Frame { index, .. } => index + 1,
        };

        let runtime = get_runtime();
        match runtime.block_on(async { self.incoming.frame().await }) {
            Some(Ok(frame)) => {
                self.state = FrameIteratorState::Frame {
                    frame: frame
                        .into_data()
                        .map_err(|_| SilqError::new("Unable to decode frame".into()))?
                        .to_vec(),
                    index,
                };
                Ok(())
            }
            None => {
                self.state = FrameIteratorState::Terminated;
                Ok(())
            }
            Some(Err(err)) => {
                self.state = FrameIteratorState::Terminated;
                Err(SilqError::from("Unable to fetch next frame", &err).into())
            }
        }
    }
}

#[php_impl]
impl FrameIterator {
    pub fn current(&self) -> Binary<u8> {
        match &self.state {
            FrameIteratorState::Frame { frame, .. } => Binary::from(frame.clone()),
            _ => {
                panic!("Invalid call to FrameIterator::current");
            }
        }
    }

    pub fn key(&self) -> usize {
        match &self.state {
            FrameIteratorState::Frame { index, .. } => *index,
            _ => {
                panic!("Invalid call to FrameIterator::key");
            }
        }
    }

    pub fn next(&mut self) -> PhpResult<()> {
        self.fetch_next_frame()
    }

    pub fn rewind(&mut self) -> PhpResult<()> {
        self.fetch_next_frame()
    }

    pub fn valid(&self) -> bool {
        matches!(self.state, FrameIteratorState::Frame { .. })
    }
}

#[php_class(name = "Silq\\ClientIdentity")]
#[derive(Clone)]
pub struct ClientIdentity {
    certificate: Certificate,
    private_key: PrivateKey,
}

#[php_impl]
impl ClientIdentity {
    pub fn from_base64_pem(certificate: &str, private_key: &str) -> PhpResult<Self> {
        let pem_certificate = String::from_utf8(
            STANDARD
                .decode(certificate)
                .map_err(|err| SilqError::from("Unable to decode base64 PEM", &err))?,
        )
        .map_err(|err| SilqError::from("Unable to parse PEM certificate", &err))?;

        let pem_private_key = String::from_utf8(
            STANDARD
                .decode(private_key)
                .map_err(|err| SilqError::from("Unable to decode base64 PEM", &err))?,
        )
        .map_err(|err| SilqError::from("Unable to parse PEM private key", &err))?;
        Self::from_pem(pem_certificate.as_str(), pem_private_key.as_str())
    }

    pub fn from_pem(certificate: &str, private_key: &str) -> PhpResult<Self> {
        let mut buffer = BufReader::new(Cursor::new(certificate));
        let certificate = match read_one(&mut buffer) {
            Ok(Some(Item::X509Certificate(data))) => Ok(Certificate(data)),
            Ok(None) => Err(SilqError::new("No certificate found".into())),
            Err(err) => Err(SilqError::from("Unable to read certificate", &err)),
            _ => Err(SilqError::new("Invalid certificate".into())),
        }?;

        let mut buffer = BufReader::new(Cursor::new(private_key));
        let private_key = match read_one(&mut buffer) {
            Ok(Some(Item::RSAKey(data)))
            | Ok(Some(Item::ECKey(data)))
            | Ok(Some(Item::PKCS8Key(data))) => Ok(PrivateKey(data)),
            Ok(None) => Err(SilqError::new("No secret key found".into())),
            Err(err) => Err(SilqError::from("Unable to read secret key", &err)),
            _ => Err(SilqError::new("Invalid secret key".into())),
        }?;

        Ok(Self {
            certificate,
            private_key,
        })
    }

    pub fn from_bytes(certificate: Binary<u8>, private_key: Binary<u8>) -> Self {
        Self {
            certificate: Certificate(certificate.to_vec()),
            private_key: PrivateKey(private_key.to_vec()),
        }
    }
}

#[php_class(name = "Silq\\CertificateAuthority")]
#[derive(Clone)]
pub struct CertificateAuthority {
    certificate: Certificate,
}

#[php_impl]
impl CertificateAuthority {
    pub fn from_base64_pem(certificate: &str) -> PhpResult<Self> {
        let pem = String::from_utf8(
            STANDARD
                .decode(certificate)
                .map_err(|err| SilqError::from("Unable to decode base64 PEM", &err))?,
        )
        .map_err(|err| SilqError::from("Unable to parse PEM certificate", &err))?;
        Self::from_pem(pem.as_str())
    }

    pub fn from_pem(certificate: &str) -> PhpResult<Self> {
        let mut buffer = BufReader::new(Cursor::new(certificate));
        match read_one(&mut buffer) {
            Ok(Some(Item::X509Certificate(data))) => Ok(Self {
                certificate: Certificate(data),
            }),
            Ok(None) => Err(SilqError::new("No certificate found".into()).into()),
            Err(err) => Err(SilqError::from("Unable to read certificate", &err).into()),
            _ => Err(SilqError::new("Invalid certificate".into()).into()),
        }
    }

    pub fn from_bytes(certificate: Binary<u8>) -> Self {
        Self {
            certificate: Certificate(certificate.to_vec()),
        }
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
