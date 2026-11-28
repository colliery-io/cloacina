/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! A private CA for the connection to the server (CLOACINA-T-1069).
//!
//! TLS between the agents and the server ends in the cluster: a mesh or a
//! gateway in front of the server. A gateway may have a certificate that a
//! private CA signs. `--ca-file` names a PEM file of CA certificates; the HTTP
//! client and the WebSocket connector trust them in addition to the system
//! roots. A file that cannot be read, or holds no certificate, stops the agent
//! at start.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow, bail};
use rustls::RootCertStore;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject as _;
use tracing::warn;

/// The CA certificates of a `--ca-file`. Empty when no file is given.
#[derive(Debug, Clone, Default)]
pub struct ExtraRoots {
    certs: Vec<CertificateDer<'static>>,
}

impl ExtraRoots {
    /// Read the CA certificates of `path`, or none with no path. Fails when
    /// the file cannot be read, is not PEM, holds no certificate, or holds a
    /// certificate that cannot be a trust anchor.
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let Some(path) = path else {
            return Ok(Self::default());
        };
        let bytes = std::fs::read(path).with_context(|| {
            format!("read the CA file {path:?} (--ca-file / CLOACINA_AGENT_CA_FILE)")
        })?;
        let certs = CertificateDer::pem_slice_iter(&bytes)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow!("the CA file {path:?} is not PEM: {e}"))?;
        if certs.is_empty() {
            bail!("the CA file {path:?} holds no certificate");
        }
        let mut store = RootCertStore::empty();
        let (_, ignored) = store.add_parsable_certificates(certs.iter().cloned());
        if ignored > 0 {
            bail!(
                "the CA file {path:?} holds {ignored} certificate(s) that cannot be a trust anchor"
            );
        }
        Ok(Self { certs })
    }

    /// Whether a CA file was given.
    pub fn is_empty(&self) -> bool {
        self.certs.is_empty()
    }

    /// The number of CA certificates.
    pub fn len(&self) -> usize {
        self.certs.len()
    }

    /// The HTTP client of the agent: the system roots, and these certificates.
    pub fn http_client(&self) -> Result<reqwest::Client> {
        let mut builder = reqwest::Client::builder()
            .user_agent(concat!("cloacina-agent/", env!("CARGO_PKG_VERSION")));
        if !self.certs.is_empty() {
            let certs = self
                .certs
                .iter()
                .map(|c| reqwest::Certificate::from_der(c.as_ref()))
                .collect::<Result<Vec<_>, _>>()
                .context("read the CA certificates for the HTTP client")?;
            builder = builder.tls_certs_merge(certs);
        }
        builder.build().context("build HTTP client")
    }

    /// The roots of the WebSocket connector: the native roots (as
    /// tokio-tungstenite loads them) and these certificates.
    fn ws_roots(&self) -> RootCertStore {
        let mut roots = RootCertStore::empty();
        let native = rustls_native_certs::load_native_certs();
        if !native.errors.is_empty() {
            warn!(errors = ?native.errors, "some system root certificates could not be loaded");
        }
        roots.add_parsable_certificates(native.certs);
        roots.add_parsable_certificates(self.certs.iter().cloned());
        roots
    }

    /// The TLS configuration of the WebSocket client. `None` with no CA file:
    /// tokio-tungstenite then uses its own, the native roots.
    pub fn ws_client_config(&self) -> Option<Arc<rustls::ClientConfig>> {
        if self.certs.is_empty() {
            return None;
        }
        Some(Arc::new(
            rustls::ClientConfig::builder()
                .with_root_certificates(self.ws_roots())
                .with_no_client_auth(),
        ))
    }

    /// The connector for `connect_async_tls_with_config`.
    pub fn ws_connector(&self) -> Option<tokio_tungstenite::Connector> {
        self.ws_client_config()
            .map(tokio_tungstenite::Connector::Rustls)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    /// A test CA, made for these tests only; valid until 2126.
    const TEST_CA_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIBnjCCAUOgAwIBAgIUfkN4aMUrFO/E3cPa88tW88sGLOowCgYIKoZIzj0EAwIw
GzEZMBcGA1UEAwwQY2xvYWNpbmEgdGVzdCBDQTAgFw0yNjExMjcyMjE0NTJaGA8y
MTI2MTEwMzIyMTQ1MlowGzEZMBcGA1UEAwwQY2xvYWNpbmEgdGVzdCBDQTBZMBMG
ByqGSM49AgEGCCqGSM49AwEHA0IABHHbKpLz77G7Cjfz7F1sBYKpuCOPB/5fNK7C
bVQqSW6+g1Ouo2wGounO4qQEq/rqOgNZrag/e0nwHgjeBO7nOSWjYzBhMB0GA1Ud
DgQWBBQPVObDqyll4COcSgfZmwPmdwW1CTAfBgNVHSMEGDAWgBQPVObDqyll4COc
SgfZmwPmdwW1CTAPBgNVHRMBAf8EBTADAQH/MA4GA1UdDwEB/wQEAwIBBjAKBggq
hkjOPQQDAgNJADBGAiEA4lKn+iviOItlhJz8LxsxoJQIbtikQP40kSpjCiaUu28C
IQC0jrq7JYgeR4eO19kvpDf+Yo4jlOqxrdgy91fR5Z/v1w==
-----END CERTIFICATE-----
";

    /// A certificate for `localhost` that the test CA signed.
    const TEST_SERVER_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIBrzCCAVSgAwIBAgIUf3+oqTLa5aTqg/KWojfIxzeZ90AwCgYIKoZIzj0EAwIw
GzEZMBcGA1UEAwwQY2xvYWNpbmEgdGVzdCBDQTAgFw0yNjExMjcyMjE0NTlaGA8y
MTI2MTEwMzIyMTQ1OVowFDESMBAGA1UEAwwJbG9jYWxob3N0MFkwEwYHKoZIzj0C
AQYIKoZIzj0DAQcDQgAE/wv8AY1EZ0On5rrvn9UuOuzUS75+Afe367vEQdXTqFUu
Byd0krwmJtRVLhhcd9EYoMtUqdnxiJjTj6f1cYLz9KN7MHkwFAYDVR0RBA0wC4IJ
bG9jYWxob3N0MAwGA1UdEwEB/wQCMAAwEwYDVR0lBAwwCgYIKwYBBQUHAwEwHQYD
VR0OBBYEFHQJ9hfZQdy9wDZ1jenIV8ZkhGvBMB8GA1UdIwQYMBaAFA9U5sOrKWXg
I5xKB9mbA+Z3BbUJMAoGCCqGSM49BAMCA0kAMEYCIQCcXkOh2e4AQg5urOwOix0v
1PG4iuAs6sefQdP2zntnuAIhAIYbgvVsGegbyBNtmg7tPGCzWKk7Q1+x5pTvBvOY
WKZK
-----END CERTIFICATE-----
";

    /// The PKCS#8 key of the test server certificate, as base64 of its DER
    /// bytes. It was made for these tests and protects nothing.
    const TEST_SERVER_KEY_PKCS8_B64: &str = "MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgDp1V/MdBStrbxHDE\
mR9BjiliUHKUXy1Sb0J30JGnr7KhRANCAAT/C/wBjURnQ6fmuu+f1S467NRLvn4B\
97fru8RB1dOoVS4HJ3SSvCYm1FUuGFx30Rigy1Sp2fGImNOPp/VxgvP0";

    fn provider() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }

    fn write_temp(name: &str, contents: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cloacina-agent-tls-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn no_ca_file_adds_nothing() {
        let roots = ExtraRoots::load(None).unwrap();
        assert!(roots.is_empty());
        assert!(roots.ws_connector().is_none());
    }

    #[test]
    fn a_missing_ca_file_stops_the_start_and_names_the_option() {
        let err = ExtraRoots::load(Some(Path::new("/nonexistent/cloacina/ca.crt"))).unwrap_err();
        let text = format!("{err:#}");
        assert!(text.contains("/nonexistent/cloacina/ca.crt"), "{text}");
        assert!(text.contains("CLOACINA_AGENT_CA_FILE"), "{text}");
    }

    #[test]
    fn a_ca_file_with_no_certificate_stops_the_start() {
        let path = write_temp("empty.crt", b"not a certificate\n");
        let err = ExtraRoots::load(Some(&path)).unwrap_err();
        assert!(
            format!("{err:#}").contains("holds no certificate"),
            "{err:#}"
        );
    }

    #[test]
    fn a_ca_file_is_read_and_both_clients_build() {
        provider();
        let path = write_temp("ca.crt", TEST_CA_PEM.as_bytes());
        let roots = ExtraRoots::load(Some(&path)).unwrap();
        assert_eq!(roots.len(), 1);
        roots
            .http_client()
            .expect("the HTTP client builds with the CA");
        let native = ExtraRoots::default().ws_roots().len();
        assert_eq!(
            roots.ws_roots().len(),
            native + 1,
            "the CA is added to the native roots"
        );
        assert!(roots.ws_connector().is_some());
    }

    /// A TLS server on 127.0.0.1 with the test server certificate. Answers
    /// each connection with one HTTP 200.
    async fn tls_server() -> u16 {
        let cert = CertificateDer::from_pem_slice(TEST_SERVER_PEM.as_bytes()).unwrap();
        let key_der = base64::engine::general_purpose::STANDARD
            .decode(TEST_SERVER_KEY_PKCS8_B64)
            .unwrap();
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der));
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            loop {
                let Ok((socket, _)) = listener.accept().await else {
                    return;
                };
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    let Ok(mut tls) = acceptor.accept(socket).await else {
                        return;
                    };
                    let mut buf = [0u8; 1024];
                    let _ = tls.read(&mut buf).await;
                    let _ = tls
                        .write_all(
                            b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\nok",
                        )
                        .await;
                    let _ = tls.shutdown().await;
                });
            }
        });
        port
    }

    /// The WebSocket connector's configuration trusts a server that the CA
    /// signed, and the default roots do not.
    #[tokio::test]
    async fn the_ws_tls_config_trusts_the_ca() {
        provider();
        let port = tls_server().await;
        let name = ServerName::try_from("localhost").unwrap();

        let path = write_temp("ca-ws.crt", TEST_CA_PEM.as_bytes());
        let config = ExtraRoots::load(Some(&path))
            .unwrap()
            .ws_client_config()
            .unwrap();
        let socket = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        tokio_rustls::TlsConnector::from(config)
            .connect(name.clone(), socket)
            .await
            .expect("the handshake succeeds with the CA");

        let without = Arc::new(
            rustls::ClientConfig::builder()
                .with_root_certificates(ExtraRoots::default().ws_roots())
                .with_no_client_auth(),
        );
        let socket = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        assert!(
            tokio_rustls::TlsConnector::from(without)
                .connect(name, socket)
                .await
                .is_err(),
            "the handshake fails without the CA"
        );
    }

    /// The HTTP client trusts a server that the CA signed, and a client with
    /// no CA file does not. Linux only, the platform of the agent image: the
    /// macOS verifier refuses a server certificate valid for more than 825
    /// days, and the test certificate is valid for 100 years.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn the_http_client_trusts_the_ca() {
        provider();
        let port = tls_server().await;
        let url = format!("https://localhost:{port}/health");

        let path = write_temp("ca-http.crt", TEST_CA_PEM.as_bytes());
        let with = ExtraRoots::load(Some(&path))
            .unwrap()
            .http_client()
            .unwrap();
        let resp = with
            .get(&url)
            .send()
            .await
            .expect("the request succeeds with the CA");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);

        let without = ExtraRoots::default().http_client().unwrap();
        assert!(
            without.get(&url).send().await.is_err(),
            "the request fails without the CA"
        );
    }
}
