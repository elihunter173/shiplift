//! Transports for communicating with the docker daemon

use std::{
    fmt, io, iter,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::{
    io::{AsyncRead, AsyncWrite},
    stream::Stream,
    StreamExt, TryFutureExt,
};
use hyper::{
    body::Bytes,
    client::{Client, HttpConnector},
    header, Body, Method, Request, StatusCode, Uri,
};
use mime::Mime;
use pin_project::pin_project;
use serde::{Deserialize, Serialize};

#[cfg(feature = "tls")]
use hyper_openssl::HttpsConnector;
#[cfg(feature = "tls")]
use openssl::ssl::{SslConnector, SslFiletype, SslMethod};

#[cfg(feature = "unix-socket")]
use hyperlocal::UnixConnector;
#[cfg(feature = "unix-socket")]
use hyperlocal::Uri as DomainUri;

use crate::{Error, Result};

pub fn tar() -> Mime {
    "application/tar".parse().unwrap()
}

/// Transports are types which define the means of communication
/// with the docker daemon
#[derive(Clone)]
pub enum Transport {
    /// A network tcp interface
    Tcp {
        client: Client<HttpConnector>,
        host: String,
    },
    /// TCP/TLS
    #[cfg(feature = "tls")]
    EncryptedTcp {
        client: Client<HttpsConnector<HttpConnector>>,
        host: String,
    },
    /// A Unix domain socket
    #[cfg(feature = "unix-socket")]
    Unix {
        client: Client<UnixConnector>,
        path: String,
    },
}

impl fmt::Debug for Transport {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        match *self {
            Transport::Tcp { ref host, .. } => write!(f, "Tcp({})", host),
            #[cfg(feature = "tls")]
            Transport::EncryptedTcp { ref host, .. } => write!(f, "EncryptedTcp({})", host),
            #[cfg(feature = "unix-socket")]
            Transport::Unix { ref path, .. } => write!(f, "Unix({})", path),
        }
    }
}

fn get_http_connector() -> HttpConnector {
    let mut http = HttpConnector::new();
    http.enforce_http(false);
    http
}

impl Transport {
    pub(crate) fn from_uri(uri: Uri) -> Self {
        let tcp_host_str = format!(
            "{}://{}:{}",
            uri.scheme_str().unwrap(),
            uri.host().unwrap().to_owned(),
            uri.port_u16().unwrap_or(80)
        );

        match uri.scheme_str() {
            #[cfg(feature = "unix-socket")]
            Some("unix") => Transport::Unix {
                client: Client::builder().build(UnixConnector),
                path: uri.path().to_owned(),
            },

            #[cfg(not(feature = "unix-socket"))]
            Some("unix") => panic!("Unix socket support is disabled"),

            _ => Self::from_tcp(tcp_host_str),
        }
    }

    #[cfg(feature = "unix-socket")]
    pub(crate) fn from_unix_socket(socket_path: String) -> Self {
        Transport::Unix {
            client: Client::builder()
                .pool_max_idle_per_host(0)
                .build(UnixConnector),
            path: socket_path,
        }
    }

    #[cfg(feature = "tls")]
    fn from_tcp(tcp_host_str: String) -> Self {
        let http = get_http_connector();
        // TODO: Don't hardcode DOCKER_CERT_PATH envvars?
        if let Ok(ref certs) = std::env::var("DOCKER_CERT_PATH") {
            // fixme: don't unwrap before you know what's in the box
            // https://github.com/hyperium/hyper/blob/master/src/net.rs#L427-L428
            let mut connector = SslConnector::builder(SslMethod::tls()).unwrap();
            connector.set_cipher_list("DEFAULT").unwrap();
            let cert = &format!("{}/cert.pem", certs);
            let key = &format!("{}/key.pem", certs);
            connector
                .set_certificate_file(&Path::new(cert), SslFiletype::PEM)
                .unwrap();
            connector
                .set_private_key_file(&Path::new(key), SslFiletype::PEM)
                .unwrap();
            if std::env::var("DOCKER_TLS_VERIFY").is_ok() {
                let ca = &format!("{}/ca.pem", certs);
                connector.set_ca_file(&Path::new(ca)).unwrap();
            }

            // If we are attempting to connec to the docker daemon via tcp
            // we need to convert the scheme to `https` to let hyper connect.
            // Otherwise, hyper will reject the connection since it does not
            // recongnize `tcp` as a valid `http` scheme.
            let tcp_host_str = if tcp_host_str.contains("tcp://") {
                tcp_host_str.replace("tcp://", "https://")
            } else {
                tcp_host_str
            };

            Self::EncryptedTcp {
                client: Client::builder()
                    .build(HttpsConnector::with_connector(http, connector).unwrap()),
                host: tcp_host_str,
            }
        } else {
            Self::Tcp {
                client: Client::builder().build(http),
                host: tcp_host_str,
            }
        }
    }

