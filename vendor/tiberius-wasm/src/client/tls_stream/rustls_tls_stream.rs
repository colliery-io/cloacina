//! FORK DELTA ([[WEIR-A-0041]] / weir-tiberius): the `rustls` backend rebuilt
//! directly on rustls 0.23 with the pure-Rust `rustls-rustcrypto` provider and
//! compiled-in webpki roots. The stock backend (tokio-rustls 0.24 + compat
//! shims + rustls-native-certs) cannot build for wasm32-wasip2.
//!
//! rustls is driven by hand here rather than through futures-rustls: its
//! handshake future drains `wants_read()` and only exits that loop via
//! `Poll::Pending` — but this crate's transports may be *blocking* streams
//! presented as always-`Ready` (the weir wasm guest's SyncSock), where the
//! post-handshake drain blocks forever waiting for bytes the server will never
//! send. The handshake loop below terminates on `!is_handshaking()` instead,
//! which is correct for both blocking and genuinely-async transports.

use crate::{
    client::{config::Config, TrustConfig},
    error::IoErrorKind,
    Error,
};
use futures_util::io::{AsyncRead, AsyncWrite};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, RootCertStore, SignatureScheme};
use rustls_pki_types::{pem::PemObject, CertificateDer, ServerName, UnixTime};
use std::{
    fs,
    io::{self, Read as _, Write as _},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tracing::{event, Level};

impl From<rustls::Error> for Error {
    fn from(e: rustls::Error) -> Self {
        crate::Error::Tls(e.to_string())
    }
}

/// `TrustConfig::TrustAll`: encrypt without verification — explicit opt-in only.
#[derive(Debug)]
struct NoCertVerifier(Arc<rustls::crypto::CryptoProvider>);

impl ServerCertVerifier for NoCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn get_server_name(config: &Config) -> crate::Result<ServerName<'static>> {
    match (
        ServerName::try_from(config.get_host().to_string()),
        &config.trust,
    ) {
        (Ok(sn), _) => Ok(sn),
        (Err(_), TrustConfig::TrustAll) => {
            Ok(ServerName::try_from("placeholder.domain.com").unwrap())
        }
        (Err(e), _) => Err(crate::Error::Tls(e.to_string())),
    }
}

/// Parse every certificate in a PEM bundle into `store`.
fn add_pem_bundle(store: &mut RootCertStore, pem: &[u8], what: &str) -> crate::Result<()> {
    let mut added = 0usize;
    for cert in CertificateDer::pem_slice_iter(pem) {
        let cert = cert.map_err(|e| Error::Io {
            kind: IoErrorKind::InvalidData,
            message: format!("{what}: bad PEM: {e:?}"),
        })?;
        store.add(cert)?;
        added += 1;
    }
    if added == 0 {
        return Err(Error::Io {
            kind: IoErrorKind::InvalidInput,
            message: format!("{what}: no certificates found"),
        });
    }
    Ok(())
}

/// Bridge one `poll_*` call into a `std::io` call: `Pending` ⇢ `WouldBlock`.
struct SyncReadAdapter<'a, 'b, T> {
    io: &'a mut T,
    cx: &'a mut Context<'b>,
}

impl<T: AsyncRead + Unpin> io::Read for SyncReadAdapter<'_, '_, T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match Pin::new(&mut *self.io).poll_read(self.cx, buf) {
            Poll::Ready(r) => r,
            Poll::Pending => Err(io::ErrorKind::WouldBlock.into()),
        }
    }
}

struct SyncWriteAdapter<'a, 'b, T> {
    io: &'a mut T,
    cx: &'a mut Context<'b>,
}

impl<T: AsyncWrite + Unpin> io::Write for SyncWriteAdapter<'_, '_, T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match Pin::new(&mut *self.io).poll_write(self.cx, buf) {
            Poll::Ready(r) => r,
            Poll::Pending => Err(io::ErrorKind::WouldBlock.into()),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match Pin::new(&mut *self.io).poll_flush(self.cx) {
            Poll::Ready(r) => r,
            Poll::Pending => Err(io::ErrorKind::WouldBlock.into()),
        }
    }
}

