#![cfg(target_os = "linux")]
use async_io::{Timer, block_on};
use futures_lite::future;
use omarchy_phone_core::{
    notification::Notifications,
    notification_bus::NotificationBus,
    observer::{Lease, Observer},
    watch::decode_snapshot,
};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use zbus::{blocking::connection::Builder, zvariant::OwnedValue};
struct Bus {
    child: Child,
    address: String,
}
impl Bus {
    fn new() -> Self {
        Self::at(None)
    }
    fn at(address: Option<&str>) -> Self {
        let mut command = Command::new("dbus-daemon");
        command.args(["--session", "--nofork", "--print-address=1"]);
        if let Some(address) = address {
            command.arg(format!("--address={address}"));
        }
        let mut child = command
            .stdout(Stdio::piped())
            .spawn()
            .expect("isolated dbus-daemon is required");
        let mut address = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut address)
            .unwrap();
        Self {
            child,
            address: address.trim().into(),
        }
    }
}
impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
#[derive(Default)]
struct Events {
    notified: usize,
    closed: Vec<u32>,
}
struct Server {
    events: Arc<Mutex<Events>>,
    delay: Duration,
    close_delay: Duration,
    close_before_reply: bool,
}
#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Server {
    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &self,
        app: &str,
        replaces: u32,
        icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        timeout: i32,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
    ) -> u32 {
        assert_eq!(app, "Phone");
        assert_eq!(replaces, 0);
        assert_eq!(icon, "call-start");
        assert_eq!(summary, "Incoming call");
        assert!(body.is_empty());
        assert_eq!(actions, ["default", "Open Phone"]);
        assert!(bool::try_from(hints.get("suppress-sound").unwrap()).unwrap());
        assert!(bool::try_from(hints.get("transient").unwrap()).unwrap());
        assert!(timeout > 0 && timeout <= 5000);
        self.events.lock().unwrap().notified += 1;
        if self.delay == Duration::MAX {
            std::future::pending::<()>().await;
        } else {
            Timer::after(self.delay).await;
        }
        if self.close_before_reply {
            emitter
                .connection()
                .emit_signal(
                    None::<&str>,
                    "/org/freedesktop/Notifications",
                    "org.freedesktop.Notifications",
                    "NotificationClosed",
                    &(7u32, 2u32),
                )
                .await
                .unwrap();
        }
        7
    }
    async fn close_notification(&self, id: u32) {
        self.events.lock().unwrap().closed.push(id);
        if self.close_delay == Duration::MAX {
            std::future::pending::<()>().await;
        } else {
            Timer::after(self.close_delay).await;
        }
    }
}
fn serve(bus: &Bus, events: Arc<Mutex<Events>>, delay: Duration) -> zbus::blocking::Connection {
    Builder::address(bus.address.as_str())
        .unwrap()
        .serve_at(
            "/org/freedesktop/Notifications",
            Server {
                events,
                delay,
                close_delay: Duration::ZERO,
                close_before_reply: false,
            },
        )
        .unwrap()
        .name("org.freedesktop.Notifications")
        .unwrap()
        .build()
        .unwrap()
}

#[test]
fn stalled_close_is_bounded_and_aborts_the_original_connection() {
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = Builder::address(bus.address.as_str())
        .unwrap()
        .serve_at(
            "/org/freedesktop/Notifications",
            Server {
                events: events.clone(),
                delay: Duration::ZERO,
                close_delay: Duration::MAX,
                close_before_reply: false,
            },
        )
        .unwrap()
        .name("org.freedesktop.Notifications")
        .unwrap()
        .build()
        .unwrap();
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let owner = test_owner(&client).await.unwrap();
        let request = Notifications::default()
            .begin(&owner, &lease(), true, now())
            .unwrap();
        let delivered = client.notify(&request).await.unwrap();
        let result = future::or(async { Some(delivered.close().await) }, async {
            Timer::after(Duration::from_millis(2500)).await;
            None
        })
        .await;
        assert!(
            result
                .expect("close must have its own finite budget")
                .is_err()
        );
        assert_eq!(events.lock().unwrap().closed, [7]);
        assert!(test_owner(&client).await.is_err());
        assert!(signals.next(Some(&owner)).await.is_err());
    });
}
// Tests that inspect another subscription use a separate disposable stream.
// Production discovery always consumes history on its own persistent stream.
async fn test_owner(client: &NotificationBus) -> zbus::Result<String> {
    let mut signals = client.subscribe().await?;
    client
        .discover(&mut signals)
        .await?
        .ok_or_else(|| zbus::Error::Failure("no test notifier".into()))
}
fn now() -> Duration {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::Boottime);
    Duration::new(t.tv_sec as u64, t.tv_nsec as u32)
}
fn lease() -> Lease {
    lease_at(now())
}
fn lease_at(start: Duration) -> Lease {
    observer_at(start).lease(start).unwrap().clone()
}
fn observer_at(start: Duration) -> Observer {
    observer_with_episode(start, "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
}
fn observer_with_episode(start: Duration, episode: &str) -> Observer {
    let mut state = Observer::default();
    for t in [start.saturating_sub(Duration::from_secs(1)), start] {
        let pending = state.begin(t);
        let bytes = serde_json::to_vec(&serde_json::json!({"v":1,"id":pending.query().id(),"generation":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","revision":1,"source":"android.telecom.InCallService","source_state":"observed","ringing":[{"id":7,"episode":episode}]})).unwrap();
        let snapshot = decode_snapshot(&bytes, pending.query().id()).unwrap();
        state.complete(pending, snapshot, t).unwrap();
    }
    state
}

#[test]
fn dismissed_episode_survives_journal_and_notifier_restart_but_new_call_notifies() {
    use omarchy_phone_core::{
        notification_policy::Dismissals,
        notification_visible::{EndReason, VisibleEvent},
    };
    struct Journal(std::path::PathBuf);
    impl Drop for Journal {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let journal =
        Journal(std::env::temp_dir().join(format!("phone-dismissal-{}", uuid::Uuid::new_v4())));
    let bus = Bus::new();
    let old = serve(
        &bus,
        Arc::new(Mutex::new(Events::default())),
        Duration::ZERO,
    );
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let ringing = lease();
        let (_publisher, mut inputs, mut visible) =
            prepare_visible(&client, &mut signals, ringing.clone()).await;
        old.emit_signal(
            None::<&str>,
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "NotificationClosed",
            &(7u32, 2u32),
        )
        .unwrap();
        assert_eq!(
            visible.next(&mut signals, &mut inputs).await.unwrap(),
            VisibleEvent::End(EndReason::Dismissed)
        );
        let mut policy = Dismissals::default();
        policy.dismiss(visible.presented(), ringing.snapshot());
        {
            let mut ledger = omarchy_phone_core::ledger::Ledger::open(&journal.0).unwrap();
            ledger.store_dismissals(&policy).unwrap();
        }
        drop(policy);
        let mut policy = {
            let ledger = omarchy_phone_core::ledger::Ledger::open(&journal.0).unwrap();
            ledger.load_dismissals().unwrap()
        };
        drop(visible.into_parts());
        old.release_name("org.freedesktop.Notifications").unwrap();
        let events = Arc::new(Mutex::new(Events::default()));
        let _replacement = serve(&bus, events.clone(), Duration::ZERO);
        let owner = client.discover(&mut signals).await.unwrap().unwrap();
        let mut guard = Notifications::default();
        assert!(
            guard
                .begin(
                    &owner,
                    &ringing,
                    policy.has_undismissed(ringing.snapshot()),
                    now()
                )
                .is_none()
        );
        assert_eq!(events.lock().unwrap().notified, 0);
        let next = observer_with_episode(now(), "cccccccccccccccccccccccccccccccc");
        let next = next.lease(now()).unwrap();
        policy.observe(next.snapshot());
        let request = guard
            .begin(&owner, next, policy.has_undismissed(next.snapshot()), now())
            .unwrap();
        client
            .notify(&request)
            .await
            .unwrap()
            .close()
            .await
            .unwrap();
        assert_eq!(events.lock().unwrap().notified, 1);
    });
}
#[test]
fn silent_generic_notification_and_close_use_real_bus() {
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let server = serve(&bus, events.clone(), Duration::ZERO);
    let client = block_on(NotificationBus::connect(&bus.address)).unwrap();
    let owner = block_on(test_owner(&client)).unwrap();
    assert_eq!(owner, server.unique_name().unwrap().as_str());
    let request = Notifications::default()
        .begin(&owner, &lease(), true, now())
        .unwrap();
    let delivered = block_on(client.notify(&request)).unwrap();
    assert_eq!(delivered.id(), 7);
    block_on(delivered.close()).unwrap();
    let events = events.lock().unwrap();
    assert_eq!(events.notified, 1);
    assert_eq!(events.closed, [7]);
}
#[test]
fn delayed_reply_close_stays_on_original_server_after_name_moves() {
    let bus = Bus::new();
    let old_events = Arc::new(Mutex::new(Events::default()));
    let old = serve(&bus, old_events.clone(), Duration::from_millis(250));
    let client = block_on(NotificationBus::connect(&bus.address)).unwrap();
    let owner = block_on(test_owner(&client)).unwrap();
    let lease = lease();
    let mut state = Notifications::default();
    let request = state.begin(&owner, &lease, true, now()).unwrap();
    std::thread::scope(|scope| {
        let connection = &client;
        let pending = scope.spawn(move || {
            let delivered = block_on(connection.notify(&request)).unwrap();
            (request, delivered)
        });
        let until = Instant::now() + Duration::from_secs(2);
        while old_events.lock().unwrap().notified == 0 {
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(5));
        }
        old.release_name("org.freedesktop.Notifications").unwrap();
        let new_events = Arc::new(Mutex::new(Events::default()));
        let _new = serve(&bus, new_events.clone(), Duration::ZERO);
        let current = block_on(test_owner(&client)).unwrap();
        assert_ne!(owner, current);
        let (request, delivered) = pending.join().unwrap();
        assert!(matches!(
            state.complete(
                request,
                delivered.id(),
                Some(&current),
                Some(&lease),
                true,
                now()
            ),
            omarchy_phone_core::notification::Completion::Close(_)
        ));
        block_on(delivered.close()).unwrap();
        assert_eq!(old_events.lock().unwrap().closed, [7]);
        assert!(new_events.lock().unwrap().closed.is_empty());
    });
}
#[test]
fn abandoned_notify_retains_late_reply_and_does_not_release_capacity() {
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::from_secs(2));
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let owner = test_owner(&client).await.unwrap();
        let lease = lease();
        let mut state = Notifications::default();
        let request = state.begin(&owner, &lease, true, now()).unwrap();
        let mut pending = Box::pin(client.notify(&request));
        let early = future::or(async { Some(pending.as_mut().await) }, async {
            Timer::after(Duration::from_millis(1500)).await;
            None
        })
        .await;
        assert!(
            early.is_none(),
            "Notify must still be pending at abandonment"
        );
        state.invalidate();
        assert!(state.begin(&owner, &lease, true, now()).is_none());
        assert_eq!(events.lock().unwrap().notified, 1);
        let delivered = pending.await.unwrap();
        assert!(matches!(
            state.complete(
                request,
                delivered.id(),
                Some(&owner),
                Some(&lease),
                true,
                now()
            ),
            omarchy_phone_core::notification::Completion::Close(_)
        ));
        delivered.close().await.unwrap();
        assert_eq!(events.lock().unwrap().closed, [7]);
        assert_eq!(events.lock().unwrap().notified, 1);
    });
}

