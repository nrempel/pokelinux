//! Direct-library hwsim controls; hooks exist only in cfg(test) builds.
//! No fake clock or event is used. The owned fixture supplies an actual AP,
//! deauthentication, independent iw readbacks and complete process/radio custody.
use super::*;
use crate::PeriodicRefresh;
use std::{
    cell::RefCell,
    fs::{self, OpenOptions},
    io::Write,
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    path::PathBuf,
    time::Duration,
};

const PERIODIC_NS: u64 = 100_000_000;
const EVENT_NS: u64 = 1_000_000_000;
const NEXT_OP_NS: u64 = 50_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Point {
    BeforeFinalReceive,
    AfterEventReceive,
    AfterConnectSend,
}
#[derive(Clone, Debug)]
struct Hook {
    point: Point,
    command_fd: i32,
    event_fd: Option<i32>,
    fired: bool,
    phase: Option<RadioObservationPhase>,
    transaction: Option<CommandTransaction>,
    original_until_ns: u64,
    entered_at_ns: u64,
    expired_at_ns: u64,
    consumed_event_bytes: usize,
    connect_sends: usize,
}
thread_local! {
    static HOOK: RefCell<Option<Hook>> = const { RefCell::new(None) };
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("missing actual fixture input {name}"))
}
fn case_until() -> u64 {
    env("RADIO_PERIODIC_CASE_UNTIL_NS").parse().unwrap()
}
fn cutoff(span: u64) -> u64 {
    let until = boottime_ns().checked_add(span).unwrap();
    assert!(
        until < case_until(),
        "fixture aggregate exhausted before operation"
    );
    until
}
fn unhex(value: &str) -> Vec<u8> {
    assert!(!value.is_empty() && value.len() <= 64 && value.len().is_multiple_of(2));
    (0..value.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&value[at..at + 2], 16).unwrap())
        .collect()
}
fn target() -> RadioTarget {
    let namespace = Namespace::retain().unwrap();
    RadioTarget {
        namespace_dev: namespace.dev,
        namespace_ino: namespace.ino,
        ifindex: env("RADIO_PERIODIC_IFINDEX").parse().unwrap(),
        frequency_mhz: env("RADIO_PERIODIC_FREQUENCY_MHZ").parse().unwrap(),
        bssid: unhex(&env("RADIO_PERIODIC_BSSID_HEX")).try_into().unwrap(),
        ssid: unhex(&env("RADIO_PERIODIC_SSID_HEX")),
    }
}
fn gates() -> PathBuf {
    PathBuf::from(env("RADIO_PERIODIC_GATES"))
}
fn publish(name: &str, body: &str, until: u64) {
    assert!(boottime_ns() < until && body.len() <= 4096);
    let path = gates().join(name);
    let temporary = path.with_extension("tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .unwrap();
    file.write_all(body.as_bytes()).unwrap();
    file.sync_all().unwrap();
    assert!(boottime_ns() < until);
    fs::rename(temporary, path).unwrap();
}
fn wait_gate(name: &str, until: u64) {
    while !gates().join(name).is_file() {
        assert!(boottime_ns() < until, "actual fixture gate missing or late");
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(boottime_ns() < until);
}
fn wait_original_timer(budget: &Budget) {
    let mut fds = [event::PollFd::new(&budget.timer, event::PollFlags::IN)];
    event::poll(
        &mut fds,
        Some(&event::Timespec {
            tv_sec: 2,
            tv_nsec: 0,
        }),
    )
    .unwrap();
    assert!(fds[0].revents().contains(event::PollFlags::IN));
    assert_eq!(budget.fence(), Err(Failure::Expired));
}
fn arm(epoch: &RadioEpoch, point: Point) {
    HOOK.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(Hook {
            point,
            command_fd: epoch.command.socket.as_raw_fd(),
            event_fd: None,
            fired: false,
            phase: None,
            transaction: None,
            original_until_ns: 0,
            entered_at_ns: 0,
            expired_at_ns: 0,
            consumed_event_bytes: 0,
            connect_sends: 0,
        });
    });
}
fn take_hook() -> Hook {
    let hook = HOOK.with(|slot| slot.borrow_mut().take().unwrap());
    assert!(hook.fired, "selected real boundary hook was not reached");
    println!(
        "actual_radio_timing_hook {{\"point\":\"{:?}\",\"phase\":\"{:?}\",\"transaction\":\"{:?}\",\"original_until_ns\":{},\"entered_at_ns\":{},\"expired_at_ns\":{},\"consumed_event_bytes\":{},\"connect_sends\":{}}}",
        hook.point,
        hook.phase,
        hook.transaction,
        hook.original_until_ns,
        hook.entered_at_ns,
        hook.expired_at_ns,
        hook.consumed_event_bytes,
        hook.connect_sends
    );
    hook
}
pub(super) fn before_final_quiet(epoch: &RadioEpoch, budget: &Budget) {
    let point = HOOK.with(|slot| slot.borrow().as_ref().map(|hook| (hook.point, hook.fired)));
    let Some((point, false)) = point else { return };
    if point == Point::AfterConnectSend {
        return;
    }
    assert_eq!(epoch.phase, RadioObservationPhase::QuietAfter);
    assert_eq!(
        epoch.command.transaction.get(),
        CommandTransaction::Completed
    );
    assert!(
        boottime_ns() < budget.until,
        "final quiet hook admitted too late"
    );
    HOOK.with(|slot| {
        let mut slot = slot.borrow_mut();
        let hook = slot.as_mut().unwrap();
        hook.event_fd = Some(epoch.events.socket.as_raw_fd());
        hook.phase = Some(epoch.phase);
        hook.transaction = Some(epoch.command.transaction.get());
        hook.original_until_ns = budget.until;
        hook.entered_at_ns = boottime_ns();
    });
    if point == Point::BeforeFinalReceive {
        wait_original_timer(budget);
        HOOK.with(|slot| {
            let mut slot = slot.borrow_mut();
            let hook = slot.as_mut().unwrap();
            hook.fired = true;
            hook.expired_at_ns = boottime_ns();
        });
    } else {
        publish(
            "quiet-after-ready.json",
            &format!("{{\"original_until_ns\":{}}}\n", budget.until),
            budget.until,
        );
        let mut fds = [
            event::PollFd::new(&epoch.events.socket, event::PollFlags::IN),
            event::PollFd::new(&budget.timer, event::PollFlags::IN),
        ];
        event::poll(&mut fds, None).unwrap();
        assert!(
            fds[0].revents() == event::PollFlags::IN
                && fds[1].revents().is_empty()
                && boottime_ns() < budget.until,
            "actual AP MLME datagram missing or late"
        );
    }
}
pub(super) fn after_receive(connection: &Connection, bytes: usize, budget: &Budget) {
    let selected = HOOK.with(|slot| {
        slot.borrow().as_ref().map(|hook| {
            (
                hook.point,
                hook.fired,
                hook.event_fd == Some(connection.socket.as_raw_fd()),
            )
        })
    });
    let Some((point, fired, true)) = selected else {
        return;
    };
    HOOK.with(|slot| slot.borrow_mut().as_mut().unwrap().consumed_event_bytes += bytes);
    if point != Point::AfterEventReceive || fired {
        return;
    }
    assert!(bytes > 0 && boottime_ns() < budget.until);
    wait_original_timer(budget);
    HOOK.with(|slot| {
        let mut slot = slot.borrow_mut();
        let hook = slot.as_mut().unwrap();
        hook.fired = true;
        hook.expired_at_ns = boottime_ns();
    });
}
pub(super) fn after_send(connection: &Connection, request: &[u8], bytes: usize, budget: &Budget) {
    let selected = HOOK.with(|slot| {
        slot.borrow().as_ref().map(|hook| {
            hook.point == Point::AfterConnectSend
                && !hook.fired
                && hook.command_fd == connection.socket.as_raw_fd()
        })
    });
    if selected != Some(true) || request.get(16) != Some(&CONNECT) {
        return;
    }
    assert_eq!(bytes, request.len());
    assert!(boottime_ns() < budget.until);
    HOOK.with(|slot| {
        let mut slot = slot.borrow_mut();
        let hook = slot.as_mut().unwrap();
        hook.connect_sends += 1;
        hook.original_until_ns = budget.until;
        hook.entered_at_ns = boottime_ns();
    });
    wait_original_timer(budget);
    HOOK.with(|slot| {
        let mut slot = slot.borrow_mut();
        let hook = slot.as_mut().unwrap();
        hook.fired = true;
        hook.expired_at_ns = boottime_ns();
    });
}