    #[cfg(not(feature = "tls"))]
    fn from_tcp(tcp_host_str: String) -> Self {
        let http = get_http_connector();
        Self::Tcp {
            client: Client::builder().build(http),
            host: tcp_host_str,
        }
    }

    // TODO: fixme
    // Taken from https://github.com/softprops/shiplift/issues/226
    // See https://github.com/fussybeaver/bollard/blob/master/src/docker.rs#L386
    /// Configure an HTTPS/HTTP connector.
    // #[cfg(feature = "rustls-tls")]
    fn from_rustls() -> Self {
        // This code is adapted from the default configuration setup at
        // https://github.com/ctz/hyper-rustls/blob/69133c8d81442f5efa1d3bba5626049bf1573c22/src/connector.rs#L27-L59

        // Set up HTTP.
        let mut http = HttpConnector::new();
        http.enforce_http(false);

        // Set up SSL parameters.
        let mut config = ClientConfig::new();
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        config.ct_logs = Some(&ct_logs::LOGS);

        // Look up any certs managed by the operating system.
        config.root_store = match rustls_native_certs::load_native_certs() {
            Ok(store) => store,
            Err((Some(store), err)) => {
                log::warn!("could not load all certificates: {}", err);
                store
            }
            Err((None, err)) => {
                log::warn!("cannot access native certificate store: {}", err);
                config.root_store
            }
        };

        // Add any webpki certs, too, in case the OS is useless.
        config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);

        // Install our Docker CA if we have one.
        if should_enable_tls() {
            let ca_path = docker_ca_pem_path()?;
            let mut rdr = open_buffered(&ca_path)?;
            config
                .root_store
                .add_pem_file(&mut rdr)
                .map_err(|_| format!("error reading {}", ca_path.display()))?;
        }

        // Install a client certificate resolver to find our client cert (if we need one).
        config.client_auth_cert_resolver = Arc::new(DockerClientCertResolver);