#[test]
fn request_that_expires_before_send_never_reaches_server() {
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::ZERO);
    let client = block_on(NotificationBus::connect(&bus.address)).unwrap();
    let lease = lease_at(now() - Duration::from_millis(4900));
    let request = Notifications::default()
        .begin(&block_on(test_owner(&client)).unwrap(), &lease, true, now())
        .unwrap();
    std::thread::sleep(Duration::from_millis(150));
    assert!(block_on(client.notify(&request)).is_err());
    assert_eq!(events.lock().unwrap().notified, 0);
}

#[test]
fn real_delayed_reply_survives_uninterrupted_observation_renewal() {
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::from_millis(1200));
    let client = block_on(NotificationBus::connect(&bus.address)).unwrap();
    let owner = block_on(test_owner(&client)).unwrap();
    let mut observer = observer_at(now());
    let first = observer.lease(now()).unwrap().clone();
    let mut state = Notifications::default();
    let request = state.begin(&owner, &first, true, now()).unwrap();
    std::thread::scope(|scope| {
        let connection = &client;
        let pending = scope.spawn(move || {
            let delivered = block_on(connection.notify(&request)).unwrap();
            (request, delivered)
        });
        std::thread::sleep(Duration::from_secs(1));
        let query = observer.begin(now());
        let bytes = serde_json::to_vec(&serde_json::json!({
            "v":1,"id":query.query().id(),"generation":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "revision":1,"source":"android.telecom.InCallService","source_state":"observed",
            "ringing":[{"id":7,"episode":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}]
        }))
        .unwrap();
        let snapshot = decode_snapshot(&bytes, query.query().id()).unwrap();
        observer.complete(query, snapshot, now()).unwrap();
        let renewed = observer.lease(now()).unwrap();
        assert_ne!(first.id(), renewed.id());
        let (request, delivered) = pending.join().unwrap();
        assert!(matches!(
            state.complete(
                request,
                delivered.id(),
                Some(&owner),
                Some(renewed),
                true,
                now()
            ),
            omarchy_phone_core::notification::Completion::Visible(_)
        ));
        assert!(events.lock().unwrap().closed.is_empty());
        block_on(delivered.close()).unwrap();
    });
}

#[test]
fn stalled_bus_authentication_is_bounded_and_connection_is_dropped() {
    use std::{io::Read, os::unix::net::UnixListener};
    let path = std::env::temp_dir().join(format!("phone-bus-{}", uuid::Uuid::new_v4()));
    let listener = UnixListener::bind(&path).unwrap();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(4)))
            .unwrap();
        let mut bytes = [0; 4096];
        loop {
            if socket.read(&mut bytes).unwrap() == 0 {
                break;
            }
        }
    });
    let start = Instant::now();
    let result = block_on(NotificationBus::connect(&format!(
        "unix:path={}",
        path.display()
    )));
    assert!(result.is_err());
    assert!(start.elapsed() < Duration::from_millis(2500));
    server.join().unwrap();
    std::fs::remove_file(path).unwrap();
}

#[test]
fn never_reply_leaves_loop_responsive_and_can_be_dropped() {
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::MAX);
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let owner = test_owner(&client).await.unwrap();
        let lease = lease();
        let mut state = Notifications::default();
        let request = state.begin(&owner, &lease, true, now()).unwrap();
        let mut pending = Box::pin(client.notify(&request));
        let early = future::or(async { Some(pending.as_mut().await) }, async {
            Timer::after(Duration::from_millis(200)).await;
            None
        })
        .await;
        assert!(early.is_none());
        assert_eq!(events.lock().unwrap().notified, 1);
        state.invalidate();
        assert!(state.begin(&owner, &lease, true, now()).is_none());
        let start = Instant::now();
        drop(pending);
        assert!(
            test_owner(&client).await.is_err(),
            "cancelled Notify must abort every connection clone"
        );
        drop(client);
        assert!(start.elapsed() < Duration::from_secs(1));
        // No ID was returned. Dropping the client cannot prove remote cleanup.
        assert!(events.lock().unwrap().closed.is_empty());
    });
}

#[test]
fn local_abstract_bus_and_address_guid_are_preserved() {
    let address = format!("unix:abstract=phone-test-{}", uuid::Uuid::new_v4());
    let bus = Bus::at(Some(&address));
    let events = Arc::new(Mutex::new(Events::default()));
    let server = serve(&bus, events, Duration::ZERO);
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        assert_eq!(
            test_owner(&client).await.unwrap(),
            server.unique_name().unwrap().as_str()
        );
        let transport = bus.address.split(",guid=").next().unwrap();
        let wrong_guid = format!("{transport},guid=00000000000000000000000000000000");
        assert!(NotificationBus::connect(&wrong_guid).await.is_err());
        assert!(
            NotificationBus::connect("tcp:host=127.0.0.1,port=9")
                .await
                .is_err()
        );
    });
}

#[test]
fn reply_barrier_drains_prior_signals_and_preserves_later_signals() {
    use omarchy_phone_core::notification_bus::{CloseReason, NotificationEvent};
    let bus = Bus::new();
    let server = Builder::address(bus.address.as_str())
        .unwrap()
        .serve_at(
            "/org/freedesktop/Notifications",
            Server {
                events: Arc::new(Mutex::new(Events::default())),
                delay: Duration::ZERO,
                close_delay: Duration::ZERO,
                close_before_reply: true,
            },
        )
        .unwrap()
        .name("org.freedesktop.Notifications")
        .unwrap()
        .build()
        .unwrap();
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let owner = test_owner(&client).await.unwrap();
        let request = Notifications::default()
            .begin(&owner, &lease(), true, now())
            .unwrap();
        // Deliberately let the method finish without polling the signal stream.
        let delivery = client.notify(&request).await.unwrap();
        server
            .emit_signal(
                None::<&str>,
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "ActionInvoked",
                &(7u32, "default"),
            )
            .unwrap();
        // Round trip ensures the later signal has reached the client's reader.
        test_owner(&client).await.unwrap();
        assert_eq!(
            signals.before_reply(&delivery).await.unwrap(),
            Some(NotificationEvent::Closed {
                id: 7,
                reason: CloseReason::Dismissed,
            })
        );
        assert_eq!(signals.before_reply(&delivery).await.unwrap(), None);
        assert_eq!(
            signals.next(Some(&owner)).await.unwrap(),
            NotificationEvent::OpenPhone { id: 7 }
        );
        assert_eq!(signals.before_reply(&delivery).await.unwrap(), None);
        let other = NotificationBus::connect(&bus.address).await.unwrap();
        let mut foreign = other.subscribe().await.unwrap();
        assert!(foreign.before_reply(&delivery).await.is_err());
        delivery.close().await.unwrap();
    });
}

#[test]
fn pending_loop_reconciles_dismissal_emitted_immediately_before_reply() {
    use omarchy_phone_core::{
        notification_inputs::presentation_channel, notification_pending::wait_pending,
    };
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = Builder::address(bus.address.as_str())
        .unwrap()
        .serve_at(
            "/org/freedesktop/Notifications",
            Server {
                events: events.clone(),
                delay: Duration::ZERO,
                close_delay: Duration::ZERO,
                close_before_reply: true,
            },
        )
        .unwrap()
        .name("org.freedesktop.Notifications")
        .unwrap()
        .build()
        .unwrap();
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let owner = test_owner(&client).await.unwrap();
        let (mut publisher, mut inputs) = presentation_channel();
        let mut guard = Notifications::default();
        publisher.publish(Some(lease()), true).unwrap();
        let first = inputs.receive(&mut guard).await.unwrap();
        let request = guard
            .begin(&owner, first.lease().unwrap(), true, now())
            .unwrap();
        let result = wait_pending(
            &client,
            &mut signals,
            &mut inputs,
            &mut guard,
            request,
            first,
        )
        .await
        .unwrap();
        assert!(result.dismissed);
        assert!(!result.open_requested);
        assert!(matches!(
            result.completion,
            omarchy_phone_core::notification::Completion::Close(_)
        ));
        result.delivery.close().await.unwrap();
        assert_eq!(events.lock().unwrap().notified, 1);
        assert_eq!(events.lock().unwrap().closed, [7]);
    });
}

