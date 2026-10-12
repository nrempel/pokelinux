//! Strict Android command encoding and authenticated reply contracts.
//! A receipt proves only the stated framework outcome, never call connection.
use crate::{CallState, Command, Identity, Request};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptOutcome {
    ControlSubmitted,
    ControlRejected,
    DialInvoked,
    DialNotInvoked,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    Accepted,
    Returned,
    DispatchError,
    Disabled,
    StaleGeneration,
    StaleRevision,
    StaleState,
    MissingCall,
    Unavailable,
    ReplayConflict,
    Capacity,
    Deadline,
    InvalidRequest,
    Pending,
    StaleOffer,
    AccountChanged,
    StorageError,
    Role,
    Permission,
    Busy,
    AccountUnavailable,
    Emergency,
}
#[derive(Debug)]
pub struct Receipt {
    outcome: ReceiptOutcome,
    reason: Reason,
}
impl Receipt {
    pub fn outcome(&self) -> ReceiptOutcome {
        self.outcome
    }
    pub fn reason(&self) -> Reason {
        self.reason
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReceipt {
    v: u8,
    id: Identity,
    generation: Identity,
    outcome: String,
    reason: Reason,
}

/// Deserialize directly from the bounded frame. Parsing through a map first
/// would discard duplicate fields and is deliberately not part of this API.
pub fn decode_receipt(bytes: &[u8], request: &Request) -> Result<Receipt, &'static str> {
    if bytes.is_empty() || bytes.len() > 65_536 {
        return Err("invalid receipt length");
    }
    let raw: RawReceipt = serde_json::from_slice(bytes).map_err(|_| "invalid receipt schema")?;
    let (dial, generation) = match request.command() {
        Command::Dial { generation, .. } => (true, generation),
        Command::Control { generation, .. } => (false, generation),
    };
    if raw.v != 1 || raw.id != *request.id() || raw.generation != *generation {
        return Err("receipt identity mismatch");
    }
    use Reason::*;
    let outcome = match (dial, raw.outcome.as_str(), raw.reason) {
        (false, "submitted", Accepted) => ReceiptOutcome::ControlSubmitted,
        (false, "unknown", DispatchError) | (true, "outcome_unknown", DispatchError) => {
            ReceiptOutcome::Unknown
        }
        (
            false,
            "rejected",
            Disabled | StaleGeneration | StaleRevision | StaleState | MissingCall | Unavailable
            | ReplayConflict | Capacity | Deadline,
        ) => ReceiptOutcome::ControlRejected,
        (true, "invocation_attempted", Returned) => ReceiptOutcome::DialInvoked,
        (
            true,
            "not_invoked",
            InvalidRequest | StaleGeneration | ReplayConflict | Capacity | Pending | StaleOffer
            | StaleRevision | AccountChanged | Deadline | StorageError | Unavailable | Disabled
            | Role | Permission | Busy | AccountUnavailable | Emergency,
        ) => ReceiptOutcome::DialNotInvoked,
        _ => return Err("invalid outcome/reason for requested operation"),
    };
    Ok(Receipt {
        outcome,
        reason: raw.reason,
    })
}
/// Local epoch/session authority stays in the Linux service. Android receives
/// its generation, operation identity and the exact validated command context.
pub fn encode_command(request: &Request) -> Vec<u8> {
    let value = match request.command() {
        Command::Dial {
            generation,
            offer,
            number,
        } => {
            json!({"v":1,"id":request.id(),"op":"dial","generation":generation,"offer":offer,"number":number})
        }
        Command::Control {
            generation,
            call_id,
            revision,
            state,
            action,
        } => {
            let state = match state {
                CallState::Ringing => 2,
                CallState::Dialing => 1,
                CallState::Connecting => 9,
                CallState::Active => 4,
                CallState::Held => 3,
                CallState::Disconnected => 7,
            };
            json!({"v":1,"id":request.id(),"op":"control","generation":generation,"call_id":call_id,"expected_revision":revision,"expected_state":state,"action":action})
        }
    };
    // Request is privately constructed by the bounded typed local decoder.
    serde_json::to_vec(&value).expect("validated command contains only JSON values")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Availability {
    Observed,
    Unavailable,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedCall {
    id: u32,
    state: u8,
    actions: Vec<crate::ControlAction>,
}
impl ObservedCall {
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn state(&self) -> u8 {
        self.state
    }
    pub fn actions(&self) -> &[crate::ControlAction] {
        &self.actions
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawControls {
    v: u8,
    id: Identity,
    generation: Identity,
    revision: u64,
    controls_allowed: bool,
    availability: Availability,
    source: String,
    calls: Vec<ObservedCall>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(try_from = "RawControls")]
pub struct Controls(RawControls);
impl Controls {
    pub fn id(&self) -> &Identity {
        &self.0.id
    }
    pub fn calls(&self) -> &[ObservedCall] {
        &self.0.calls
    }
    pub fn revision(&self) -> u64 {
        self.0.revision
    }
    pub fn generation(&self) -> &Identity {
        &self.0.generation
    }
    pub fn allowed(&self) -> bool {
        self.0.controls_allowed
    }
}
fn decode_frame<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, &'static str> {
    if bytes.is_empty() || bytes.len() > 65_536 {
        return Err("invalid response length");
    }
    serde_json::from_slice(bytes).map_err(|_| "invalid response schema")
}
pub fn decode_controls(bytes: &[u8], id: &Identity) -> Result<Controls, &'static str> {
    let raw: RawControls = decode_frame(bytes)?;
    if &raw.id != id {
        return Err("controls identity mismatch");
    }
    Controls::try_from(raw)
}
impl TryFrom<RawControls> for Controls {
    type Error = &'static str;
    fn try_from(raw: RawControls) -> Result<Self, Self::Error> {
        if raw.v != 1
            || !(1..=9_007_199_254_740_991).contains(&raw.revision)
            || raw.source != "android.telecom.InCallService"
            || raw.calls.len() > 64
            || (raw.availability == Availability::Observed) == raw.calls.is_empty()
        {
            return Err("invalid control observation");
        }
        let mut ids = std::collections::HashSet::new();
        for call in &raw.calls {
            if !(1..=2_147_483_647).contains(&call.id) || call.state > 20 || !ids.insert(call.id) {
                return Err("invalid observed call");
            }
            for (index, action) in call.actions.iter().enumerate() {
                if call.actions[..index].contains(action)
                    || !raw.controls_allowed
                    || raw.calls.len() != 1
                {
                    return Err("invalid observed action");
                }
                use crate::ControlAction::*;
                let permitted = match action {
                    Answer | Reject => call.state == 2,
                    Disconnect => matches!(call.state, 1 | 3 | 4 | 9),
                };
                if !permitted {
                    return Err("action does not match observed state");
                }
            }
        }
        Ok(Controls(raw))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DialReason {
    Ready,
    Disabled,
    Role,
    Permission,
    Busy,
    AccountUnavailable,
    Unavailable,
    OfferCapacity,
    Pending,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawDialStatus {
    v: u8,
    id: Identity,
    generation: Identity,
    can_dial: bool,
    offer: String,
    pending: bool,
    reason: DialReason,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(try_from = "RawDialStatus", into = "RawDialStatus")]
pub struct DialStatus {
    id: Identity,
    generation: Identity,
    offer: Option<Identity>,
    pending: bool,
    reason: DialReason,
}
impl DialStatus {
    pub fn id(&self) -> &Identity {
        &self.id
    }
    pub fn generation(&self) -> &Identity {
        &self.generation
    }
    pub fn offer(&self) -> Option<&Identity> {
        self.offer.as_ref()
    }
    pub fn pending(&self) -> bool {
        self.pending
    }
    pub fn reason(&self) -> DialReason {
        self.reason
    }
}
pub fn decode_dial_status(bytes: &[u8], id: &Identity) -> Result<DialStatus, &'static str> {
    let raw: RawDialStatus = decode_frame(bytes)?;
    if raw.id != *id {
        return Err("dial observation identity mismatch");
    }
    DialStatus::try_from(raw)
}
impl TryFrom<RawDialStatus> for DialStatus {
    type Error = &'static str;
    fn try_from(raw: RawDialStatus) -> Result<Self, Self::Error> {
        if raw.v != 1 {
            return Err("invalid dial observation version");
        }
        let offer = if raw.can_dial {
            if raw.pending || raw.reason != DialReason::Ready {
                return Err("invalid ready dial observation");
            }
            Some(Identity::try_from(raw.offer)?)
        } else {
            if !raw.offer.is_empty()
                || raw.reason == DialReason::Ready
                || raw.pending != (raw.reason == DialReason::Pending)
            {
                return Err("invalid unavailable dial observation");
            }
            None
        };
        Ok(DialStatus {
            id: raw.id,
            generation: raw.generation,
            offer,
            pending: raw.pending,
            reason: raw.reason,
        })
    }
}
impl From<DialStatus> for RawDialStatus {
    fn from(status: DialStatus) -> Self {
        Self {
            v: 1,
            id: status.id,
            generation: status.generation,
            can_dial: status.offer.is_some(),
            offer: status.offer.map(|id| id.0).unwrap_or_default(),
            pending: status.pending,
            reason: status.reason,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatusCall {
    id: u32,
    state: u8,
}
impl StatusCall {
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn state(&self) -> u8 {
        self.state
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawPhoneState {
    availability: Availability,
    // A custom field decoder makes this field required, while permitting an
    // explicit JSON null. Bare Option would also accept an absent field.
    #[serde(deserialize_with = "required_nullable_state")]
    state: Option<u8>,
    source: String,
    scope: String,
}
fn required_nullable_state<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<u8>, D::Error> {
    Option::<u8>::deserialize(decoder)
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawStatus {
    v: u8,
    id: Identity,
    generation: Identity,
    observed_ms: u64,
    dialer_role: bool,
    availability: Availability,
    source: String,
    calls: Vec<StatusCall>,
    phone_state: RawPhoneState,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(try_from = "RawStatus")]
pub struct PhoneStatus(RawStatus);
impl PhoneStatus {
    pub fn id(&self) -> &Identity {
        &self.0.id
    }
    pub fn calls(&self) -> &[StatusCall] {
        &self.0.calls
    }
    pub fn phone_state(&self) -> Option<u8> {
        self.0.phone_state.state
    }
    pub fn dialer_role(&self) -> bool {
        self.0.dialer_role
    }
    pub fn generation(&self) -> &Identity {
        &self.0.generation
    }
    /// Android elapsed realtime, not a local or wall-clock timestamp.
    pub fn observed_ms(&self) -> u64 {
        self.0.observed_ms
    }
}
pub fn decode_status(bytes: &[u8], id: &Identity) -> Result<PhoneStatus, &'static str> {
    let raw: RawStatus = decode_frame(bytes)?;
    if &raw.id != id {
        return Err("status identity mismatch");
    }
    PhoneStatus::try_from(raw)
}
impl TryFrom<RawStatus> for PhoneStatus {
    type Error = &'static str;
    fn try_from(raw: RawStatus) -> Result<Self, Self::Error> {
        if raw.v != 1
            || raw.observed_ms > 9_007_199_254_740_991
            || raw.source != "android.telecom.InCallService"
            || raw.calls.len() > 64
            || (raw.availability == Availability::Observed) == raw.calls.is_empty()
            || (!raw.dialer_role && !raw.calls.is_empty())
        {
            return Err("invalid phone status");
        }
        let mut ids = std::collections::HashSet::new();
        for call in &raw.calls {
            if !(1..=2_147_483_647).contains(&call.id) || call.state > 20 || !ids.insert(call.id) {
                return Err("invalid status call");
            }
        }
        let phone = &raw.phone_state;
        if phone.source != "android.telephony.TelephonyCallback"
            || phone.scope != "default_subscription"
            || (phone.availability == Availability::Observed) != phone.state.is_some()
            || phone.state.is_some_and(|state| state > 2)
        {
            return Err("invalid default-subscription state");
        }
        Ok(PhoneStatus(raw))
    }
}
