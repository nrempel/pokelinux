//! Read-only enrollment loading from one private, descriptor-relative directory.
use crate::tls::TlsClient;
use rustix::fs::{Mode, OFlags, open, openat};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use serde::Deserialize;
use std::{
    fs::File,
    io::{self, Read},
    net::SocketAddr,
    os::unix::fs::MetadataExt,
    path::Path,
};
use zeroize::Zeroizing;

pub struct Enrollment {
    address: SocketAddr,
    client: TlsClient,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Endpoint {
    endpoint: SocketAddr,
    server_name: String,
    server_sha256: String,
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid or unsafe enrollment")
}
fn read_file(
    directory: &File,
    name: &str,
    limit: usize,
    private: bool,
) -> io::Result<Zeroizing<Vec<u8>>> {
    let file = File::from(openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
    )?);
    let metadata = file.metadata()?;
    let forbidden = if private { 0o077 } else { 0o022 };
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & forbidden != 0
        || metadata.nlink() != 1
    {
        return Err(invalid());
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take((limit + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() > limit {
        return Err(invalid());
    }
    Ok(bytes)
}
fn certificates(bytes: &[u8]) -> io::Result<Vec<CertificateDer<'static>>> {
    CertificateDer::pem_slice_iter(bytes)
        .map(|cert| cert.map_err(|_| invalid()))
        .collect()
}
impl Enrollment {
    pub fn load(path: &Path) -> io::Result<Self> {
        let directory = File::from(open(
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )?);
        let metadata = directory.metadata()?;
        if metadata.uid() != rustix::process::geteuid().as_raw() || metadata.mode() & 0o777 != 0o700
        {
            return Err(invalid());
        }
        let endpoint: Endpoint =
            serde_json::from_slice(&read_file(&directory, "endpoint.json", 4096, false)?)
                .map_err(|_| invalid())?;
        if !endpoint.endpoint.ip().is_loopback()
            || endpoint.endpoint.port() == 0
            || endpoint.server_sha256.len() != 64
            || !endpoint
                .server_sha256
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(invalid());
        }
        let mut pin = [0; 32];
        for (index, byte) in pin.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&endpoint.server_sha256[index * 2..index * 2 + 2], 16)
                .map_err(|_| invalid())?;
        }
        let roots = certificates(&read_file(&directory, "ca.pem", 65536, false)?)?;
        let chain = certificates(&read_file(&directory, "client.pem", 65536, false)?)?;
        let raw_key = read_file(&directory, "client.key", 65536, true)?;
        let key = PrivateKeyDer::from_pem_slice(&raw_key).map_err(|_| invalid())?;
        let client = TlsClient::new(roots, chain, key, endpoint.server_name, pin)?;
        Ok(Self {
            address: endpoint.endpoint,
            client,
        })
    }
    pub fn into_parts(self) -> (SocketAddr, TlsClient) {
        (self.address, self.client)
    }
    pub fn address(&self) -> SocketAddr {
        self.address
    }
    pub fn client(&self) -> &TlsClient {
        &self.client
    }
}