#[test]
fn discovery_discards_old_owner_history_and_preserves_future_changes() {
    use omarchy_phone_core::{
        notification_bus::NotificationEvent, notification_inputs::presentation_channel,
        notification_pending::wait_pending,
    };
    let bus = Bus::new();
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let old = serve(
            &bus,
            Arc::new(Mutex::new(Events::default())),
            Duration::ZERO,
        );
        old.release_name("org.freedesktop.Notifications").unwrap();
        let current = serve(
            &bus,
            Arc::new(Mutex::new(Events::default())),
            Duration::ZERO,
        );
        let owner = client.discover(&mut signals).await.unwrap().unwrap();
        assert_eq!(owner, current.unique_name().unwrap().as_str());
        assert!(
            future::or(async { Some(signals.next(Some(&owner)).await) }, async {
                Timer::after(Duration::from_millis(20)).await;
                None
            })
            .await
            .is_none(),
            "owner history before the snapshot must be consumed"
        );
        let (mut publisher, mut inputs) = presentation_channel();
        let mut guard = Notifications::default();
        publisher.publish(Some(lease()), true).unwrap();
        let first = inputs.receive(&mut guard).await.unwrap();
        let request = guard
            .begin(&owner, first.lease().unwrap(), true, now())
            .unwrap();
        let result = wait_pending(
            &client,
            &mut signals,
            &mut inputs,
            &mut guard,
            request,
            first,
        )
        .await
        .unwrap();
        assert!(
            matches!(
                result.completion,
                omarchy_phone_core::notification::Completion::Visible(_)
            ),
            "old discovery history must not revoke a new request"
        );
        result.delivery.close().await.unwrap();
        current
            .release_name("org.freedesktop.Notifications")
            .unwrap();
        assert_eq!(
            signals.next(Some(&owner)).await.unwrap(),
            NotificationEvent::OwnerChanged {
                old_owner: owner,
                new_owner: String::new(),
            }
        );
        let other = NotificationBus::connect(&bus.address).await.unwrap();
        let mut foreign = other.subscribe().await.unwrap();
        assert!(client.discover(&mut foreign).await.is_err());
    });
}

#[test]
fn discovery_without_a_notifier_keeps_later_appearance_observable() {
    use omarchy_phone_core::notification_bus::NotificationEvent;
    let bus = Bus::new();
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        assert_eq!(client.discover(&mut signals).await.unwrap(), None);
        let server = serve(
            &bus,
            Arc::new(Mutex::new(Events::default())),
            Duration::ZERO,
        );
        assert_eq!(
            signals.next(None).await.unwrap(),
            NotificationEvent::OwnerChanged {
                old_owner: String::new(),
                new_owner: server.unique_name().unwrap().to_string(),
            }
        );
    });
}

#[test]
fn visible_notification_withdraws_on_coalesced_permission_loss() {
    use omarchy_phone_core::{
        notification_inputs::presentation_channel,
        notification_pending::wait_pending,
        notification_visible::{EndReason, VisibleEvent, VisibleNotification},
    };
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::ZERO);
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let owner = client.discover(&mut signals).await.unwrap().unwrap();
        let (mut publisher, mut inputs) = presentation_channel();
        let mut guard = Notifications::default();
        let lease = lease();
        publisher.publish(Some(lease.clone()), true).unwrap();
        let first = inputs.receive(&mut guard).await.unwrap();
        let request = guard.begin(&owner, &lease, true, now()).unwrap();
        let reply = wait_pending(
            &client,
            &mut signals,
            &mut inputs,
            &mut guard,
            request,
            first,
        )
        .await
        .unwrap();
        let mut visible = VisibleNotification::new(reply)
            .unwrap_or_else(|_| panic!("expected visible notification"));
        publisher.publish(Some(lease.clone()), false).unwrap();
        publisher.publish(Some(lease), true).unwrap();
        assert_eq!(
            visible.next(&mut signals, &mut inputs).await.unwrap(),
            VisibleEvent::End(EndReason::Revoked)
        );
        assert_eq!(
            visible.next(&mut signals, &mut inputs).await.unwrap(),
            VisibleEvent::End(EndReason::Revoked)
        );
        visible.into_parts().0.close().await.unwrap();
        assert_eq!(events.lock().unwrap().closed, [7]);
        assert_eq!(events.lock().unwrap().notified, 1);
    });
}

async fn prepare_visible(
    client: &NotificationBus,
    signals: &mut omarchy_phone_core::notification_bus::NotificationSignals,
    lease: Lease,
) -> (
    omarchy_phone_core::notification_inputs::PresentationPublisher,
    omarchy_phone_core::notification_inputs::PresentationInputs,
    omarchy_phone_core::notification_visible::VisibleNotification,
) {
    use omarchy_phone_core::{
        notification_inputs::presentation_channel, notification_pending::wait_pending,
        notification_visible::VisibleNotification,
    };
    let owner = client.discover(signals).await.unwrap().unwrap();
    let (mut publisher, mut inputs) = presentation_channel();
    let mut guard = Notifications::default();
    publisher.publish(Some(lease.clone()), true).unwrap();
    let first = inputs.receive(&mut guard).await.unwrap();
    let request = guard.begin(&owner, &lease, true, now()).unwrap();
    let reply = wait_pending(client, signals, &mut inputs, &mut guard, request, first)
        .await
        .unwrap();
    let visible =
        VisibleNotification::new(reply).unwrap_or_else(|_| panic!("expected visible notification"));
    (publisher, inputs, visible)
}

#[test]
fn visible_open_is_once_only_and_expiry_remains_independent() {
    use omarchy_phone_core::notification_visible::{EndReason, VisibleEvent};
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let server = serve(&bus, events.clone(), Duration::ZERO);
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let lease = lease_at(now() - Duration::from_millis(4500));
        let (mut publisher, mut inputs, mut visible) =
            prepare_visible(&client, &mut signals, lease.clone()).await;
        publisher.publish(Some(lease), true).unwrap();
        server
            .emit_signal(
                None::<&str>,
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "NotificationClosed",
                &(99u32, 2u32),
            )
            .unwrap();
        for _ in 0..2 {
            server
                .emit_signal(
                    None::<&str>,
                    "/org/freedesktop/Notifications",
                    "org.freedesktop.Notifications",
                    "ActionInvoked",
                    &(7u32, "default"),
                )
                .unwrap();
        }
        assert_eq!(
            visible.next(&mut signals, &mut inputs).await.unwrap(),
            VisibleEvent::OpenPhone
        );
        assert_eq!(
            visible.next(&mut signals, &mut inputs).await.unwrap(),
            VisibleEvent::End(EndReason::Expired)
        );
        visible.into_parts().0.close().await.unwrap();
        assert_eq!(events.lock().unwrap().closed, [7]);
    });
}

#[test]
fn visible_action_followed_by_close_preserves_open_intent() {
    use omarchy_phone_core::notification_visible::{EndReason, VisibleEvent};
    let bus = Bus::new();
    let server = serve(
        &bus,
        Arc::new(Mutex::new(Events::default())),
        Duration::ZERO,
    );
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let (_publisher, mut inputs, mut visible) =
            prepare_visible(&client, &mut signals, lease()).await;
        server
            .emit_signal(
                None::<&str>,
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "ActionInvoked",
                &(7u32, "default"),
            )
            .unwrap();
        server
            .emit_signal(
                None::<&str>,
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "NotificationClosed",
                &(7u32, 2u32),
            )
            .unwrap();
        // Flush both preceding messages through the real broker before polling
        // the consumer, so this covers a queued action-and-close burst.
        server
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "GetId",
                &(),
            )
            .unwrap();
        Timer::after(Duration::from_millis(30)).await;
        assert_eq!(
            visible.next(&mut signals, &mut inputs).await.unwrap(),
            VisibleEvent::OpenPhone
        );
        assert_eq!(
            visible.next(&mut signals, &mut inputs).await.unwrap(),
            VisibleEvent::End(EndReason::Dismissed)
        );
    });
}

#[test]
fn visible_matching_dismissal_is_distinct_from_server_expiry() {
    use omarchy_phone_core::notification_visible::{EndReason, VisibleEvent};
    for (reason, expected) in [(2u32, EndReason::Dismissed), (1, EndReason::ServerClosed)] {
        let bus = Bus::new();
        let events = Arc::new(Mutex::new(Events::default()));
        let server = serve(&bus, events.clone(), Duration::ZERO);
        block_on(async {
            let client = NotificationBus::connect(&bus.address).await.unwrap();
            let mut signals = client.subscribe().await.unwrap();
            let (_publisher, mut inputs, mut visible) =
                prepare_visible(&client, &mut signals, lease()).await;
            server
                .emit_signal(
                    None::<&str>,
                    "/org/freedesktop/Notifications",
                    "org.freedesktop.Notifications",
                    "NotificationClosed",
                    &(7u32, reason),
                )
                .unwrap();
            assert_eq!(
                visible.next(&mut signals, &mut inputs).await.unwrap(),
                VisibleEvent::End(expected)
            );
            drop(visible.into_parts());
            assert!(events.lock().unwrap().closed.is_empty());
        });
    }
}