#[derive(Debug, PartialEq, Eq)]
struct Pins {
    namespace: (u64, u64),
    command: (i32, u64, u64, u32),
    events: (i32, u64, u64, u32),
    thread: std::thread::ThreadId,
}
fn socket_pin(connection: &Connection) -> (i32, u64, u64, u32) {
    let fd = connection.socket.as_raw_fd();
    let metadata = fs::metadata(format!("/proc/self/fd/{fd}")).unwrap();
    let mut address = netlink_sys::SocketAddr::new(0, 0);
    connection.socket.get_address(&mut address).unwrap();
    let port = address.port_number();
    (fd, metadata.dev(), metadata.ino(), port)
}
fn pins(epoch: &RadioEpoch) -> Pins {
    Pins {
        namespace: (epoch.namespace.dev, epoch.namespace.ino),
        command: socket_pin(&epoch.command),
        events: socket_pin(&epoch.events),
        thread: std::thread::current().id(),
    }
}
fn attached(associated: bool) -> RadioEpoch {
    let mut epoch = RadioEpoch::attach_disconnected(target(), cutoff(EVENT_NS)).unwrap();
    if associated {
        assert!(
            epoch
                .set_associated(true, cutoff(EVENT_NS))
                .unwrap()
                .associated
        );
    }
    epoch.refresh(cutoff(EVENT_NS)).unwrap();
    epoch
}
fn next_op() {
    let timer = Budget::new(boottime_ns() + NEXT_OP_NS).unwrap();
    wait_original_timer(&timer);
}