pub(crate) struct TlsStream<S: AsyncRead + AsyncWrite + Unpin + Send> {
    io: S,
    conn: ClientConnection,
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send> TlsStream<S> {
    pub(super) async fn new(config: &Config, stream: S) -> crate::Result<Self> {
        event!(Level::INFO, "Performing a TLS handshake");

        let provider = Arc::new(rustls_rustcrypto::provider());
        let builder = ClientConfig::builder_with_provider(provider.clone())
            .with_protocol_versions(rustls::DEFAULT_VERSIONS)
            .map_err(|e| crate::Error::Tls(e.to_string()))?;

        let client_config = match &config.trust {
            // FORK DELTA: inline-PEM trust — the only usable CA path for wasm
            // guests (no filesystem).
            TrustConfig::CaCertificatePem(pem) => {
                let mut store = RootCertStore::empty();
                add_pem_bundle(&mut store, pem.as_bytes(), "trust_cert_pem")?;
                builder.with_root_certificates(store).with_no_client_auth()
            }
            TrustConfig::CaCertificateLocation(path) => {
                let buf = fs::read(path).map_err(|_| Error::Io {
                    kind: IoErrorKind::InvalidData,
                    message: "Could not read provided CA certificate!".to_string(),
                })?;
                let mut store = RootCertStore::empty();
                match path.extension() {
                    Some(ext)
                        if ext.to_ascii_lowercase() == "pem"
                            || ext.to_ascii_lowercase() == "crt" =>
                    {
                        add_pem_bundle(&mut store, &buf, "trust_cert_ca")?;
                    }
                    Some(ext) if ext.to_ascii_lowercase() == "der" => {
                        store.add(CertificateDer::from(buf))?;
                    }
                    Some(_) | None => {
                        return Err(Error::Io {
                            kind: IoErrorKind::InvalidInput,
                            message: "Provided CA certificate with unsupported file-extension! \
                                      Supported types are pem, crt and der."
                                .to_string(),
                        })
                    }
                }
                builder.with_root_certificates(store).with_no_client_auth()
            }
            TrustConfig::TrustAll => {
                event!(
                    Level::WARN,
                    "Trusting the server certificate without validation."
                );
                builder
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(NoCertVerifier(provider)))
                    .with_no_client_auth()
            }
            TrustConfig::Default => {
                // FORK DELTA: compiled-in webpki roots — there is no platform
                // trust store on wasm32-wasip2.
                event!(Level::INFO, "Using webpki trust roots.");
                let mut store = RootCertStore::empty();
                store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
                builder.with_root_certificates(store).with_no_client_auth()
            }
        };

        let conn = ClientConnection::new(Arc::new(client_config), get_server_name(config)?)?;
        let mut tls = TlsStream { io: stream, conn };

        futures_util::future::poll_fn(|cx| tls.poll_handshake(cx))
            .await
            .map_err(|e| crate::Error::Tls(e.to_string()))?;

        event!(Level::INFO, "TLS handshake successful");
        Ok(tls)
    }

    pub(crate) fn get_mut(&mut self) -> &mut S {
        &mut self.io
    }

    /// Drive the handshake to completion. Terminates on `!is_handshaking()` —
    /// deliberately NOT on a `wants_read()` drain, which never ends on an
    /// always-ready blocking transport.
    fn poll_handshake(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        loop {
            while self.conn.wants_write() {
                let mut w = SyncWriteAdapter {
                    io: &mut self.io,
                    cx,
                };
                match self.conn.write_tls(&mut w) {
                    Ok(_) => {}
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Poll::Pending,
                    Err(e) => return Poll::Ready(Err(e)),
                }
            }
            match Pin::new(&mut self.io).poll_flush(cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }

            if !self.conn.is_handshaking() {
                return Poll::Ready(Ok(()));
            }

            let mut r = SyncReadAdapter {
                io: &mut self.io,
                cx,
            };
            match self.conn.read_tls(&mut r) {
                Ok(0) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "tls handshake eof",
                    )));
                }
                Ok(_) => {
                    if let Err(e) = self.conn.process_new_packets() {
                        // Push out any alert describing the failure before erroring.
                        let mut w = SyncWriteAdapter {
                            io: &mut self.io,
                            cx,
                        };
                        let _ = self.conn.write_tls(&mut w);
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            e.to_string(),
                        )));
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Poll::Pending,
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send> AsyncRead for TlsStream<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = Pin::get_mut(self);
        loop {
            // Serve buffered plaintext first.
            match this.conn.reader().read(buf) {
                Ok(n) => return Poll::Ready(Ok(n)),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {} // need more TLS data
                Err(e) => return Poll::Ready(Err(e)),
            }
            let mut r = SyncReadAdapter {
                io: &mut this.io,
                cx,
            };
            match this.conn.read_tls(&mut r) {
                Ok(0) => return Poll::Ready(Ok(0)), // EOF
                Ok(_) => {
                    if let Err(e) = this.conn.process_new_packets() {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            e.to_string(),
                        )));
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Poll::Pending,
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send> AsyncWrite for TlsStream<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = Pin::get_mut(self);
        let n = this.conn.writer().write(buf)?;
        while this.conn.wants_write() {
            let mut w = SyncWriteAdapter {
                io: &mut this.io,
                cx,
            };
            match this.conn.write_tls(&mut w) {
                Ok(_) => {}
                // Bytes are buffered in rustls; a later flush pushes the rest.
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
        Poll::Ready(Ok(n))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = Pin::get_mut(self);
        while this.conn.wants_write() {
            let mut w = SyncWriteAdapter {
                io: &mut this.io,
                cx,
            };
            match this.conn.write_tls(&mut w) {
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Poll::Pending,
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
        Pin::new(&mut this.io).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = Pin::get_mut(self);
        this.conn.send_close_notify();
        while this.conn.wants_write() {
            let mut w = SyncWriteAdapter {
                io: &mut this.io,
                cx,
            };
            match this.conn.write_tls(&mut w) {
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Poll::Pending,
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
        Pin::new(&mut this.io).poll_close(cx)
    }
}