#[test]
fn visible_owner_loss_keeps_cleanup_on_original_server() {
    use omarchy_phone_core::notification_visible::{EndReason, VisibleEvent};
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let server = serve(&bus, events.clone(), Duration::ZERO);
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let (_publisher, mut inputs, mut visible) =
            prepare_visible(&client, &mut signals, lease()).await;
        server
            .release_name("org.freedesktop.Notifications")
            .unwrap();
        assert_eq!(
            visible.next(&mut signals, &mut inputs).await.unwrap(),
            VisibleEvent::End(EndReason::OwnerChanged)
        );
        let (delivery, _, owner) = visible.into_parts();
        assert_eq!(owner, None);
        delivery.close().await.unwrap();
        assert_eq!(events.lock().unwrap().closed, [7]);
    });
}

#[test]
fn visible_input_loss_withdraws_authority_and_retains_cleanup_handle() {
    use omarchy_phone_core::notification_visible::{EndReason, VisibleEvent};
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::ZERO);
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let (publisher, mut inputs, mut visible) =
            prepare_visible(&client, &mut signals, lease()).await;
        drop(publisher);
        assert!(visible.next(&mut signals, &mut inputs).await.is_err());
        assert_eq!(
            visible.next(&mut signals, &mut inputs).await.unwrap(),
            VisibleEvent::End(EndReason::Revoked)
        );
        visible.into_parts().0.close().await.unwrap();
        assert_eq!(events.lock().unwrap().closed, [7]);
    });
}

#[test]
fn visible_notification_rejects_another_connections_signal_stream() {
    let bus = Bus::new();
    let _server = serve(
        &bus,
        Arc::new(Mutex::new(Events::default())),
        Duration::ZERO,
    );
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let (_publisher, mut inputs, mut visible) =
            prepare_visible(&client, &mut signals, lease()).await;
        let other = NotificationBus::connect(&bus.address).await.unwrap();
        let mut foreign = other.subscribe().await.unwrap();
        let result = future::or(
            async { Some(visible.next(&mut foreign, &mut inputs).await) },
            async {
                Timer::after(Duration::from_millis(200)).await;
                None
            },
        )
        .await;
        assert!(
            result
                .expect("foreign connection must be rejected immediately")
                .is_err()
        );
        visible.into_parts().0.close().await.unwrap();
    });
}

#[test]
fn notification_signals_reject_other_senders_and_unapproved_actions() {
    use omarchy_phone_core::notification_bus::{CloseReason, NotificationEvent};
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let server = serve(&bus, events, Duration::ZERO);
    let attacker = Builder::address(bus.address.as_str())
        .unwrap()
        .build()
        .unwrap();
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let owner = test_owner(&client).await.unwrap();
        for peer in [&attacker, &server] {
            peer.emit_signal(
                None::<&str>,
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "ActionInvoked",
                &(
                    99u32,
                    if peer.unique_name() == server.unique_name() {
                        "answer"
                    } else {
                        "default"
                    },
                ),
            )
            .unwrap();
        }
        server
            .emit_signal(
                None::<&str>,
                "/org/freedesktop/Wrong",
                "org.freedesktop.Notifications",
                "ActionInvoked",
                &(99u32, "default"),
            )
            .unwrap();
        attacker
            .emit_signal(
                None::<&str>,
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "NotificationClosed",
                &(99u32, 2u32),
            )
            .unwrap();
        server
            .emit_signal(
                None::<&str>,
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "ActionInvoked",
                &(7u32, "default"),
            )
            .unwrap();
        server
            .emit_signal(
                None::<&str>,
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "NotificationClosed",
                &(7u32, 2u32),
            )
            .unwrap();
        assert_eq!(
            signals.next(Some(&owner)).await.unwrap(),
            NotificationEvent::OpenPhone { id: 7 }
        );
        assert_eq!(
            signals.next(Some(&owner)).await.unwrap(),
            NotificationEvent::Closed {
                id: 7,
                reason: CloseReason::Dismissed
            }
        );
    });
}

#[test]
fn subscription_preserves_owner_changes_before_discovery_completes() {
    use omarchy_phone_core::notification_bus::NotificationEvent;
    let bus = Bus::new();
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let server = serve(
            &bus,
            Arc::new(Mutex::new(Events::default())),
            Duration::ZERO,
        );
        let owner = test_owner(&client).await.unwrap();
        assert_eq!(
            signals.next(None).await.unwrap(),
            NotificationEvent::OwnerChanged {
                old_owner: String::new(),
                new_owner: owner.clone()
            }
        );
        server
            .release_name("org.freedesktop.Notifications")
            .unwrap();
        let replacement = serve(
            &bus,
            Arc::new(Mutex::new(Events::default())),
            Duration::ZERO,
        );
        assert_eq!(
            signals.next(Some(&owner)).await.unwrap(),
            NotificationEvent::OwnerChanged {
                old_owner: owner,
                new_owner: String::new()
            }
        );
        assert_eq!(
            signals.next(None).await.unwrap(),
            NotificationEvent::OwnerChanged {
                old_owner: String::new(),
                new_owner: replacement.unique_name().unwrap().to_string()
            }
        );
    });
}

#[test]
fn dismissal_signal_before_notify_reply_keeps_the_late_result_revoked() {
    use omarchy_phone_core::notification_bus::{CloseReason, NotificationEvent};
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let server = serve(&bus, events.clone(), Duration::from_millis(400));
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let owner = test_owner(&client).await.unwrap();
        let lease = lease();
        let mut state = Notifications::default();
        let request = state.begin(&owner, &lease, true, now()).unwrap();
        let mut pending = Box::pin(client.notify(&request));
        let early = future::or(async { Some(pending.as_mut().await) }, async {
            Timer::after(Duration::from_millis(100)).await;
            None
        })
        .await;
        assert!(early.is_none());
        assert_eq!(events.lock().unwrap().notified, 1);
        server
            .emit_signal(
                None::<&str>,
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "NotificationClosed",
                &(7u32, 2u32),
            )
            .unwrap();
        assert_eq!(
            signals.next(Some(&owner)).await.unwrap(),
            NotificationEvent::Closed {
                id: 7,
                reason: CloseReason::Dismissed
            }
        );
        state.invalidate();
        let delivered = pending.await.unwrap();
        assert!(matches!(
            state.complete(
                request,
                delivered.id(),
                Some(&owner),
                Some(&lease),
                true,
                now()
            ),
            omarchy_phone_core::notification::Completion::Close(_)
        ));
        delivered.close().await.unwrap();
        assert_eq!(events.lock().unwrap().closed, [7]);
    });
}

#[test]
fn bus_disconnect_ends_signal_stream_without_an_event() {
    let mut bus = Bus::new();
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        bus.child.kill().unwrap();
        bus.child.wait().unwrap();
        let ended = future::or(async { Some(signals.next(None).await) }, async {
            Timer::after(Duration::from_secs(2)).await;
            None
        })
        .await;
        assert!(ended.expect("disconnect must end the stream").is_err());
    });
}

#[test]
fn coalesced_policy_loss_cleans_up_a_real_pending_notification() {
    use omarchy_phone_core::notification_inputs::presentation_channel;
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::from_millis(400));
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let owner = test_owner(&client).await.unwrap();
        let lease = lease();
        let (mut publisher, mut inputs) = presentation_channel();
        let mut guard = Notifications::default();
        publisher.publish(Some(lease.clone()), true).unwrap();
        let first = inputs.receive(&mut guard).await.unwrap();
        let request = guard
            .begin(&owner, first.lease().unwrap(), first.allowed(), now())
            .unwrap();
        let mut pending = Box::pin(client.notify(&request));
        assert!(
            future::or(async { Some(pending.as_mut().await) }, async {
                Timer::after(Duration::from_millis(100)).await;
                None
            },)
            .await
            .is_none()
        );
        publisher.publish(Some(lease.clone()), false).unwrap();
        publisher.publish(Some(lease), true).unwrap();
        let latest = inputs.receive(&mut guard).await.unwrap();
        assert!(latest.allowed());
        let delivered = pending.await.unwrap();
        assert!(matches!(
            guard.complete(
                request,
                delivered.id(),
                Some(&owner),
                latest.lease(),
                latest.allowed(),
                now()
            ),
            omarchy_phone_core::notification::Completion::Close(_)
        ));
        delivered.close().await.unwrap();
        assert_eq!(events.lock().unwrap().notified, 1);
        assert_eq!(events.lock().unwrap().closed, [7]);
    });
}

#[test]
fn boottime_expiry_runs_while_notify_is_pending_and_late_id_is_closed() {
    use omarchy_phone_core::lease_timer::LeaseTimer;
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::from_millis(500));
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let owner = test_owner(&client).await.unwrap();
        let lease = lease_at(now() - Duration::from_millis(4800));
        let mut guard = Notifications::default();
        let request = guard.begin(&owner, &lease, true, now()).unwrap();
        let timer = LeaseTimer::new().unwrap().into_async().unwrap();
        timer.arm(Some(lease.expires())).unwrap();
        let mut pending = Box::pin(client.notify(&request));
        assert!(
            future::or(async { Some(pending.as_mut().await) }, async {
                timer.expired().await.unwrap();
                None
            })
            .await
            .is_none()
        );
        assert!(now() >= lease.expires());
        assert_eq!(events.lock().unwrap().notified, 1);
        guard.invalidate();
        assert!(guard.begin(&owner, &self::lease(), true, now()).is_none());
        let delivered = pending.await.unwrap();
        assert!(matches!(
            guard.complete(
                request,
                delivered.id(),
                Some(&owner),
                Some(&lease),
                true,
                now()
            ),
            omarchy_phone_core::notification::Completion::Close(_)
        ));
        delivered.close().await.unwrap();
        assert_eq!(events.lock().unwrap().closed, [7]);
        assert_eq!(events.lock().unwrap().notified, 1);
    });
}