#[test]
#[ignore = "requires exclusive actual hwsim fixture and owned AP"]
fn periodic_final_quiet_defers_then_same_epoch_observes() {
    let mut epoch = attached(false);
    let before = pins(&epoch);
    arm(&epoch, Point::BeforeFinalReceive);
    let until = cutoff(PERIODIC_NS);
    let result = epoch.refresh_periodic(until);
    let hook = take_hook();
    assert_eq!(hook.original_until_ns, until);
    assert_eq!(hook.consumed_event_bytes, 0);
    assert_eq!(hook.phase, Some(RadioObservationPhase::QuietAfter));
    assert_eq!(hook.transaction, Some(CommandTransaction::Completed));
    assert_eq!(pins(&epoch), before);
    assert!(
        matches!(
            result,
            Ok(PeriodicRefresh::Deferred) | Err(Error::Failed(Failure::Expired))
        ),
        "final quiet outcome is not the selected strict baseline or Deferred"
    );
    if result.is_err() {
        assert_eq!(epoch.state.retired, Some(Error::Failed(Failure::Expired)));
    }
    assert_eq!(
        result,
        Ok(PeriodicRefresh::Deferred),
        "periodic final quiet must defer without retiring or publishing a fact"
    );
    assert!(epoch.state.retired.is_none());
    next_op();
    let started = boottime_ns();
    let PeriodicRefresh::Observed(fact) = epoch.refresh_periodic(cutoff(PERIODIC_NS)).unwrap()
    else {
        panic!("separately scheduled full observation did not return a fact");
    };
    assert!(!fact.associated && fact.observed_at_boottime_ns >= started);
    assert_eq!(pins(&epoch), before);
}

