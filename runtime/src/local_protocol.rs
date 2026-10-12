//! One local RPC format. Lookup observes; acknowledgement never changes outcome.
use crate::{Command, Envelope, Identity, MAX_FRAME, Request};
use serde::Deserialize;
#[derive(Debug)]
pub enum LocalRequest {
    Fence {
        id: Identity,
        epoch: Identity,
        session: Identity,
    },
    Recovery {
        id: Identity,
        epoch: Identity,
    },
    Controls {
        id: Identity,
    },
    DialStatus {
        id: Identity,
    },
    Status {
        id: Identity,
    },
    Hello {
        id: Identity,
    },
    Submit(Request, Identity),
    Lookup {
        id: Identity,
        epoch: Identity,
        target: Identity,
    },
    Acknowledge {
        id: Identity,
        epoch: Identity,
        target: Identity,
    },
}
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Wire {
    Fence {
        v: u8,
        id: Identity,
        epoch: Identity,
        session: Identity,
    },
    Recovery {
        v: u8,
        id: Identity,
        epoch: Identity,
    },
    Controls {
        v: u8,
        id: Identity,
    },
    DialStatus {
        v: u8,
        id: Identity,
    },
    Status {
        v: u8,
        id: Identity,
    },
    Hello {
        v: u8,
        id: Identity,
    },
    Submit {
        v: u8,
        id: Identity,
        epoch: Identity,
        session: Identity,
        admission: Identity,
        command: Command,
    },
    Lookup {
        v: u8,
        id: Identity,
        epoch: Identity,
        target: Identity,
    },
    Acknowledge {
        v: u8,
        id: Identity,
        epoch: Identity,
        target: Identity,
    },
}
pub fn decode(frame: &[u8]) -> Result<LocalRequest, &'static str> {
    if frame.is_empty()
        || frame.len() > MAX_FRAME
        || frame.last() != Some(&b'\n')
        || frame[..frame.len() - 1].contains(&b'\n')
    {
        return Err("invalid local frame");
    }
    let wire: Wire = serde_json::from_slice(frame).map_err(|_| "invalid local request")?;
    match wire {
        Wire::Fence {
            v: 1,
            id,
            epoch,
            session,
        } => Ok(LocalRequest::Fence { id, epoch, session }),
        Wire::Recovery { v: 1, id, epoch } => Ok(LocalRequest::Recovery { id, epoch }),
        Wire::Controls { v: 1, id } => Ok(LocalRequest::Controls { id }),
        Wire::DialStatus { v: 1, id } => Ok(LocalRequest::DialStatus { id }),
        Wire::Status { v: 1, id } => Ok(LocalRequest::Status { id }),
        Wire::Hello { v: 1, id } => Ok(LocalRequest::Hello { id }),
        Wire::Submit {
            v: 1,
            id,
            epoch,
            session,
            admission,
            command,
        } if command.valid() => Ok(LocalRequest::Submit(
            Request(Envelope {
                v: 1,
                id,
                epoch,
                session,
                command,
            }),
            admission,
        )),
        Wire::Lookup {
            v: 1,
            id,
            epoch,
            target,
        } => Ok(LocalRequest::Lookup { id, epoch, target }),
        Wire::Acknowledge {
            v: 1,
            id,
            epoch,
            target,
        } => Ok(LocalRequest::Acknowledge { id, epoch, target }),
        _ => Err("invalid local request"),
    }
}