#[test]
fn pending_loop_processes_policy_loss_without_cancelling_the_reply() {
    use omarchy_phone_core::{
        notification_inputs::presentation_channel, notification_pending::wait_pending,
    };
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::from_millis(400));
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let owner = test_owner(&client).await.unwrap();
        let (mut publisher, mut inputs) = presentation_channel();
        let mut guard = Notifications::default();
        let lease = lease();
        publisher.publish(Some(lease.clone()), true).unwrap();
        let first = inputs.receive(&mut guard).await.unwrap();
        let request = guard.begin(&owner, &lease, true, now()).unwrap();
        let (result, ()) = future::zip(
            wait_pending(
                &client,
                &mut signals,
                &mut inputs,
                &mut guard,
                request,
                first,
            ),
            async {
                Timer::after(Duration::from_millis(100)).await;
                publisher.publish(Some(lease.clone()), false).unwrap();
                publisher.publish(Some(lease), true).unwrap();
            },
        )
        .await;
        let result = result.unwrap();
        assert!(matches!(
            result.completion,
            omarchy_phone_core::notification::Completion::Close(_)
        ));
        assert!(!result.open_requested);
        result.delivery.close().await.unwrap();
        assert_eq!(events.lock().unwrap().notified, 1);
        assert_eq!(events.lock().unwrap().closed, [7]);
    });
}

#[test]
fn pending_loop_matches_early_signals_to_its_own_id() {
    use omarchy_phone_core::{
        notification_inputs::presentation_channel, notification_pending::wait_pending,
    };
    for (dismissed_id, reason, action_first) in [
        (7u32, 2u32, false),
        (99, 2, false),
        (7, 1, false),
        (7, 2, true),
        (7, 1, true),
    ] {
        let bus = Bus::new();
        let events = Arc::new(Mutex::new(Events::default()));
        let server = serve(&bus, events.clone(), Duration::from_millis(400));
        block_on(async {
            let client = NotificationBus::connect(&bus.address).await.unwrap();
            let mut signals = client.subscribe().await.unwrap();
            let owner = test_owner(&client).await.unwrap();
            let (mut publisher, mut inputs) = presentation_channel();
            let mut guard = Notifications::default();
            publisher.publish(Some(lease()), true).unwrap();
            let first = inputs.receive(&mut guard).await.unwrap();
            let request = guard
                .begin(&owner, first.lease().unwrap(), true, now())
                .unwrap();
            let (result, ()) = future::zip(
                wait_pending(
                    &client,
                    &mut signals,
                    &mut inputs,
                    &mut guard,
                    request,
                    first,
                ),
                async {
                    Timer::after(Duration::from_millis(100)).await;
                    if action_first {
                        server
                            .emit_signal(
                                None::<&str>,
                                "/org/freedesktop/Notifications",
                                "org.freedesktop.Notifications",
                                "ActionInvoked",
                                &(7u32, "default"),
                            )
                            .unwrap();
                    }
                    server
                        .emit_signal(
                            None::<&str>,
                            "/org/freedesktop/Notifications",
                            "org.freedesktop.Notifications",
                            "NotificationClosed",
                            &(dismissed_id, reason),
                        )
                        .unwrap();
                    server
                        .emit_signal(
                            None::<&str>,
                            "/org/freedesktop/Notifications",
                            "org.freedesktop.Notifications",
                            "ActionInvoked",
                            &(7u32, "default"),
                        )
                        .unwrap();
                },
            )
            .await;
            let result = result.unwrap();
            assert_eq!(
                matches!(
                    result.completion,
                    omarchy_phone_core::notification::Completion::Close(_)
                ),
                dismissed_id == 7
            );
            assert_eq!(result.open_requested, dismissed_id != 7 || action_first);
            assert_eq!(result.dismissed, dismissed_id == 7 && reason == 2);
            result.delivery.close().await.unwrap();
        });
    }
}

#[test]
fn pending_loop_expires_without_inputs_and_closes_the_late_result() {
    use omarchy_phone_core::{
        notification_inputs::presentation_channel, notification_pending::wait_pending,
    };
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::from_millis(500));
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let owner = test_owner(&client).await.unwrap();
        let (mut publisher, mut inputs) = presentation_channel();
        let mut guard = Notifications::default();
        publisher
            .publish(Some(lease_at(now() - Duration::from_millis(4800))), true)
            .unwrap();
        let first = inputs.receive(&mut guard).await.unwrap();
        let request = guard
            .begin(&owner, first.lease().unwrap(), true, now())
            .unwrap();
        let result = wait_pending(
            &client,
            &mut signals,
            &mut inputs,
            &mut guard,
            request,
            first,
        )
        .await
        .unwrap();
        assert!(matches!(
            result.completion,
            omarchy_phone_core::notification::Completion::Close(_)
        ));
        result.delivery.close().await.unwrap();
        assert_eq!(events.lock().unwrap().closed, [7]);
    });
}

#[test]
fn pending_loop_exits_on_producer_loss_even_if_notify_never_replies() {
    use omarchy_phone_core::{
        notification_inputs::presentation_channel, notification_pending::wait_pending,
    };
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::MAX);
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let owner = test_owner(&client).await.unwrap();
        let (mut publisher, mut inputs) = presentation_channel();
        let mut guard = Notifications::default();
        publisher.publish(Some(lease()), true).unwrap();
        let first = inputs.receive(&mut guard).await.unwrap();
        let request = guard
            .begin(&owner, first.lease().unwrap(), true, now())
            .unwrap();
        let started = Instant::now();
        let (result, ()) = future::zip(
            wait_pending(
                &client,
                &mut signals,
                &mut inputs,
                &mut guard,
                request,
                first,
            ),
            async move {
                Timer::after(Duration::from_millis(100)).await;
                drop(publisher);
            },
        )
        .await;
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(guard.begin(&owner, &lease(), true, now()).is_none());
        drop(signals);
        drop(client);
        assert_eq!(events.lock().unwrap().notified, 1);
        // Shutdown with no returned ID cannot prove remote cleanup.
        assert!(events.lock().unwrap().closed.is_empty());
    });
}

#[test]
fn pending_loop_keeps_signal_ids_scoped_to_the_original_owner() {
    use omarchy_phone_core::{
        notification_inputs::presentation_channel, notification_pending::wait_pending,
    };
    for signal_from_old in [false, true] {
        let bus = Bus::new();
        let events = Arc::new(Mutex::new(Events::default()));
        let old = serve(&bus, events.clone(), Duration::from_millis(500));
        block_on(async {
            let client = NotificationBus::connect(&bus.address).await.unwrap();
            let mut signals = client.subscribe().await.unwrap();
            let owner = test_owner(&client).await.unwrap();
            let (mut publisher, mut inputs) = presentation_channel();
            let mut guard = Notifications::default();
            publisher.publish(Some(lease()), true).unwrap();
            let first = inputs.receive(&mut guard).await.unwrap();
            let request = guard
                .begin(&owner, first.lease().unwrap(), true, now())
                .unwrap();
            let new_events = Arc::new(Mutex::new(Events::default()));
            let (result, replacement) = future::zip(
                wait_pending(
                    &client,
                    &mut signals,
                    &mut inputs,
                    &mut guard,
                    request,
                    first,
                ),
                async {
                    Timer::after(Duration::from_millis(100)).await;
                    old.release_name("org.freedesktop.Notifications").unwrap();
                    let replacement = serve(&bus, new_events.clone(), Duration::ZERO);
                    let emitter = if signal_from_old { &old } else { &replacement };
                    emitter
                        .emit_signal(
                            None::<&str>,
                            "/org/freedesktop/Notifications",
                            "org.freedesktop.Notifications",
                            "NotificationClosed",
                            &(7u32, 2u32),
                        )
                        .unwrap();
                    replacement
                },
            )
            .await;
            let result = result.unwrap();
            assert!(matches!(
                result.completion,
                omarchy_phone_core::notification::Completion::Close(_)
            ));
            assert_eq!(result.dismissed, signal_from_old);
            assert_eq!(
                result.owner.as_deref(),
                replacement.unique_name().map(|n| n.as_str())
            );
            result.delivery.close().await.unwrap();
            assert_eq!(events.lock().unwrap().closed, [7]);
            assert!(new_events.lock().unwrap().closed.is_empty());
        });
    }
}

#[test]
fn ambiguous_notify_is_not_repeated_after_reconnect_but_a_new_call_notifies() {
    use omarchy_phone_core::notification_policy::{BannerAttempts, Dismissals};
    let bus = Bus::new();
    let old_events = Arc::new(Mutex::new(Events::default()));
    let old_server = serve(&bus, old_events.clone(), Duration::MAX);
    block_on(async {
        let mut attempts = BannerAttempts::default();
        let dismissals = Dismissals::default();
        let original = lease();
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = client.subscribe().await.unwrap();
        let owner = client.discover(&mut signals).await.unwrap().unwrap();
        let mut guard = Notifications::default();
        let request = guard.begin(&owner, &original, true, now()).unwrap();
        assert!(attempts.claim(original.snapshot(), &dismissals));
        let mut pending = Box::pin(client.notify(&request));
        let result = future::or(async { Some(pending.as_mut().await) }, async {
            Timer::after(Duration::from_millis(100)).await;
            None
        })
        .await;
        assert!(result.is_none());
        assert_eq!(old_events.lock().unwrap().notified, 1);
        // The server accepted Notify, but the client never learned its ID.
        // Aborting the connection cannot prove that no banner was displayed.
        drop(pending);
        drop(signals);
        drop(client);
        let reconnected = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = reconnected.subscribe().await.unwrap();
        assert_eq!(
            reconnected.discover(&mut signals).await.unwrap().as_deref(),
            Some(owner.as_str())
        );
        assert!(!attempts.claim(original.snapshot(), &dismissals));
        old_server
            .release_name("org.freedesktop.Notifications")
            .unwrap();
        let new_events = Arc::new(Mutex::new(Events::default()));
        let replacement = serve(&bus, new_events.clone(), Duration::ZERO);
        assert!(!attempts.claim(original.snapshot(), &dismissals));
        let start = now();
        let observer = observer_with_episode(start, "cccccccccccccccccccccccccccccccc");
        let fresh = observer.lease(start).unwrap();
        let mut guard = Notifications::default();
        let request = guard
            .begin(
                replacement.unique_name().unwrap().as_str(),
                fresh,
                true,
                now(),
            )
            .unwrap();
        assert!(attempts.claim(fresh.snapshot(), &dismissals));
        let delivered = reconnected.notify(&request).await.unwrap();
        assert_eq!(old_events.lock().unwrap().notified, 1);
        assert_eq!(new_events.lock().unwrap().notified, 1);
        delivered.close().await.unwrap();
    });
}

