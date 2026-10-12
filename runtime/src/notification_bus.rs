//! Async D-Bus boundary. The notification loop must retain an unresolved Notify
//! after revoking adoption, so a late ID can still be closed within the owner's
//! bounded cleanup window. A terminal owner timeout aborts the entire connection;
//! it does not establish remote removal. Cleanup stays on the original connection.
use crate::notification::NotifyRequest;
use async_io::{Async, Timer};
use futures_lite::future;
use std::{
    collections::HashMap,
    ffi::OsString,
    net::Shutdown,
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        net::UnixStream,
    },
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use zbus::{
    Address, Connection,
    address::transport::{Transport, UnixSocket},
    connection::Builder,
    zvariant::Value,
};
const PATH: &str = "/org/freedesktop/Notifications";
const INTERFACE: &str = "org.freedesktop.Notifications";
const SETUP_TIMEOUT: Duration = Duration::from_millis(1500);
const CLOSE_TIMEOUT: Duration = Duration::from_millis(1500);

/// Cancelling a partially written D-Bus message invalidates the whole stream.
/// Raw shutdown takes no zbus write lock and wakes all connection clones.
struct AbortOnDrop(Option<Arc<UnixStream>>);
impl AbortOnDrop {
    fn new(socket: &Arc<UnixStream>) -> Self {
        Self(Some(socket.clone()))
    }
    fn disarm(&mut self) {
        self.0 = None;
    }
}
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        if let Some(socket) = &self.0 {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }
}