#[test]
#[ignore = "requires exclusive actual hwsim fixture and owned AP"]
fn strict_refresh_final_quiet_expiry_stays_terminal() {
    let mut epoch = attached(false);
    arm(&epoch, Point::BeforeFinalReceive);
    let until = cutoff(PERIODIC_NS);
    assert_eq!(epoch.refresh(until), Err(Error::Failed(Failure::Expired)));
    let hook = take_hook();
    assert_eq!(hook.original_until_ns, until);
    assert_eq!(hook.consumed_event_bytes, 0);
    assert_eq!(epoch.state.retired, Some(Error::Failed(Failure::Expired)));
    assert_eq!(
        epoch.refresh(cutoff(PERIODIC_NS)),
        Err(Error::Failed(Failure::Expired))
    );
}

#[test]
#[ignore = "requires actual AP deauth at quiet-after-ready gate"]
fn consumed_mlme_then_expiry_stays_terminal() {
    let mut epoch = attached(true);
    arm(&epoch, Point::AfterEventReceive);
    let until = cutoff(EVENT_NS);
    assert_eq!(
        epoch.refresh_periodic(until),
        Err(Error::Failed(Failure::Expired))
    );
    let hook = take_hook();
    assert_eq!(hook.original_until_ns, until);
    assert!(hook.consumed_event_bytes > 0);
    assert_eq!(hook.phase, Some(RadioObservationPhase::QuietAfter));
    assert_eq!(hook.transaction, Some(CommandTransaction::Completed));
    assert_eq!(epoch.state.retired, Some(Error::Failed(Failure::Expired)));
    assert_eq!(
        epoch.refresh_periodic(cutoff(PERIODIC_NS)),
        Err(Error::Failed(Failure::Expired))
    );
}

#[test]
#[ignore = "requires safe deferral then actual AP deauth before next observation"]
fn queued_ap_loss_during_deferral_stays_terminal() {
    let mut epoch = attached(true);
    let before = pins(&epoch);
    arm(&epoch, Point::BeforeFinalReceive);
    assert_eq!(
        epoch.refresh_periodic(cutoff(PERIODIC_NS)),
        Ok(PeriodicRefresh::Deferred)
    );
    assert_eq!(take_hook().consumed_event_bytes, 0);
    let action_until = cutoff(EVENT_NS);
    publish(
        "deferred-ready.json",
        &format!("{{\"original_until_ns\":{action_until}}}\n"),
        action_until,
    );
    wait_gate("loss-ready.json", action_until);
    next_op();
    assert_eq!(
        epoch.refresh_periodic(cutoff(PERIODIC_NS)),
        Err(Error::Failed(Failure::Contended))
    );
    assert_eq!(epoch.state.retired, Some(Error::Failed(Failure::Contended)));
    assert_eq!(pins(&epoch), before);
}

#[test]
#[ignore = "requires exclusive actual hwsim fixture; Connect dispatch is uncertain"]
fn dispatched_connect_expiry_stays_uncertain() {
    let mut epoch = attached(false);
    arm(&epoch, Point::AfterConnectSend);
    let until = cutoff(EVENT_NS);
    assert_eq!(
        epoch.set_associated(true, until),
        Err(Error::Uncertain(Failure::Expired))
    );
    let hook = take_hook();
    assert_eq!(hook.original_until_ns, until);
    assert_eq!(hook.connect_sends, 1);
    assert_eq!(
        epoch.state.retired,
        Some(Error::Uncertain(Failure::Expired))
    );
    let sequence = epoch.command.sequence;
    assert_eq!(
        epoch.refresh_periodic(cutoff(PERIODIC_NS)),
        Err(Error::Uncertain(Failure::Expired))
    );
    assert_eq!(epoch.command.sequence, sequence);
}