#[test]
fn pending_loop_bounds_unknown_delivery_despite_fresh_inputs() {
    use omarchy_phone_core::{
        notification_inputs::presentation_channel, notification_pending::wait_pending,
    };
    for renew in [false, true] {
        let bus = Bus::new();
        let events = Arc::new(Mutex::new(Events::default()));
        let _server = serve(&bus, events.clone(), Duration::MAX);
        block_on(async {
            let client = NotificationBus::connect(&bus.address).await.unwrap();
            let mut signals = client.subscribe().await.unwrap();
            let owner = client.discover(&mut signals).await.unwrap().unwrap();
            let (mut publisher, mut inputs) = presentation_channel();
            let mut guard = Notifications::default();
            let original = lease_at(now() - Duration::from_millis(4800));
            publisher.publish(Some(original.clone()), true).unwrap();
            let first = inputs.receive(&mut guard).await.unwrap();
            let request = guard.begin(&owner, &original, true, now()).unwrap();
            let started = Instant::now();
            let result = future::or(
                async {
                    Some(
                        future::or(
                            wait_pending(
                                &client,
                                &mut signals,
                                &mut inputs,
                                &mut guard,
                                request,
                                first,
                            ),
                            async {
                                loop {
                                    Timer::after(Duration::from_millis(40)).await;
                                    if renew {
                                        publisher.publish(Some(lease()), true).unwrap();
                                    }
                                }
                            },
                        )
                        .await,
                    )
                },
                async {
                    Timer::after(Duration::from_secs(3)).await;
                    None
                },
            )
            .await
            .expect("unresolved Notify must end after its bounded cleanup window");
            let error = match result {
                Ok(_) => panic!("server never returned a notification ID"),
                Err(error) => error,
            };
            assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
            assert!(started.elapsed() >= Duration::from_millis(1400));
            assert!(started.elapsed() < Duration::from_millis(2500));
            assert_eq!(events.lock().unwrap().notified, 1);
            assert!(events.lock().unwrap().closed.is_empty());
            // A duplicate subscription can be satisfied by zbus's cached
            // match rule. Owner discovery performs an actual bus round trip.
            assert!(
                client.discover(&mut signals).await.is_err(),
                "timeout must abort the original connection"
            );
            // The old generation remains unusable; only a new generation can
            // get a free slot, retaining the service's per-episode attempt state.
            assert!(guard.begin(&owner, &lease(), true, now()).is_none());
        });
    }
}

#[test]
fn generation_forwards_click_then_closes_on_input_loss() {
    use omarchy_phone_core::{
        notification_generation::{Notice, run_generation},
        notification_inputs::presentation_channel,
        notification_policy::{BannerAttempts, Dismissals},
    };
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let server = serve(&bus, events.clone(), Duration::ZERO);
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let (mut publisher, mut inputs) = presentation_channel();
        publisher.publish(Some(lease()), true).unwrap();
        let (sender, receiver) = omarchy_phone_core::notification_events::notice_channel().unwrap();
        let receiver = async_io::Async::new(receiver).unwrap();
        let mut attempts = BannerAttempts::default();
        let mut claim = |snapshot: &omarchy_phone_core::watch::Snapshot| {
            Ok(attempts.claim(snapshot, &Dismissals::default()))
        };
        let mut emit = |notice| sender.send(notice);
        let (result, notices) = future::zip(
            run_generation(&client, &mut inputs, &mut claim, &mut emit),
            async {
                let deadline = Instant::now() + Duration::from_secs(2);
                while events.lock().unwrap().notified == 0 {
                    assert!(Instant::now() < deadline, "generation must dispatch Notify");
                    Timer::after(Duration::from_millis(5)).await;
                }
                server
                    .emit_signal(
                        None::<&str>,
                        "/org/freedesktop/Notifications",
                        "org.freedesktop.Notifications",
                        "ActionInvoked",
                        &(7u32, "default"),
                    )
                    .unwrap();
                let batch = future::or(
                    async {
                        Some(
                            receiver
                                .read_with(|queue| {
                                    let batch = queue.drain()?;
                                    if batch.notices.is_empty() && !batch.closed {
                                        Err(std::io::ErrorKind::WouldBlock.into())
                                    } else {
                                        Ok(batch)
                                    }
                                })
                                .await
                                .unwrap(),
                        )
                    },
                    async {
                        Timer::after(Duration::from_secs(2)).await;
                        None
                    },
                )
                .await
                .expect("real notice queue must wake on Open Phone");
                assert!(!batch.closed);
                drop(publisher);
                batch.notices
            },
        )
        .await;
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::BrokenPipe);
        assert!(matches!(notices.as_slice(), [Notice::OpenPhone]));
        assert_eq!(events.lock().unwrap().closed, [7]);
    });
}

#[test]
fn generation_waits_for_owner_and_forwards_dismissal_without_duplicate_close() {
    exercise_generation_dismissal(false);
}

#[test]
fn generation_does_not_close_an_already_closed_banner_when_event_delivery_fails() {
    exercise_generation_dismissal(true);
}

fn exercise_generation_dismissal(reject_event: bool) {
    use omarchy_phone_core::{
        notification_generation::{Notice, run_generation},
        notification_inputs::presentation_channel,
        notification_policy::{BannerAttempts, Dismissals},
    };
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let (mut publisher, mut inputs) = presentation_channel();
        let initial = lease();
        publisher.publish(Some(initial.clone()), true).unwrap();
        let notices = Arc::new(Mutex::new(Vec::new()));
        let mut attempts = BannerAttempts::default();
        let mut claim = |snapshot: &omarchy_phone_core::watch::Snapshot| {
            Ok(attempts.claim(snapshot, &Dismissals::default()))
        };
        let mut emit = |notice| {
            notices.lock().unwrap().push(notice);
            if reject_event {
                Err(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "event queue full",
                ))
            } else {
                Ok(())
            }
        };
        let (result, ()) = future::zip(
            run_generation(&client, &mut inputs, &mut claim, &mut emit),
            async {
                Timer::after(Duration::from_millis(40)).await;
                let server = serve(&bus, events.clone(), Duration::ZERO);
                let deadline = Instant::now() + Duration::from_secs(2);
                while events.lock().unwrap().notified == 0 {
                    assert!(
                        Instant::now() < deadline,
                        "new owner must enable first attempt"
                    );
                    Timer::after(Duration::from_millis(5)).await;
                }
                server
                    .emit_signal(
                        None::<&str>,
                        "/org/freedesktop/Notifications",
                        "org.freedesktop.Notifications",
                        "NotificationClosed",
                        &(7u32, 2u32),
                    )
                    .unwrap();
                while notices.lock().unwrap().is_empty() {
                    assert!(
                        Instant::now() < deadline,
                        "dismissal must reach service owner"
                    );
                    Timer::after(Duration::from_millis(5)).await;
                }
                publisher.publish(Some(initial.clone()), true).unwrap();
                Timer::after(Duration::from_millis(40)).await;
                assert_eq!(events.lock().unwrap().notified, 1);
                drop(publisher);
            },
        )
        .await;
        assert_eq!(
            result.unwrap_err().kind(),
            if reject_event {
                std::io::ErrorKind::WouldBlock
            } else {
                std::io::ErrorKind::BrokenPipe
            }
        );
        let notices = notices.lock().unwrap();
        let [Notice::Dismissed(presented)] = notices.as_slice() else {
            panic!("expected one dismissal")
        };
        assert_eq!(presented.cursor(), initial.snapshot().cursor());
        assert_eq!(
            presented.ringing()[0].episode(),
            initial.snapshot().ringing()[0].episode()
        );
        assert!(
            events.lock().unwrap().closed.is_empty(),
            "server already closed this ID"
        );
    });
}

#[test]
fn generation_expires_one_banner_and_handles_a_new_call_without_repeating_old_call() {
    use omarchy_phone_core::{
        notification_generation::run_generation,
        notification_inputs::presentation_channel,
        notification_policy::{BannerAttempts, Dismissals},
    };
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::ZERO);
    block_on(async {
        let client = NotificationBus::connect(&bus.address).await.unwrap();
        let (mut publisher, mut inputs) = presentation_channel();
        publisher
            .publish(Some(lease_at(now() - Duration::from_millis(4800))), true)
            .unwrap();
        let mut attempts = BannerAttempts::default();
        let mut claim = |snapshot: &omarchy_phone_core::watch::Snapshot| {
            Ok(attempts.claim(snapshot, &Dismissals::default()))
        };
        let mut emit = |_| panic!("expiry and shutdown are not user actions");
        let (result, ()) = future::zip(
            run_generation(&client, &mut inputs, &mut claim, &mut emit),
            async {
                let deadline = Instant::now() + Duration::from_secs(2);
                while events.lock().unwrap().closed.is_empty() {
                    assert!(
                        Instant::now() < deadline,
                        "banner must expire without new inputs"
                    );
                    Timer::after(Duration::from_millis(5)).await;
                }
                publisher.publish(Some(lease()), true).unwrap();
                Timer::after(Duration::from_millis(40)).await;
                assert_eq!(
                    events.lock().unwrap().notified,
                    1,
                    "fresh same episode must not repeat"
                );
                let start = now();
                let new_call = observer_with_episode(start, "cccccccccccccccccccccccccccccccc");
                publisher
                    .publish(new_call.lease(start).cloned(), true)
                    .unwrap();
                while events.lock().unwrap().notified < 2 {
                    assert!(Instant::now() < deadline, "new episode must notify");
                    Timer::after(Duration::from_millis(5)).await;
                }
                drop(publisher);
            },
        )
        .await;
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::BrokenPipe);
        assert_eq!(events.lock().unwrap().closed, [7, 7]);
    });
}