pub struct NotificationBus {
    connection: Connection,
    socket: Arc<UnixStream>,
}
pub struct Delivery {
    connection: Connection,
    socket: Arc<UnixStream>,
    owner: String,
    id: u32,
    received: zbus::message::Sequence,
}
impl NotificationBus {
    /// End a supervised generation without waiting for a zbus write lock.
    /// All retained clones share this socket; remote delivery stays uncertain.
    pub(crate) fn abort(&self) {
        let _ = self.socket.shutdown(Shutdown::Both);
    }
    pub async fn session() -> zbus::Result<Self> {
        Self::build(Address::session()?).await
    }
    pub async fn connect(address: &str) -> zbus::Result<Self> {
        Self::build(address.try_into()?).await
    }
    async fn build(address: Address) -> zbus::Result<Self> {
        // Race the whole build, including authentication and Hello. Losing
        // build state is dropped rather than leaving a detached blocked thread.
        future::or(
            async {
                // The Omarchy notification service is on the local Unix session
                // bus. Do not execute an autolaunch command or connect over TCP.
                let path = match address.transport() {
                    Transport::Unix(unix) => match unix.path() {
                        UnixSocket::File(path) => path.clone(),
                        UnixSocket::Abstract(name) => {
                            let mut bytes = vec![0];
                            bytes.extend_from_slice(name.as_bytes());
                            PathBuf::from(OsString::from_vec(bytes))
                        }
                        _ => {
                            return Err(zbus::Error::Address(
                                "notification bus needs a connectable Unix address".into(),
                            ));
                        }
                    },
                    _ => {
                        return Err(zbus::Error::Address(
                            "notification bus must be local Unix".into(),
                        ));
                    }
                };
                let stream = Async::<UnixStream>::connect(path).await?.into_inner()?;
                let socket = Arc::new(stream.try_clone()?);
                let mut abort = AbortOnDrop::new(&socket);
                let connection = Builder::async_io_unix_stream(stream)
                    .max_queued(32)
                    .build()
                    .await?;
                if address
                    .guid()
                    .is_some_and(|guid| guid.as_str() != connection.server_guid().as_str())
                {
                    return Err(zbus::Error::Failure(
                        "notification bus GUID mismatch".into(),
                    ));
                }
                abort.disarm();
                Ok(Self { connection, socket })
            },
            async {
                Timer::after(SETUP_TIMEOUT).await;
                Err(zbus::Error::Failure(
                    "notification bus setup timed out".into(),
                ))
            },
        )
        .await
    }
    /// Initial discovery on an already subscribed stream. The snapshot replaces
    /// history before its reply; later changes stay queued. Do not use this to
    /// resynchronize active deliveries without first revoking their authority.
    pub async fn discover(
        &self,
        signals: &mut NotificationSignals,
    ) -> zbus::Result<Option<String>> {
        if !Arc::ptr_eq(&self.socket, &signals.socket) {
            return Err(zbus::Error::Failure(
                "notification subscription belongs to another connection".into(),
            ));
        }
        let mut abort = AbortOnDrop::new(&self.socket);
        let result = future::or(
            async {
                let reply = self
                    .connection
                    .call_method(
                        Some("org.freedesktop.DBus"),
                        "/org/freedesktop/DBus",
                        Some("org.freedesktop.DBus"),
                        "GetNameOwner",
                        &(INTERFACE,),
                    )
                    .await;
                let (message, owner) = match reply {
                    Ok(message) => {
                        let owner: String = message.body().deserialize()?;
                        zbus::names::UniqueName::try_from(owner.as_str())?;
                        (message, Some(owner))
                    }
                    Err(zbus::Error::MethodError(name, _, message))
                        if name.as_str() == "org.freedesktop.DBus.Error.NameHasNoOwner" =>
                    {
                        (message, None)
                    }
                    Err(error) => return Err(error),
                };
                if message.header().sender().map(|name| name.as_str())
                    != Some("org.freedesktop.DBus")
                {
                    return Err(zbus::Error::Failure(
                        "notification owner reply sender mismatch".into(),
                    ));
                }
                // The bus snapshot supersedes these initial transitions. None as the
                // notifier owner also discards old notification signals before startup.
                while signals
                    .before(&self.socket, message.recv_position(), None)
                    .await?
                    .is_some()
                {}
                Ok(owner)
            },
            async {
                Timer::after(SETUP_TIMEOUT).await;
                Err(zbus::Error::Failure(
                    "notification owner discovery timed out".into(),
                ))
            },
        )
        .await;
        if result.is_ok() {
            abort.disarm();
        }
        result
    }
    /// Sends only generic, silent presentation. Actions open Phone; they do not
    /// carry call commands. Poll alongside expiry, policy and shutdown events.
    /// Cancellation or failure aborts the original socket: a partial write
    /// must never be followed by another message on that stream.
    pub async fn notify(&self, request: &NotifyRequest) -> zbus::Result<Delivery> {
        let clock = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
        let now = Duration::new(clock.tv_sec as u64, clock.tv_nsec as u32);
        let timeout = request
            .timeout_at(now)
            .ok_or_else(|| zbus::Error::Failure("notification lease expired before send".into()))?;
        let hints = HashMap::from([
            ("suppress-sound", Value::from(true)),
            ("transient", Value::from(true)),
        ]);
        let mut abort = AbortOnDrop::new(&self.socket);
        let reply = self
            .connection
            .call_method(
                Some(request.owner()),
                PATH,
                Some(INTERFACE),
                "Notify",
                &(
                    "Phone",
                    0u32,
                    "call-start",
                    "Incoming call",
                    "",
                    vec!["default", "Open Phone"],
                    hints,
                    timeout,
                ),
            )
            .await?;
        if reply.header().sender().map(|name| name.as_str()) != Some(request.owner()) {
            return Err(zbus::Error::Failure(
                "notification reply owner mismatch".into(),
            ));
        }
        let id: u32 = reply.body().deserialize()?;
        if id == 0 {
            return Err(zbus::Error::Failure("zero notification ID".into()));
        }
        abort.disarm();
        Ok(Delivery {
            connection: self.connection.clone(),
            socket: self.socket.clone(),
            owner: request.owner().into(),
            id,
            received: reply.recv_position(),
        })
    }
}
impl Delivery {
    pub fn id(&self) -> u32 {
        self.id
    }
    /// Never resolve the well-known name again and never use a new connection.
    /// Consumes the delivery: cleanup is attempted once. Timeout, cancellation
    /// or failure aborts the entire original connection, with remote state
    /// explicitly unknown. No automatic retry is safe after an uncertain send.
    pub async fn close(self) -> zbus::Result<()> {
        let mut abort = AbortOnDrop::new(&self.socket);
        let result = future::or(
            async {
                let reply = self
                    .connection
                    .call_method(
                        Some(self.owner.as_str()),
                        PATH,
                        Some(INTERFACE),
                        "CloseNotification",
                        &(self.id,),
                    )
                    .await?;
                if reply.header().sender().map(|name| name.as_str()) != Some(self.owner.as_str()) {
                    return Err(zbus::Error::Failure(
                        "notification close owner mismatch".into(),
                    ));
                }
                reply.body().deserialize()
            },
            async {
                Timer::after(CLOSE_TIMEOUT).await;
                Err(zbus::Error::Failure(
                    "notification cleanup timed out; remote state unknown".into(),
                ))
            },
        )
        .await;
        if result.is_ok() {
            abort.disarm();
        }
        result
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CloseReason {
    Expired,
    Dismissed,
    Closed,
    Undefined,
}
#[derive(Debug, PartialEq, Eq)]
pub enum NotificationEvent {
    OwnerChanged {
        old_owner: String,
        new_owner: String,
    },
    Closed {
        id: u32,
        reason: CloseReason,
    },
    OpenPhone {
        id: u32,
    },
}
/// Connection-scoped, ordered signal stream. The owner must continue polling
/// while Notify is pending, and reconcile every owner transition before adoption.
pub struct NotificationSignals {
    stream: zbus::MessageStream,
    socket: Arc<UnixStream>,
    buffered: Option<zbus::Message>,
}
impl NotificationBus {
    /// Subscribe before discovering the owner or sending Notify. One ordered,
    /// bounded stream covers bus-owner and notification signals. The decoder
    /// below rejects unrelated paths/interfaces, names, senders and actions.
    pub async fn subscribe(&self) -> zbus::Result<NotificationSignals> {
        let mut abort = AbortOnDrop::new(&self.socket);
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .path_namespace("/org/freedesktop")?
            .build();
        let signals = NotificationSignals {
            stream: zbus::MessageStream::for_match_rule(rule, &self.connection, Some(32)).await?,
            socket: self.socket.clone(),
            buffered: None,
        };
        abort.disarm();
        Ok(signals)
    }
}
impl NotificationSignals {
    pub(crate) fn belongs_to(&self, delivery: &Delivery) -> bool {
        Arc::ptr_eq(&self.socket, &delivery.socket)
    }
    /// Drain events received before this Notify reply. None is a receive-order
    /// barrier, not a timeout. A later signal is retained for next(). Only
    /// receipts from this exact connection can supply meaningful ordering.
    pub async fn before_reply(
        &mut self,
        delivery: &Delivery,
    ) -> zbus::Result<Option<NotificationEvent>> {
        self.before(&delivery.socket, delivery.received, Some(&delivery.owner))
            .await
    }
    async fn before(
        &mut self,
        socket: &Arc<UnixStream>,
        received: zbus::message::Sequence,
        owner: Option<&str>,
    ) -> zbus::Result<Option<NotificationEvent>> {
        use ordered_stream::{OrderedStream, PollResult};
        use std::{future::poll_fn, pin::Pin};
        if !Arc::ptr_eq(&self.socket, socket) {
            return Err(zbus::Error::Failure(
                "notification receipt belongs to another connection".into(),
            ));
        }
        loop {
            let message = if let Some(message) = self.buffered.take() {
                message
            } else {
                match poll_fn(|cx| Pin::new(&mut self.stream).poll_next_before(cx, Some(&received)))
                    .await
                {
                    PollResult::Item { data, .. } => data?,
                    PollResult::NoneBefore => return Ok(None),
                    PollResult::Terminated => {
                        return Err(zbus::Error::Failure(
                            "notification signal stream closed".into(),
                        ));
                    }
                }
            };
            if message.recv_position() >= received {
                self.buffered = Some(message);
                return Ok(None);
            }
            if let Some(event) = decode_event(&message, owner)? {
                return Ok(Some(event));
            }
            future::yield_now().await;
        }
    }
    /// Pass the current unique notifier owner, or None before discovery/loss.
    /// Returned delivery IDs still require matching against owned deliveries;
    /// reception never grants call-control authority or launches an application.
    pub async fn next(&mut self, owner: Option<&str>) -> zbus::Result<NotificationEvent> {
        use futures_lite::StreamExt;
        loop {
            let message = if let Some(message) = self.buffered.take() {
                message
            } else {
                self.stream.next().await.ok_or_else(|| {
                    zbus::Error::Failure("notification signal stream closed".into())
                })??
            };
            if let Some(event) = decode_event(&message, owner)? {
                return Ok(event);
            }
            future::yield_now().await;
        }
    }
}
fn decode_event(
    message: &zbus::Message,
    owner: Option<&str>,
) -> zbus::Result<Option<NotificationEvent>> {
    let header = message.header();
    let sender = header.sender().map(|value| value.as_str());
    let path = header.path().map(|value| value.as_str());
    let interface = header.interface().map(|value| value.as_str());
    let member = header.member().map(|value| value.as_str());
    if sender == Some("org.freedesktop.DBus")
        && path == Some("/org/freedesktop/DBus")
        && interface == Some("org.freedesktop.DBus")
        && member == Some("NameOwnerChanged")
    {
        let (name, old_owner, new_owner): (String, String, String) =
            message.body().deserialize()?;
        if name == INTERFACE {
            for value in [&old_owner, &new_owner] {
                if !value.is_empty() {
                    zbus::names::UniqueName::try_from(value.as_str())?;
                }
            }
            return Ok(Some(NotificationEvent::OwnerChanged {
                old_owner,
                new_owner,
            }));
        }
    } else if owner.is_some()
        && sender == owner
        && path == Some(PATH)
        && interface == Some(INTERFACE)
    {
        match member {
            Some("NotificationClosed") => {
                let (id, reason): (u32, u32) = message.body().deserialize()?;
                let reason = match reason {
                    1 => CloseReason::Expired,
                    2 => CloseReason::Dismissed,
                    3 => CloseReason::Closed,
                    4 => CloseReason::Undefined,
                    _ => {
                        return Err(zbus::Error::Failure(
                            "invalid notification close reason".into(),
                        ));
                    }
                };
                if id != 0 {
                    return Ok(Some(NotificationEvent::Closed { id, reason }));
                }
            }
            Some("ActionInvoked") => {
                let (id, action): (u32, String) = message.body().deserialize()?;
                if id != 0 && action == "default" {
                    return Ok(Some(NotificationEvent::OpenPhone { id }));
                }
            }
            _ => (),
        }
    }
    Ok(None)
}

/// Navigation only: activate the fixed installed Phone app, without arguments.
/// The caller must check current session/presentation policy before invoking.
/// Use a dedicated connection so cancelling or timing out app startup cannot
/// invalidate the notification worker's pending delivery or signal stream.
/// This operation never retries an uncertain activation. Setup and the entire
/// method exchange each have a 1500 ms bound while the process is scheduled.
pub async fn activate_phone(address: &str) -> zbus::Result<()> {
    let bus = NotificationBus::connect(address).await?;
    // Always retire this dedicated connection, including success and cancellation.
    let _abort = AbortOnDrop::new(&bus.socket);
    future::or(
        async {
            let reply = bus
                .connection
                .call_method(
                    Some("org.omarchy.Phone"),
                    "/org/omarchy/Phone",
                    Some("org.omarchy.Phone"),
                    "Activate",
                    &(),
                )
                .await?;
            if reply.body().deserialize::<bool>()? {
                Ok(())
            } else {
                Err(zbus::Error::Failure("Phone activation refused".into()))
            }
        },
        async {
            Timer::after(Duration::from_millis(1500)).await;
            Err(zbus::Error::Failure(
                "Phone activation timed out; remote state unknown".into(),
            ))
        },
    )
    .await
}