        Ok(Connector::Https(HttpsConnector::from((http, config))))
    }

    /// Make a request and return the whole response in a `String`
    pub async fn request<B>(
        &self,
        method: Method,
        endpoint: impl AsRef<str>,
        body: Option<(B, Mime)>,
    ) -> Result<String>
    where
        B: Into<Body>,
    {
        let body = self
            .get_body(method, endpoint, body, None::<iter::Empty<_>>)
            .await?;
        let bytes = hyper::body::to_bytes(body).await?;
        let string = String::from_utf8(bytes.to_vec())?;

        Ok(string)
    }

    async fn get_body<B, H>(
        &self,
        method: Method,
        endpoint: impl AsRef<str>,
        body: Option<(B, Mime)>,
        headers: Option<H>,
    ) -> Result<Body>
    where
        B: Into<Body>,
        H: IntoIterator<Item = (&'static str, String)>,
    {
        let req = self
            .build_request(method, endpoint, body, headers, Request::builder())
            .expect("Failed to build request!");

        let response = self.send_request(req).await?;

        let status = response.status();

        match status {
            // Success case: pass on the response
            StatusCode::OK
            | StatusCode::CREATED
            | StatusCode::SWITCHING_PROTOCOLS
            | StatusCode::NO_CONTENT => Ok(response.into_body()),
            _ => {
                let bytes = hyper::body::to_bytes(response.into_body()).await?;
                let message_body = String::from_utf8(bytes.to_vec())?;

                Err(Error::Fault {
                    code: status,
                    message: Self::get_error_message(&message_body).unwrap_or_else(|| {
                        status
                            .canonical_reason()
                            .unwrap_or("unknown error code")
                            .to_owned()
                    }),
                })
            }
        }
    }

    async fn get_chunk_stream<B, H>(
        &self,
        method: Method,
        endpoint: impl AsRef<str>,
        body: Option<(B, Mime)>,
        headers: Option<H>,
    ) -> Result<impl Stream<Item = Result<Bytes>>>
    where
        B: Into<Body>,
        H: IntoIterator<Item = (&'static str, String)>,
    {
        let body = self.get_body(method, endpoint, body, headers).await?;

        Ok(stream_body(body))
    }

    pub fn stream_chunks<'a, H, B>(
        &'a self,
        method: Method,
        endpoint: impl AsRef<str> + 'a,
        body: Option<(B, Mime)>,
        headers: Option<H>,
    ) -> impl Stream<Item = Result<Bytes>> + 'a
    where
        H: IntoIterator<Item = (&'static str, String)> + 'a,
        B: Into<Body> + 'a,
    {
        self.get_chunk_stream(method, endpoint, body, headers)
            .try_flatten_stream()
    }

    /// Builds an HTTP request.
    fn build_request<B, H>(
        &self,
        method: Method,
        endpoint: impl AsRef<str>,
        body: Option<(B, Mime)>,
        headers: Option<H>,
        builder: hyper::http::request::Builder,
    ) -> Result<Request<Body>>
    where
        B: Into<Body>,
        H: IntoIterator<Item = (&'static str, String)>,
    {
        let req = match *self {
            Transport::Tcp { ref host, .. } => {
                builder
                    .method(method)
                    .uri(&format!("{}{}", host, endpoint.as_ref()))
            }
            #[cfg(feature = "tls")]
            Transport::EncryptedTcp { ref host, .. } => {
                builder
                    .method(method)
                    .uri(&format!("{}{}", host, endpoint.as_ref()))
            }
            #[cfg(feature = "unix-socket")]
            Transport::Unix { ref path, .. } => {
                let uri = DomainUri::new(&path, endpoint.as_ref());
                builder.method(method).uri(uri)
            }
        };
        let mut req = req.header(header::HOST, "");

        if let Some(h) = headers {
            for (k, v) in h.into_iter() {
                req = req.header(k, v);
            }
        }

        match body {
            Some((b, c)) => Ok(req
                .header(header::CONTENT_TYPE, &c.to_string()[..])
                .body(b.into())?),
            _ => Ok(req.body(Body::empty())?),
        }
    }

    /// Send the given request to the docker daemon and return a Future of the response.
    async fn send_request(
        &self,
        req: Request<hyper::Body>,
    ) -> Result<hyper::Response<Body>> {
        match self {
            Transport::Tcp { ref client, .. } => Ok(client.request(req).await?),
            #[cfg(feature = "tls")]
            Transport::EncryptedTcp { ref client, .. } => Ok(client.request(req).await?),
            #[cfg(feature = "unix-socket")]
            Transport::Unix { ref client, .. } => Ok(client.request(req).await?),
        }
    }

    /// Makes an HTTP request, upgrading the connection to a TCP
    /// stream on success.
    ///
    /// This method can be used for operations such as viewing
    /// docker container logs interactively.
    async fn stream_upgrade_tokio<B>(
        &self,
        method: Method,
        endpoint: impl AsRef<str>,
        body: Option<(B, Mime)>,
    ) -> Result<hyper::upgrade::Upgraded>
    where
        B: Into<Body>,
    {
        let req = self
            .build_request(
                method,
                endpoint,
                body,
                None::<iter::Empty<_>>,
                Request::builder()
                    .header(header::CONNECTION, "Upgrade")
                    .header(header::UPGRADE, "tcp"),
            )
            .expect("Failed to build request!");

        let response = self.send_request(req).await?;

        match response.status() {
            StatusCode::SWITCHING_PROTOCOLS => Ok(hyper::upgrade::on(response).await?),
            _ => Err(Error::ConnectionNotUpgraded),
        }
    }

    pub async fn stream_upgrade<B>(
        &self,
        method: Method,
        endpoint: impl AsRef<str>,
        body: Option<(B, Mime)>,
    ) -> Result<impl AsyncRead + AsyncWrite>
    where
        B: Into<Body>,
    {
        let tokio_multiplexer = self.stream_upgrade_tokio(method, endpoint, body).await?;

        Ok(Compat { tokio_multiplexer })
    }

    /// Extract the error message content from an HTTP response that
    /// contains a Docker JSON error structure.
    fn get_error_message(body: &str) -> Option<String> {
        serde_json::from_str::<ErrorResponse>(body)
            .map(|e| e.message)
            .ok()
    }
}

#[pin_project]
struct Compat<S> {
    #[pin]
    tokio_multiplexer: S,
}

impl<S> AsyncRead for Compat<S>
where
    S: tokio::io::AsyncRead,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut readbuf = tokio::io::ReadBuf::new(buf);
        match self.project().tokio_multiplexer.poll_read(cx, &mut readbuf) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => Poll::Ready(Ok(readbuf.filled().len())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
        }
    }
}

impl<S> AsyncWrite for Compat<S>
where
    S: tokio::io::AsyncWrite,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.project().tokio_multiplexer.poll_write(cx, buf)
    }
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        self.project().tokio_multiplexer.poll_flush(cx)
    }
    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        self.project().tokio_multiplexer.poll_shutdown(cx)
    }
}

#[derive(Serialize, Deserialize)]
struct ErrorResponse {
    message: String,
}

fn stream_body(body: Body) -> impl Stream<Item = Result<Bytes>> {
    async fn unfold(mut body: Body) -> Option<(Result<Bytes>, Body)> {
        let chunk_result = body.next().await?.map_err(Error::from);

        Some((chunk_result, body))
    }

    futures_util::stream::unfold(body, unfold)
}