#[test]
fn supervisor_reconnects_after_bus_replacement_without_repeating_unknown_delivery() {
    use omarchy_phone_core::{
        notification_inputs::presentation_channel,
        notification_policy::{BannerAttempts, Dismissals},
        notification_supervisor::run_notifications,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    let address = format!(
        "unix:abstract=phone-supervisor-{}",
        uuid::Uuid::new_v4().simple()
    );
    let mut bus = Bus::at(Some(&address));
    let old_events = Arc::new(Mutex::new(Events::default()));
    let old_server = serve(&bus, old_events.clone(), Duration::MAX);
    block_on(async {
        let (mut publisher, mut inputs) = presentation_channel();
        publisher.publish(Some(lease()), true).unwrap();
        let claims = AtomicUsize::new(0);
        let mut attempts = BannerAttempts::default();
        let mut claim = |snapshot: &omarchy_phone_core::watch::Snapshot| {
            claims.fetch_add(1, Ordering::SeqCst);
            Ok(attempts.claim(snapshot, &Dismissals::default()))
        };
        let mut emit = |_| panic!("no user action was sent");
        let (result, ()) = future::zip(
            run_notifications(&address, &mut inputs, &mut claim, &mut emit),
            async {
                let deadline = Instant::now() + Duration::from_secs(5);
                while old_events.lock().unwrap().notified == 0 {
                    assert!(Instant::now() < deadline, "first Notify deadline");
                    Timer::after(Duration::from_millis(5)).await;
                }
                bus.child.kill().unwrap();
                bus.child.wait().unwrap();
                drop(old_server);
                let replacement_bus = Bus::at(Some(&address));
                let new_events = Arc::new(Mutex::new(Events::default()));
                let _replacement = serve(&replacement_bus, new_events.clone(), Duration::ZERO);
                while claims.load(Ordering::SeqCst) < 2 {
                    assert!(
                        Instant::now() < deadline,
                        "supervisor must reconnect and reevaluate"
                    );
                    publisher.publish(Some(lease()), true).unwrap();
                    Timer::after(Duration::from_millis(40)).await;
                }
                assert_eq!(old_events.lock().unwrap().notified, 1);
                assert_eq!(
                    new_events.lock().unwrap().notified,
                    0,
                    "unknown old delivery must not repeat"
                );
                let start = now();
                let new_call = observer_with_episode(start, "cccccccccccccccccccccccccccccccc");
                publisher
                    .publish(new_call.lease(start).cloned(), true)
                    .unwrap();
                while new_events.lock().unwrap().notified == 0 {
                    assert!(
                        Instant::now() < deadline,
                        "new call must notify after reconnect"
                    );
                    Timer::after(Duration::from_millis(5)).await;
                }
                assert_eq!(new_events.lock().unwrap().notified, 1);
                drop(publisher);
            },
        )
        .await;
        result.unwrap();
    });
}

#[test]
fn supervisor_backs_off_failed_setup_and_stops_when_inputs_close() {
    use omarchy_phone_core::{
        notification_inputs::presentation_channel, notification_supervisor::run_notifications,
    };
    use std::os::{
        linux::net::SocketAddrExt,
        unix::net::{SocketAddr, UnixListener},
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    let name = format!("phone-supervisor-fault-{}", uuid::Uuid::new_v4().simple());
    let listener =
        UnixListener::bind_addr(&SocketAddr::from_abstract_name(name.as_bytes()).unwrap()).unwrap();
    listener.set_nonblocking(true).unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let peer_count = accepted.clone();
    let peer = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(4);
        let mut times = Vec::new();
        while times.len() < 2 {
            match listener.accept() {
                Ok((socket, _)) => {
                    times.push(Instant::now());
                    drop(socket); // fail authentication without a busy-loop fixture
                    peer_count.fetch_add(1, Ordering::SeqCst);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "setup retry deadline");
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("fixture accept: {error}"),
            }
        }
        assert!(times[1].duration_since(times[0]) >= Duration::from_millis(900));
    });
    block_on(async {
        let (publisher, mut inputs) = presentation_channel();
        let mut claim = |_: &omarchy_phone_core::watch::Snapshot| {
            panic!("failed authentication cannot claim a banner")
        };
        let mut emit = |_| panic!("failed authentication cannot emit a notice");
        let address = format!("unix:abstract={name}");
        let started = Instant::now();
        let (result, ()) = future::zip(
            run_notifications(&address, &mut inputs, &mut claim, &mut emit),
            async {
                while accepted.load(Ordering::SeqCst) < 2 {
                    assert!(started.elapsed() < Duration::from_secs(4));
                    Timer::after(Duration::from_millis(5)).await;
                }
                drop(publisher);
            },
        )
        .await;
        result.unwrap();
        assert!(started.elapsed() < Duration::from_millis(3500));
    });
    peer.join().unwrap();
}

#[test]
fn supervisor_reports_permanent_claim_failure_without_retry_or_notify() {
    use omarchy_phone_core::{
        notification_inputs::presentation_channel, notification_supervisor::run_notifications,
    };
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::ZERO);
    block_on(async {
        let (mut publisher, mut inputs) = presentation_channel();
        publisher.publish(Some(lease()), true).unwrap();
        let mut called = 0;
        let mut claim = |_: &omarchy_phone_core::watch::Snapshot| {
            called += 1;
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "presentation disabled",
            ))
        };
        let mut emit = |_| panic!("no notice without a claim");
        let result = future::or(
            async {
                Some(run_notifications(&bus.address, &mut inputs, &mut claim, &mut emit).await)
            },
            async {
                Timer::after(Duration::from_millis(700)).await;
                None
            },
        )
        .await
        .expect("permanent local failure must end supervision");
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert_eq!(called, 1);
        assert_eq!(events.lock().unwrap().notified, 0);
    });
}

#[test]
fn supervisor_stops_and_cleans_up_when_service_event_queue_is_full() {
    use omarchy_phone_core::{
        notification_events::notice_channel, notification_generation::Notice,
        notification_inputs::presentation_channel, notification_supervisor::run_notifications,
    };
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let server = serve(&bus, events.clone(), Duration::ZERO);
    block_on(async {
        let (mut publisher, mut inputs) = presentation_channel();
        publisher.publish(Some(lease()), true).unwrap();
        let (sender, _receiver) = notice_channel().unwrap();
        for _ in 0..16 {
            sender.send(Notice::OpenPhone).unwrap();
        }
        let mut claim = |_: &omarchy_phone_core::watch::Snapshot| Ok(true);
        let mut emit = |notice| sender.send(notice);
        let work = future::zip(
            run_notifications(&bus.address, &mut inputs, &mut claim, &mut emit),
            async {
                let deadline = Instant::now() + Duration::from_secs(2);
                while events.lock().unwrap().notified == 0 {
                    assert!(Instant::now() < deadline);
                    Timer::after(Duration::from_millis(5)).await;
                }
                server
                    .emit_signal(
                        None::<&str>,
                        "/org/freedesktop/Notifications",
                        "org.freedesktop.Notifications",
                        "ActionInvoked",
                        &(7u32, "default"),
                    )
                    .unwrap();
            },
        );
        let (result, ()) = future::or(async { Some(work.await) }, async {
            Timer::after(Duration::from_secs(2)).await;
            None
        })
        .await
        .expect("full event queue must terminate supervision");
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
        assert_eq!(events.lock().unwrap().notified, 1);
        assert_eq!(events.lock().unwrap().closed, [7]);
    });
}

#[test]
fn service_worker_shutdown_closes_visible_banner_and_event_stream() {
    use omarchy_phone_core::notification_worker::NotificationWorker;
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::ZERO);
    let mut worker = NotificationWorker::start(bus.address.clone(), |_| Ok(true)).unwrap();
    worker.publish(Some(lease()), true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while events.lock().unwrap().notified == 0 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    worker.shutdown().unwrap();
    assert_eq!(events.lock().unwrap().closed, [7]);
    assert!(worker.events().drain().unwrap().closed);
    assert!(worker.is_finished());
    assert_eq!(
        worker.publish(None, false).unwrap_err().kind(),
        std::io::ErrorKind::BrokenPipe
    );
    worker.shutdown().unwrap();
}

