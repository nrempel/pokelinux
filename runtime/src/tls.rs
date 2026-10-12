//! One authenticated exchange. No retries, fallback trust or early data.
use crate::watch::{Snapshot, WatchQuery, decode_snapshot};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Read, Write},
    net::{Shutdown, SocketAddr, TcpStream},
    sync::{Arc, Mutex},
    time::Duration,
};

const MAX_REQUEST: usize = 4096;
const MAX_RESPONSE: usize = 65_536;
#[derive(Debug)]
pub enum ExchangeError {
    /// No application write was attempted on this connection.
    NotSent(io::Error),
    /// A write was attempted. Delivery/execution cannot be inferred from I/O.
    Unknown(io::Error),
}
#[derive(Default)]
enum WatchState {
    #[default]
    Ready,
    Running(Option<TcpStream>),
    Cancelled,
}
/// Observation-only lifetime cancellation. Once canceled, this token cannot be
/// reused. Commands never accept it. One token owns at most one active exchange.
#[derive(Default)]
pub struct WatchCancel(Mutex<WatchState>);
impl WatchCancel {
    pub fn cancel(&self) {
        let previous = {
            let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
            std::mem::replace(&mut *state, WatchState::Cancelled)
        };
        if let WatchState::Running(Some(socket)) = previous {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }
    fn begin(&self) -> io::Result<WatchRegistration<'_>> {
        let mut state = self.0.lock().map_err(|_| cancelled())?;
        match &*state {
            WatchState::Ready => {
                *state = WatchState::Running(None);
                Ok(WatchRegistration(self))
            }
            WatchState::Cancelled => Err(cancelled()),
            WatchState::Running(_) => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "watch already active",
            )),
        }
    }
}
fn cancelled() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "watch cancelled")
}
struct WatchRegistration<'a>(&'a WatchCancel);
impl WatchRegistration<'_> {
    fn attach(&self, raw: &TcpStream) -> io::Result<()> {
        let mut state = self.0.0.lock().map_err(|_| cancelled())?;
        match &mut *state {
            WatchState::Running(socket @ None) => {
                *socket = Some(raw.try_clone()?);
                Ok(())
            }
            _ => Err(cancelled()),
        }
    }
    fn check(&self) -> io::Result<()> {
        let state = self.0.0.lock().map_err(|_| cancelled())?;
        if matches!(&*state, WatchState::Running(_)) {
            Ok(())
        } else {
            Err(cancelled())
        }
    }
}
impl Drop for WatchRegistration<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.0.lock()
            && matches!(&*state, WatchState::Running(_))
        {
            *state = WatchState::Ready;
        }
    }
}
#[derive(Clone)]
pub struct TlsClient {
    config: Arc<ClientConfig>,
    server_name: ServerName<'static>,
    pin: [u8; 32],
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
// Linux CLOCK_BOOTTIME includes suspend. Socket timeouts alone do not: checks
// after I/O also reject data returned after resume or scheduler delay. This is
// an acceptance deadline, not a guarantee that a blocked thread wakes in suspend.
fn now() -> Duration {
    let value = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    Duration::new(value.tv_sec as u64, value.tv_nsec as u32)
}
fn remaining_at(deadline: Duration, observed: Duration) -> io::Result<Duration> {
    deadline
        .checked_sub(observed)
        .filter(|d| !d.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "exchange deadline expired"))
}
fn remaining(deadline: Duration) -> io::Result<Duration> {
    remaining_at(deadline, now())
}
/// All rustls I/O, including complete_io's internal loops, uses this wrapper.
struct DeadlineSocket {
    raw: TcpStream,
    deadline: Duration,
}
impl Read for DeadlineSocket {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.raw.set_read_timeout(Some(remaining(self.deadline)?))?;
        let result = self.raw.read(out);
        remaining(self.deadline)?;
        result
    }
}
impl Write for DeadlineSocket {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.raw
            .set_write_timeout(Some(remaining(self.deadline)?))?;
        let result = self.raw.write(bytes);
        remaining(self.deadline)?;
        result
    }
    fn flush(&mut self) -> io::Result<()> {
        remaining(self.deadline)?;
        self.raw.flush()
    }
}
impl TlsClient {
    /// Enrollment loading and file permissions are separate. This constructor
    /// owns an immutable trust snapshot and never consults system roots.
    pub fn new(
        roots: Vec<CertificateDer<'static>>,
        chain: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
        name: String,
        pin: [u8; 32],
    ) -> io::Result<Self> {
        if roots.is_empty() || chain.is_empty() {
            return Err(invalid("enrollment certificates required"));
        }
        let mut store = RootCertStore::empty();
        for cert in roots {
            store
                .add(cert)
                .map_err(|_| invalid("invalid enrollment root"))?;
        }
        let mut config =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_protocol_versions(&[&rustls::version::TLS13])
                .map_err(|_| invalid("TLS configuration failed"))?
                .with_root_certificates(store)
                .with_client_auth_cert(chain, key)
                .map_err(|_| invalid("invalid client identity"))?;
        config.resumption = rustls::client::Resumption::disabled();
        config.enable_early_data = false;
        let server_name =
            ServerName::try_from(name).map_err(|_| invalid("invalid enrolled server name"))?;
        Ok(Self {
            config: Arc::new(config),
            server_name,
            pin,
        })
    }
    /// The caller must have durable admission before exchanging a command.
    /// The response is still untrusted protocol data until typed validation.
    pub fn exchange(
        &self,
        address: SocketAddr,
        body: &[u8],
        budget: Duration,
    ) -> Result<Vec<u8>, ExchangeError> {
        self.exchange_decoded(address, body, budget, |bytes| Ok(bytes.to_vec()))
    }
    /// Validation shares the transport acceptance deadline. The callback must
    /// only decode bounded data; it must not dispatch or mutate service state.
    pub(crate) fn exchange_decoded<T>(
        &self,
        address: SocketAddr,
        body: &[u8],
        budget: Duration,
        decode: impl FnOnce(&[u8]) -> io::Result<T>,
    ) -> Result<T, ExchangeError> {
        self.exchange_with_budget(
            address,
            body,
            budget.min(Duration::from_secs(5)),
            None,
            decode,
        )
    }
    /// Read-only observation has its own bounded budget. A typed query prevents
    /// callers from sending a command through the longer observation path.
    /// No retry is performed; invalid or late snapshots are discarded.
    pub fn watch(
        &self,
        address: SocketAddr,
        query: &WatchQuery,
        cancel: &WatchCancel,
    ) -> io::Result<Snapshot> {
        let budget = if query.wait_ms() <= 1000 {
            Duration::from_secs(5)
        } else {
            Duration::from_secs(30)
        };
        self.exchange_with_budget(address, &query.encode(), budget, Some(cancel), |bytes| {
            decode_snapshot(bytes, query.id()).map_err(invalid)
        })
        .map_err(|error| match error {
            ExchangeError::NotSent(error) | ExchangeError::Unknown(error) => error,
        })
    }
    fn exchange_with_budget<T>(
        &self,
        address: SocketAddr,
        body: &[u8],
        budget: Duration,
        cancel: Option<&WatchCancel>,
        decode: impl FnOnce(&[u8]) -> io::Result<T>,
    ) -> Result<T, ExchangeError> {
        let registration = cancel
            .map(WatchCancel::begin)
            .transpose()
            .map_err(ExchangeError::NotSent)?;
        if body.is_empty() || body.len() > MAX_REQUEST {
            return Err(ExchangeError::NotSent(invalid("invalid request length")));
        }
        let deadline = now() + budget;
        let mut connect_budget = remaining(deadline).map_err(ExchangeError::NotSent)?;
        // Before a socket exists, cancellation cannot shut it down. Bound that
        // watch-only connect phase to one second; commands retain their budget.
        if registration.is_some() {
            connect_budget = connect_budget.min(Duration::from_secs(1));
        }
        let raw =
            TcpStream::connect_timeout(&address, connect_budget).map_err(ExchangeError::NotSent)?;
        if let Some(registration) = &registration {
            registration.attach(&raw).map_err(ExchangeError::NotSent)?;
        }
        let mut socket = DeadlineSocket { raw, deadline };
        let mut conn = ClientConnection::new(self.config.clone(), self.server_name.clone())
            .map_err(|_| ExchangeError::NotSent(invalid("TLS connection failed")))?;
        while conn.is_handshaking() {
            conn.complete_io(&mut socket)
                .map_err(ExchangeError::NotSent)?;
        }
        remaining(deadline).map_err(ExchangeError::NotSent)?;
        let cert = conn
            .peer_certificates()
            .and_then(|chain| chain.first())
            .ok_or_else(|| ExchangeError::NotSent(invalid("missing server certificate")))?;
        let actual: [u8; 32] = Sha256::digest(cert.as_ref()).into();
        // The fingerprint is public. Chain and hostname verification above are
        // still mandatory; matching the pin never bypasses either check.
        if actual != self.pin {
            return Err(ExchangeError::NotSent(invalid(
                "server certificate pin mismatch",
            )));
        }
        if let Some(registration) = &registration {
            registration.check().map_err(ExchangeError::NotSent)?;
        }
        let mut stream = StreamOwned::new(conn, socket);
        // From the first write attempt onwards, every failure is ambiguous.
        let result = (|| {
            stream.write_all(&(body.len() as u32).to_be_bytes())?;
            stream.write_all(body)?;
            stream.flush()?;
            let mut header = [0; 4];
            stream.read_exact(&mut header)?;
            let size = u32::from_be_bytes(header) as usize;
            if !(1..=MAX_RESPONSE).contains(&size) {
                return Err(invalid("invalid response length"));
            }
            let mut response = vec![0; size];
            stream.read_exact(&mut response)?;
            remaining(deadline)?;
            if let Some(registration) = &registration {
                registration.check()?;
            }
            let decoded = decode(&response)?;
            remaining(deadline)?;
            if let Some(registration) = &registration {
                registration.check()?;
            }
            Ok(decoded)
        })();
        result.map_err(ExchangeError::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deadline_rejects_exact_expiry_and_resume_after_expiry() {
        let deadline = Duration::from_secs(20);
        assert_eq!(
            remaining_at(deadline, Duration::from_secs(19)).unwrap(),
            Duration::from_secs(1)
        );
        assert!(remaining_at(deadline, deadline).is_err());
        assert!(remaining_at(deadline, Duration::from_secs(120)).is_err());
        assert!(!now().is_zero());
    }
}
