//! Safe, nonblocking protocol adapter to the existing sole Rust audio owner.
use crate::audio::{Event, Intent, MuteIntent, PreviewIntent, Provider, Route, State};
use pokelinux_audio::client::{Client, Event as ClientEvent, SendOutcome, SubmitError};
use rustix::net::{self, AddressFamily, SocketAddrUnix, SocketFlags, SocketType};
use std::{
    io,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
};

pub struct Owner {
    path: PathBuf,
    owner_uid: u32,
    client: Client,
    incarnation: String,
    pending: Option<Operation>,
    awaiting_application: bool,
}
enum Operation {
    Inspect,
    Select {
        id: u64,
        route: String,
    },
    Preview {
        id: u64,
        route: Option<String>,
    },
    Mute {
        id: u64,
        generation: u64,
        muted: bool,
    },
}
fn connect(path: &Path) -> io::Result<UnixStream> {
    if !path.is_absolute() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let fd = net::socket_with(
        AddressFamily::UNIX,
        SocketType::STREAM,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )?;
    // AF_UNIX backlog pressure returns without waiting; no queued route exists.
    net::connect(&fd, &SocketAddrUnix::new(path)?)?;
    Ok(UnixStream::from(fd))
}
impl Owner {
    pub fn connect(path: &Path, owner_uid: u32) -> io::Result<Self> {
        Self::from_stream(connect(path)?, path, owner_uid)
    }
    /// Trusted setup may retain a socket alias for isolated syscall tests. The
    /// client still verifies actual SO_PEERCRED and owns normal EOF/close policy.
    pub fn from_stream(stream: UnixStream, path: &Path, owner_uid: u32) -> io::Result<Self> {
        if !path.is_absolute() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let client = Client::from_stream(stream, owner_uid)?;
        let incarnation = format!("audio-{:016x}", client.snapshot().incarnation);
        Ok(Self {
            path: path.into(),
            owner_uid,
            client,
            incarnation,
            pending: None,
            awaiting_application: false,
        })
    }
    fn sent(
        &mut self,
        result: Result<SendOutcome, SubmitError>,
        operation: Operation,
    ) -> io::Result<bool> {
        match result {
            Ok(SendOutcome::Sent) => {
                if matches!(
                    operation,
                    Operation::Select { .. } | Operation::Preview { .. }
                ) {
                    self.awaiting_application = true;
                }
                self.pending = Some(operation);
                Ok(true)
            }
            Ok(SendOutcome::WouldBlock) | Err(SubmitError::Busy) => Ok(false),
            Err(error) => Err(io::Error::other(format!("audio submission: {error:?}"))),
        }
    }
}
impl Provider for Owner {
    fn incarnation(&self) -> &str {
        &self.incarnation
    }
    fn state(&self) -> State {
        let snapshot = self.client.snapshot();
        State {
            connected: snapshot.connected,
            available: snapshot.connected
                && !snapshot.uncertain
                && snapshot.observation.as_ref().is_some_and(|o| o.available),
            pending: snapshot.in_flight
                || matches!(
                    snapshot.preview,
                    crate::audio::PreviewState::Starting | crate::audio::PreviewState::Retiring
                )
                || (self.awaiting_application
                    && snapshot.admitted.is_some()
                    && !snapshot.policy_settled
                    && snapshot.applied.is_none()),
            requested: snapshot.admitted.as_ref().map(|a| a.route.clone()),
            applied: snapshot.applied.as_ref().map(|a| a.route.clone()),
            policy_settled: snapshot.policy_settled,
            preview: snapshot.preview,
            preview_generation: snapshot.preview_generation,
            preview_muted: snapshot.preview_muted,
        }
    }
    fn routes(&self) -> Vec<Route> {
        self.client
            .snapshot()
            .media_routes
            .into_iter()
            .map(|r| Route {
                id: r.id,
                label: r.label,
            })
            .collect()
    }
    fn submit(&mut self, intent: &Intent) -> io::Result<bool> {
        let result = self.client.select(&intent.route, intent.until_ns);
        self.sent(
            result,
            Operation::Select {
                id: intent.id,
                route: intent.route.clone(),
            },
        )
    }
    fn play(&mut self, preview: &PreviewIntent) -> io::Result<bool> {
        let intent = &preview.route_intent;
        let result = self
            .client
            .preview_play(&intent.route, preview.level, intent.until_ns);
        let sent = self.sent(
            result,
            Operation::Preview {
                id: intent.id,
                route: Some(intent.route.clone()),
            },
        );
        // Only actual dispatch logs: Unsent backpressure can be polled again.
        if matches!(&sent, Ok(true)) {
            eprintln!(
                "preview submit: effective_forwarded_until_ns={} sent=true",
                intent.until_ns
            );
        }
        sent
    }
    fn mute_preview(&mut self, intent: &MuteIntent) -> io::Result<bool> {
        let result = self
            .client
            .preview_mute(intent.generation, intent.muted, intent.until_ns);
        self.sent(
            result,
            Operation::Mute {
                id: intent.id,
                generation: intent.generation,
                muted: intent.muted,
            },
        )
    }
    fn stop_preview(&mut self, id: u64, until_ns: u64) -> io::Result<bool> {
        let result = self.client.preview_stop(until_ns);
        self.sent(result, Operation::Preview { id, route: None })
    }
    fn inspect(&mut self, until_ns: u64) -> io::Result<bool> {
        let result = self.client.inspect(until_ns);
        self.sent(result, Operation::Inspect)
    }
    fn poll(&mut self) -> Option<Event> {
        let event = self.client.poll()?;
        let pending = self.pending.take();
        let snapshot = self.client.snapshot();
        if snapshot.policy_settled
            || snapshot.applied.is_some()
            || snapshot.admitted.is_none()
            || matches!(&event, ClientEvent::Rejected { .. })
        {
            self.awaiting_application = false;
        }
        Some(match (event, pending) {
            (ClientEvent::Updated(snapshot), Some(Operation::Select { id, route }))
                if snapshot.admitted.as_ref().is_some_and(|a| a.route == route) =>
            {
                Event::Selected {
                    id,
                    state: self.state(),
                }
            }
            (ClientEvent::Updated(snapshot), Some(Operation::Preview { id, route }))
                if route.as_ref().is_none_or(|route| {
                    snapshot
                        .admitted
                        .as_ref()
                        .is_some_and(|a| &a.route == route)
                }) =>
            {
                Event::Preview {
                    id,
                    state: self.state(),
                }
            }
            (
                ClientEvent::Updated(snapshot),
                Some(Operation::Mute {
                    id,
                    generation,
                    muted,
                }),
            ) if snapshot.preview == crate::audio::PreviewState::Playing
                && snapshot.preview_generation == Some(generation)
                && snapshot.preview_muted == Some(muted) =>
            {
                Event::Muted {
                    id,
                    state: self.state(),
                }
            }
            (
                ClientEvent::Rejected { .. },
                Some(
                    Operation::Select { id, .. }
                    | Operation::Preview { id, .. }
                    | Operation::Mute { id, .. },
                ),
            ) => Event::Rejected { id },
            (ClientEvent::Updated(_), Some(Operation::Inspect)) => Event::Observed,
            _ => {
                self.revoke();
                Event::Unavailable
            }
        })
    }
    fn revoke(&mut self) {
        self.client.revoke();
        self.pending = None;
        self.awaiting_application = false;
    }
    fn reconnect(&mut self) -> io::Result<()> {
        if self.client.snapshot().connected {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        let client = Client::from_stream(connect(&self.path)?, self.owner_uid)?;
        self.incarnation = format!("audio-{:016x}", client.snapshot().incarnation);
        self.client = client;
        self.pending = None;
        self.awaiting_application = false;
        Ok(())
    }
}