#[test]
fn service_worker_surfaces_policy_failure_and_wakes_event_receiver() {
    use omarchy_phone_core::notification_worker::NotificationWorker;
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::ZERO);
    let mut worker = NotificationWorker::start(bus.address.clone(), |_| {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "policy unavailable",
        ))
    })
    .unwrap();
    worker.publish(Some(lease()), true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !worker.is_finished() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(worker.events().drain().unwrap().closed);
    assert_eq!(
        worker.shutdown().unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(events.lock().unwrap().notified, 0);
}

#[test]
fn dropping_service_worker_joins_and_closes_visible_banner() {
    use omarchy_phone_core::notification_worker::NotificationWorker;
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let _server = serve(&bus, events.clone(), Duration::ZERO);
    let mut worker = NotificationWorker::start(bus.address.clone(), |_| Ok(true)).unwrap();
    worker.publish(Some(lease()), true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while events.lock().unwrap().notified == 0 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(worker);
    assert_eq!(events.lock().unwrap().closed, [7]);
}

struct PhoneActivationServer {
    calls: Arc<std::sync::atomic::AtomicUsize>,
    accept: bool,
    hold: bool,
}
#[zbus::interface(name = "org.omarchy.Phone")]
impl PhoneActivationServer {
    async fn activate(&self) -> bool {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.hold {
            std::future::pending::<()>().await;
        }
        self.accept
    }
}
fn phone_activation_server(
    bus: &Bus,
    accept: bool,
    hold: bool,
) -> (
    zbus::blocking::Connection,
    Arc<std::sync::atomic::AtomicUsize>,
) {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let server = Builder::address(bus.address.as_str())
        .unwrap()
        .serve_at(
            "/org/omarchy/Phone",
            PhoneActivationServer {
                calls: calls.clone(),
                accept,
                hold,
            },
        )
        .unwrap()
        .name("org.omarchy.Phone")
        .unwrap()
        .build()
        .unwrap();
    (server, calls)
}
#[test]
fn fixed_phone_activation_calls_once_and_reports_refusal() {
    for accept in [true, false] {
        let bus = Bus::new();
        let (_server, calls) = phone_activation_server(&bus, accept, false);
        let result = block_on(omarchy_phone_core::notification_bus::activate_phone(
            &bus.address,
        ));
        assert_eq!(result.is_ok(), accept);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
#[test]
fn fixed_phone_activation_timeout_does_not_retry_or_break_notification_connection() {
    let bus = Bus::new();
    let (_server, calls) = phone_activation_server(&bus, true, true);
    let events = Arc::new(Mutex::new(Events::default()));
    let _notifier = serve(&bus, events, Duration::ZERO);
    block_on(async {
        let notifications = NotificationBus::connect(&bus.address).await.unwrap();
        let mut signals = notifications.subscribe().await.unwrap();
        let before = notifications.discover(&mut signals).await.unwrap();
        let start = Instant::now();
        let error = omarchy_phone_core::notification_bus::activate_phone(&bus.address)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(3));
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(notifications.discover(&mut signals).await.unwrap(), before);
    });
}

#[test]
fn activation_slot_rejects_overlap_and_wakes_poll_on_completion() {
    use omarchy_phone_core::activation_worker::ActivationSlot;
    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    let bus = Bus::new();
    let (_server, calls) = phone_activation_server(&bus, true, true);
    let mut slot = ActivationSlot::default();
    assert!(slot.completion().is_none());
    slot.start(bus.address.clone()).unwrap();
    assert_eq!(
        slot.start(bus.address.clone()).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(slot.finish().unwrap().is_none());
    let mut fds = [PollFd::new(slot.completion().unwrap(), PollFlags::IN)];
    assert_eq!(
        poll(
            &mut fds,
            Some(&Timespec {
                tv_sec: 4,
                tv_nsec: 0
            })
        )
        .unwrap(),
        1
    );
    assert!(slot.finish().unwrap().unwrap().is_err());
    assert!(slot.completion().is_none());
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn activation_slot_reaps_refusal_then_accepts_a_new_explicit_request() {
    use omarchy_phone_core::activation_worker::ActivationSlot;
    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    let bus = Bus::new();
    let (_server, calls) = phone_activation_server(&bus, false, false);
    let mut slot = ActivationSlot::default();
    for _ in 0..2 {
        slot.start(bus.address.clone()).unwrap();
        let mut fds = [PollFd::new(slot.completion().unwrap(), PollFlags::IN)];
        assert_eq!(
            poll(
                &mut fds,
                Some(&Timespec {
                    tv_sec: 4,
                    tv_nsec: 0
                })
            )
            .unwrap(),
            1
        );
        assert!(slot.finish().unwrap().unwrap().is_err());
        assert!(slot.completion().is_none());
    }
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[test]
fn dropping_activation_slot_joins_held_request_without_retry() {
    use omarchy_phone_core::activation_worker::ActivationSlot;
    let bus = Bus::new();
    let (_server, calls) = phone_activation_server(&bus, true, true);
    let mut slot = ActivationSlot::default();
    slot.start(bus.address.clone()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while calls.load(std::sync::atomic::Ordering::SeqCst) == 0 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    let start = Instant::now();
    drop(slot);
    assert!(
        start.elapsed() >= Duration::from_millis(500),
        "Drop must join the outstanding request"
    );
    assert!(start.elapsed() < Duration::from_secs(3));
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
}

fn presentation_coordinator(
    path: &std::path::Path,
) -> Arc<omarchy_phone_core::coordinator::Coordinator> {
    use sha2::{Digest, Sha256};
    let identity = rcgen::generate_simple_self_signed(vec!["android.test".into()]).unwrap();
    let tls = omarchy_phone_core::tls::TlsClient::new(
        vec![identity.cert.der().clone()],
        vec![identity.cert.der().clone()],
        rustls::pki_types::PrivatePkcs8KeyDer::from(identity.signing_key.serialize_der()).into(),
        "android.test".into(),
        Sha256::digest(identity.cert.der().as_ref()).into(),
    )
    .unwrap();
    Arc::new(
        omarchy_phone_core::coordinator::Coordinator::new(
            omarchy_phone_core::ledger::Ledger::open(path).unwrap(),
            tls,
            "127.0.0.1:1".parse().unwrap(),
        )
        .unwrap(),
    )
}
#[test]
fn presentation_owner_dispatches_click_and_persists_dismissal() {
    use omarchy_phone_core::presentation_owner::PresentationOwner;
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let notifier = serve(&bus, events.clone(), Duration::ZERO);
    let (_phone, activations) = phone_activation_server(&bus, true, false);
    let path = std::env::temp_dir().join(format!("presentation-{}", uuid::Uuid::new_v4().simple()));
    let coordinator = presentation_coordinator(&path);
    let current = lease();
    let mut owner = PresentationOwner::start(bus.address.clone(), coordinator.clone()).unwrap();
    owner.publish(Some(current.clone()), true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while events.lock().unwrap().notified == 0 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    notifier
        .emit_signal(
            None::<&str>,
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "ActionInvoked",
            &(7u32, "default"),
        )
        .unwrap();
    while activations.load(std::sync::atomic::Ordering::SeqCst) == 0 {
        owner.drain_notices().unwrap();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    notifier
        .emit_signal(
            None::<&str>,
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "NotificationClosed",
            &(7u32, 2u32),
        )
        .unwrap();
    while coordinator
        .observe_presentation(current.snapshot())
        .unwrap()
    {
        owner.drain_notices().unwrap();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    owner.shutdown().unwrap();
    drop(owner);
    drop(coordinator);
    let ledger = omarchy_phone_core::ledger::Ledger::open(&path).unwrap();
    assert!(
        !ledger
            .load_dismissals()
            .unwrap()
            .has_undismissed(current.snapshot())
    );
    drop(ledger);
    std::fs::remove_dir_all(path).unwrap();
    assert_eq!(activations.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn presentation_owner_rechecks_policy_before_processing_queued_click() {
    use omarchy_phone_core::presentation_owner::PresentationOwner;
    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let notifier = serve(&bus, events.clone(), Duration::ZERO);
    let (_phone, activations) = phone_activation_server(&bus, true, false);
    let path = std::env::temp_dir().join(format!("presentation-{}", uuid::Uuid::new_v4().simple()));
    let coordinator = presentation_coordinator(&path);
    let mut owner = PresentationOwner::start(bus.address.clone(), coordinator.clone()).unwrap();
    owner.publish(Some(lease()), true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while events.lock().unwrap().notified == 0 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    notifier
        .emit_signal(
            None::<&str>,
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "ActionInvoked",
            &(7u32, "default"),
        )
        .unwrap();
    let mut fds = [PollFd::new(owner.notices(), PollFlags::IN)];
    assert_eq!(
        poll(
            &mut fds,
            Some(&Timespec {
                tv_sec: 2,
                tv_nsec: 0
            })
        )
        .unwrap(),
        1
    );
    owner.publish(None, false).unwrap();
    owner.drain_notices().unwrap();
    assert!(owner.activation_completion().is_none());
    owner.shutdown().unwrap();
    drop(owner);
    drop(coordinator);
    std::fs::remove_dir_all(path).unwrap();
    assert_eq!(activations.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
fn presentation_owner_keeps_queued_dismissal_across_observation_loss() {
    use omarchy_phone_core::presentation_owner::PresentationOwner;
    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    let bus = Bus::new();
    let events = Arc::new(Mutex::new(Events::default()));
    let notifier = serve(&bus, events.clone(), Duration::ZERO);
    let path = std::env::temp_dir().join(format!("presentation-{}", uuid::Uuid::new_v4().simple()));
    let coordinator = presentation_coordinator(&path);
    let current = lease();
    let mut owner = PresentationOwner::start(bus.address.clone(), coordinator.clone()).unwrap();
    owner.publish(Some(current.clone()), true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while events.lock().unwrap().notified == 0 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    notifier
        .emit_signal(
            None::<&str>,
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "NotificationClosed",
            &(7u32, 2u32),
        )
        .unwrap();
    let mut fds = [PollFd::new(owner.notices(), PollFlags::IN)];
    assert_eq!(
        poll(
            &mut fds,
            Some(&Timespec {
                tv_sec: 2,
                tv_nsec: 0
            })
        )
        .unwrap(),
        1
    );
    owner.publish(None, false).unwrap();
    owner.drain_notices().unwrap();
    assert!(
        !coordinator
            .observe_presentation(current.snapshot())
            .unwrap(),
        "queued explicit dismissal must survive observation loss"
    );
    owner.shutdown().unwrap();
    drop(owner);
    drop(coordinator);
    std::fs::remove_dir_all(path).unwrap();
}
