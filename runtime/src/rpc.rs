//! One request/reply per local connection. A failed reply write never cancels
//! an admitted command or changes its durable receipt.
use crate::{
    Identity,
    coordinator::Coordinator,
    ipc::receive_frame,
    ledger::{Error, Outcome, StoredReceipt},
    local_protocol::{self, LocalRequest},
};
use serde::Serialize;
use std::{
    io::{self, Write},
    os::unix::net::UnixStream,
    time::{Duration, Instant},
};
#[derive(Serialize)]
struct Response {
    v: u8,
    id: Identity,
    result: Body,
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Body {
    Fence {
        epoch: Identity,
        session: Identity,
        admission: Identity,
        items: Vec<crate::local_reply::RecoveryItem>,
    },
    Recovery {
        epoch: Identity,
        items: Vec<crate::local_reply::RecoveryItem>,
    },
    Controls {
        snapshot: crate::android_protocol::Controls,
    },
    DialStatus {
        snapshot: crate::android_protocol::DialStatus,
    },
    Status {
        snapshot: crate::android_protocol::PhoneStatus,
    },
    Hello {
        epoch: Identity,
        session: Identity,
    },
    Submission {
        outcome: Outcome,
    },
    Receipt {
        epoch: Identity,
        target: Identity,
        receipt: Option<StoredReceipt>,
    },
    Acknowledged {
        epoch: Identity,
        target: Identity,
    },
    Error {
        code: &'static str,
    },
}
fn failure(error: Error) -> Body {
    let code = match error {
        Error::Refused("unfinished operation requires recovery") => "busy",
        Error::Refused(_) => "refused",
        Error::Io(_) | Error::Database(_) => "unavailable",
    };
    Body::Error { code }
}
pub fn serve(owner: &Coordinator, socket: &mut UnixStream) -> io::Result<()> {
    // Reject foreign peers before reading data or returning session information.
    let (epoch, session) = owner
        .hello(socket)
        .map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied, "local peer unavailable"))?;
    let frame = receive_frame(socket, Duration::from_secs(2))?;
    let request = local_protocol::decode(&frame)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid local request"))?;
    let (id, result) = match request {
        LocalRequest::Fence { id, epoch, session } => {
            let result = owner
                .fence(socket, &epoch, &session)
                .map(|(admission, items)| Body::Fence {
                    epoch,
                    session,
                    admission,
                    items,
                })
                .unwrap_or_else(failure);
            (id, result)
        }
        LocalRequest::Recovery { id, epoch } => {
            let result = owner
                .recovery(socket, &epoch)
                .map(|items| Body::Recovery { epoch, items })
                .unwrap_or_else(failure);
            (id, result)
        }
        LocalRequest::Controls { id } => {
            let result = owner
                .controls(socket, &id)
                .map(|snapshot| Body::Controls { snapshot })
                .unwrap_or(Body::Error {
                    code: "unavailable",
                });
            (id, result)
        }
        LocalRequest::DialStatus { id } => {
            let result = owner
                .dial_status(socket, &id)
                .map(|snapshot| Body::DialStatus { snapshot })
                .unwrap_or(Body::Error {
                    code: "unavailable",
                });
            (id, result)
        }
        LocalRequest::Status { id } => {
            let result = owner
                .status(socket, &id)
                .map(|snapshot| Body::Status { snapshot })
                .unwrap_or(Body::Error {
                    code: "unavailable",
                });
            (id, result)
        }
        LocalRequest::Hello { id } => (id, Body::Hello { epoch, session }),
        LocalRequest::Submit(request, admission) => {
            let id = request.id().clone();
            let body = owner
                .admit(socket, request, &admission)
                .map(|outcome| Body::Submission { outcome })
                .unwrap_or_else(failure);
            (id, body)
        }
        LocalRequest::Lookup { id, epoch, target } => {
            let body = owner
                .lookup(socket, &epoch, &target)
                .map(|receipt| Body::Receipt {
                    epoch,
                    target,
                    receipt,
                })
                .unwrap_or_else(failure);
            (id, body)
        }
        LocalRequest::Acknowledge { id, epoch, target } => {
            let body = owner
                .acknowledge(socket, &epoch, &target)
                .map(|()| Body::Acknowledged { epoch, target })
                .unwrap_or_else(failure);
            (id, body)
        }
    };
    let mut bytes = serde_json::to_vec(&Response { v: 1, id, result }).map_err(io::Error::other)?;
    bytes.push(b'\n');
    let original = socket.write_timeout()?;
    let deadline = Instant::now() + Duration::from_secs(2);
    let result = (|| {
        let mut remaining = bytes.as_slice();
        while !remaining.is_empty() {
            let budget = deadline.saturating_duration_since(Instant::now());
            if budget.is_zero() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "local reply deadline",
                ));
            }
            socket.set_write_timeout(Some(budget))?;
            match socket.write(remaining) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "local reply closed",
                    ));
                }
                Ok(count) => remaining = &remaining[count..],
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    })();
    let restored = socket.set_write_timeout(original);
    result.and(restored)
}
