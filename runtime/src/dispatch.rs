//! Durable command handoff. The service must authorize the caller and session
//! before entering this function and must own its execution through completion.
use crate::{
    Request,
    android_protocol::{ReceiptOutcome, decode_receipt, encode_command},
    ledger::{Admission, Error, Ledger, Outcome},
    tls::{ExchangeError, TlsClient},
};
use std::{io, net::SocketAddr, time::Duration};

/// No local client socket is borrowed. A dropped UI connection must not cancel
/// this work. A journal error is never permission to retry the transport.
pub fn execute(
    ledger: &mut Ledger,
    client: &TlsClient,
    address: SocketAddr,
    request: &Request,
    budget: Duration,
) -> Result<Outcome, Error> {
    match ledger.reserve(request)? {
        Admission::Existing(outcome) => return Ok(outcome),
        Admission::New => (),
    }
    let outcome = exchange_command(client, address, request, budget);
    // If commit fails, no terminal receipt is returned. The pending admission
    // still prevents another send and becomes unknown on service restart.
    ledger.finish(request.id(), outcome)?;
    Ok(outcome)
}

/// Called only after durable admission; does not own or mutate the journal.
pub(crate) fn exchange_command(
    client: &TlsClient,
    address: SocketAddr,
    request: &Request,
    budget: Duration,
) -> Outcome {
    let body = encode_command(request);
    let result = client.exchange_decoded(address, &body, budget, |bytes| {
        decode_receipt(bytes, request)
            .map(|receipt| receipt.outcome())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid command receipt"))
    });
    match result {
        Ok(ReceiptOutcome::DialInvoked) => Outcome::DialInvoked,
        Ok(ReceiptOutcome::DialNotInvoked) => Outcome::DialNotInvoked,
        Ok(ReceiptOutcome::ControlSubmitted) => Outcome::ControlSubmitted,
        Ok(ReceiptOutcome::ControlRejected) => Outcome::ControlRejected,
        Ok(ReceiptOutcome::Unknown) | Err(ExchangeError::Unknown(_)) => Outcome::Unknown,
        Err(ExchangeError::NotSent(_)) => Outcome::NotSent,
    }
}
