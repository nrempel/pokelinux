//! Native-client reply validation. A valid receipt is not proof of call connection.
use crate::{Command, Identity, MAX_FRAME, local_protocol::LocalRequest};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pending,
    Unknown,
    NotSent,
    DialInvoked,
    DialNotInvoked,
    ControlSubmitted,
    ControlRejected,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Busy,
    Refused,
    Unavailable,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    outcome: Outcome,
    acknowledged: bool,
}
impl Receipt {
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }
    pub fn acknowledged(&self) -> bool {
        self.acknowledged
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryItem {
    pub(crate) id: Identity,
    pub(crate) outcome: Outcome,
}
impl RecoveryItem {
    pub fn id(&self) -> &Identity {
        &self.id
    }
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Body {
    Fence {
        epoch: Identity,
        session: Identity,
        admission: Identity,
        items: Vec<RecoveryItem>,
    },
    Recovery {
        epoch: Identity,
        items: Vec<RecoveryItem>,
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
        #[serde(deserialize_with = "required_receipt")]
        receipt: Option<Receipt>,
    },
    Acknowledged {
        epoch: Identity,
        target: Identity,
    },
    Error {
        code: ErrorCode,
    },
}
fn required_receipt<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<Receipt>, D::Error> {
    Option::<Receipt>::deserialize(decoder)
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawReply {
    v: u8,
    id: Identity,
    result: Body,
}
#[derive(Debug, Serialize)]
pub struct Reply(RawReply);
impl Reply {
    pub fn id(&self) -> &Identity {
        &self.0.id
    }
    pub fn body(&self) -> &Body {
        &self.0.result
    }
}
pub fn decode(frame: &[u8], request: &LocalRequest) -> Result<Reply, &'static str> {
    if frame.is_empty()
        || frame.len() > MAX_FRAME
        || frame.last() != Some(&b'\n')
        || frame[..frame.len() - 1].contains(&b'\n')
    {
        return Err("invalid local reply frame");
    }
    let raw: RawReply = serde_json::from_slice(frame).map_err(|_| "invalid local reply schema")?;
    let expected_id = match request {
        LocalRequest::Fence { id, .. }
        | LocalRequest::Recovery { id, .. }
        | LocalRequest::Controls { id }
        | LocalRequest::DialStatus { id }
        | LocalRequest::Status { id }
        | LocalRequest::Hello { id }
        | LocalRequest::Lookup { id, .. }
        | LocalRequest::Acknowledge { id, .. } => id,
        LocalRequest::Submit(command, _) => command.id(),
    };
    if raw.v != 1 || &raw.id != expected_id {
        return Err("local reply identity mismatch");
    }
    let matches = match (request, &raw.result) {
        (
            LocalRequest::Fence { epoch, session, .. },
            Body::Fence {
                epoch: actual,
                session: incarnation,
                items,
                ..
            },
        ) => {
            epoch == actual
                && session == incarnation
                && items.len() <= 1
                && items
                    .iter()
                    .all(|item| matches!(item.outcome, Outcome::Pending | Outcome::Unknown))
        }
        (
            LocalRequest::Recovery { epoch, .. },
            Body::Recovery {
                epoch: actual,
                items,
            },
        ) => {
            epoch == actual
                && items.len() <= 1
                && items
                    .iter()
                    .all(|item| matches!(item.outcome, Outcome::Pending | Outcome::Unknown))
        }
        (LocalRequest::Controls { id }, Body::Controls { snapshot }) => snapshot.id() == id,
        (LocalRequest::DialStatus { id }, Body::DialStatus { snapshot }) => snapshot.id() == id,
        (LocalRequest::Status { id }, Body::Status { snapshot }) => snapshot.id() == id,
        (_, Body::Error { .. }) | (LocalRequest::Hello { .. }, Body::Hello { .. }) => true,
        (LocalRequest::Submit(request, _), Body::Submission { outcome }) => {
            matches!(
                (request.command(), outcome),
                (_, Outcome::Pending | Outcome::Unknown | Outcome::NotSent)
                    | (
                        Command::Dial { .. },
                        Outcome::DialInvoked | Outcome::DialNotInvoked
                    )
                    | (
                        Command::Control { .. },
                        Outcome::ControlSubmitted | Outcome::ControlRejected
                    )
            )
        }
        (
            LocalRequest::Lookup { epoch, target, .. },
            Body::Receipt {
                epoch: actual_epoch,
                target: actual_target,
                receipt,
            },
        ) => {
            epoch == actual_epoch
                && target == actual_target
                && receipt
                    .as_ref()
                    .is_none_or(|r| !r.acknowledged || r.outcome == Outcome::Unknown)
        }
        (
            LocalRequest::Acknowledge { epoch, target, .. },
            Body::Acknowledged {
                epoch: actual_epoch,
                target: actual_target,
            },
        ) => epoch == actual_epoch && target == actual_target,
        _ => false,
    };
    if !matches {
        return Err("local reply operation mismatch");
    }
    Ok(Reply(raw))
}
