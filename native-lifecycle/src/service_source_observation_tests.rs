//! Local typed Authority/Provider branches only; no producer/IPC proof.
use super::{refresh_session, stop_session};
use crate::{
    authority::Authority,
    boottime_ns,
    session::{Provider, SAMPLE_BUDGET_NS, Snapshot},
    storage::Durable,
};
use serde_json::Value;
use std::io;
struct Facts {
    sample: Option<Snapshot>,
    stopped: bool,
}
impl Provider for Facts {
    fn poll(&mut self) -> io::Result<Option<Snapshot>> {
        if self.stopped {
            Err(io::ErrorKind::NotConnected.into())
        } else {
            Ok(self.sample.clone())
        }
    }
    fn stop(&mut self) {
        self.stopped = true;
    }
    fn occupied(&self) -> bool {
        false
    }
}
fn setup() -> (Authority, Facts) {
    let mut a = Authority::loading(vec![]).unwrap();
    a.activate(Durable::default());
    a.require_session().unwrap();
    let now = boottime_ns();
    let f = Facts {
        sample: Some(Snapshot {
            source: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            revision: 1,
            sequence: 1,
            sampled_at_ns: now,
            until: now + SAMPLE_BUDGET_NS,
            locked: false,
            focus: None,
            locker: None,
            lock_generation: None,
            protocol_unlock: None,
        }),
        stopped: false,
    };
    (a, f)
}
fn poll(a: &mut Authority, f: &mut Facts) {
    refresh_session(a, &mut Some(f as &mut dyn Provider)).unwrap();
}
fn status(a: &mut Authority) -> Value {
    let v = serde_json::to_value(a.snapshot(false, false)).unwrap();
    assert!(v["source"].is_object(), "MISSING_OBSERVED_SOURCE_STATUS");
    v["source"].clone()
}
#[test]
fn authority_rejection_keeps_only_actually_accepted_sample() {
    let (mut a, mut f) = setup();
    poll(&mut a, &mut f);
    f.sample.as_mut().unwrap().sequence = 0;
    poll(&mut a, &mut f);
    assert!(a.session_terminal());
    let s = status(&mut a);
    assert_eq!(s["state"], "failed");
    assert_eq!(s["last_accepted"]["sequence"], 1);
    assert_eq!(s["first_failure"]["category"], "authority_rejected");
    assert!(s["first_failure"]["error_kind"].is_null());
}
#[test]
fn missing_after_established_retains_first_category() {
    let (mut a, mut f) = setup();
    poll(&mut a, &mut f);
    f.sample = None;
    poll(&mut a, &mut f);
    assert!(a.session_terminal());
    f.stop();
    poll(&mut a, &mut f);
    let s = status(&mut a);
    assert_eq!(s["first_failure"]["category"], "missing_after_established");
    assert_eq!(s["last_accepted"]["sequence"], 1);
}
#[test]
fn ignored_reconciliation_does_not_advance_accepted_observation() {
    let (mut a, mut f) = setup();
    poll(&mut a, &mut f);
    a.lifecycle(None, Some(true), None).unwrap();
    a.lifecycle(None, Some(false), None).unwrap();
    f.sample.as_mut().unwrap().sequence = 2;
    poll(&mut a, &mut f);
    assert!(!a.session_terminal());
    let s = status(&mut a);
    assert_eq!(s["state"], "observed");
    assert_eq!(s["last_accepted"]["sequence"], 1);
    assert!(s["first_failure"].is_null());
}
#[test]
fn intentional_stop_is_not_a_new_failure_cause() {
    let (mut a, mut f) = setup();
    poll(&mut a, &mut f);
    stop_session(&mut a, &mut f);
    poll(&mut a, &mut f);
    let s = status(&mut a);
    assert_eq!(s["state"], "stopping");
    assert!(s["first_failure"].is_null());
    assert_eq!(s["last_accepted"]["sequence"], 1);
}
