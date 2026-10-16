use pokelinux_render::bootstrap::{
    BootstrapError, BorrowedBootstrapEvent, MAX_BOOTSTRAP_DEVICES, MAX_BOOTSTRAP_EVENTS,
    MAX_BOOTSTRAP_FDS, OwnedBootstrap, SessionRenderEvent,
};
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    mem::{align_of, offset_of, size_of},
    os::{
        fd::{AsFd, AsRawFd, FromRawFd, OwnedFd},
        unix::fs::MetadataExt,
    },
    sync::Mutex,
};
static SERIAL: Mutex<()> = Mutex::new(());
unsafe extern "C" {
    fn fcntl(fd: i32, command: i32, ...) -> i32;
    fn memfd_create(name: *const std::ffi::c_char, flags: u32) -> i32;
}
fn fd_flags(fd: i32) -> i32 {
    unsafe { fcntl(fd, 1) }
}
fn assert_open(file: &File) {
    assert!(fd_flags(file.as_raw_fd()) >= 0);
    assert!(file.metadata().is_ok());
}
fn assert_closed(fd: i32) {
    assert_eq!(fd_flags(fd), -1);
    assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(9));
}
fn character(name: &str) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(name)
        .unwrap()
}
fn memfd() -> File {
    let fd = unsafe { memfd_create(c"owned-bootstrap-test".as_ptr(), 1) };
    assert!(fd >= 0);
    File::from(unsafe { OwnedFd::from_raw_fd(fd) })
}
fn text<const N: usize>(target: &mut [u8; N], value: &[u8]) {
    assert!(value.len() < N);
    target[..value.len()].copy_from_slice(value);
    target[value.len()] = 0;
}
fn fd_set() -> BTreeSet<i32> {
    std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .map(|p| p.unwrap().file_name().to_string_lossy().parse().unwrap())
        .collect()
}
struct Fixture {
    metadata: Vec<SessionRenderEvent>,
    files: Vec<File>,
}
impl Fixture {
    fn new() -> Self {
        let files = vec![character("/dev/null"), character("/dev/zero")];
        let mut init = SessionRenderEvent {
            event_type: 1,
            session_sequence: 1,
            session_generation: 7,
            active: 1,
            vt: 9,
            ..SessionRenderEvent::default()
        };
        text(&mut init.seat_name, b"seat0");
        init.seat_name[20] = 0xfe;
        let mut a = SessionRenderEvent {
            event_type: 2,
            session_sequence: 2,
            session_generation: 7,
            device_lease_id: 31,
            device_generation: 41,
            rdev: files[0].metadata().unwrap().rdev(),
            boot_vga: 1,
            ..SessionRenderEvent::default()
        };
        text(&mut a.device_path, b"/dev/null");
        a.device_path[200] = 0xfd;
        let mut b = a;
        b.session_sequence = 3;
        b.device_lease_id = 32;
        b.device_generation = 42;
        b.rdev = files[1].metadata().unwrap().rdev();
        b.boot_vga = 0;
        text(&mut b.device_path, b"/dev/zero");
        let done = SessionRenderEvent {
            event_type: 3,
            session_sequence: 4,
            session_generation: 7,
            ..SessionRenderEvent::default()
        };
        Self {
            metadata: vec![init, a, b, done],
            files,
        }
    }
    fn borrowed(&self) -> Vec<BorrowedBootstrapEvent<'_>> {
        self.metadata
            .iter()
            .enumerate()
            .map(|(i, metadata)| BorrowedBootstrapEvent {
                metadata,
                primary: match i {
                    1 => Some(self.files[0].as_fd()),
                    2 => Some(self.files[1].as_fd()),
                    _ => None,
                },
                render: match i {
                    1 => Some(self.files[1].as_fd()),
                    2 => Some(self.files[0].as_fd()),
                    _ => None,
                },
            })
            .collect()
    }
    fn capture(&self) -> OwnedBootstrap {
        OwnedBootstrap::capture(7, &self.borrowed()).expect("valid actual FD pack")
    }
}
fn invalid(f: &Fixture) {
    assert!(matches!(
        OwnedBootstrap::capture(7, &f.borrowed()),
        Err(BootstrapError::Invalid)
    ));
    for p in &f.files {
        assert_open(p);
    }
}

#[test]
fn exact_aq408_value_layout_and_caps() {
    assert_eq!(
        (
            size_of::<SessionRenderEvent>(),
            align_of::<SessionRenderEvent>()
        ),
        (408, 8)
    );
    assert_eq!(
        (
            offset_of!(SessionRenderEvent, reserved),
            offset_of!(SessionRenderEvent, seat_name),
            offset_of!(SessionRenderEvent, device_path)
        ),
        (72, 88, 152)
    );
    assert_eq!(
        (
            offset_of!(SessionRenderEvent, session_sequence),
            offset_of!(SessionRenderEvent, active)
        ),
        (8, 48)
    );
    assert_eq!(
        (
            MAX_BOOTSTRAP_DEVICES,
            MAX_BOOTSTRAP_EVENTS,
            MAX_BOOTSTRAP_FDS
        ),
        (32, 34, 64)
    );
}
#[test]
fn captures_exact_values_without_retaining_metadata_or_original_handles() {
    let _guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let mut f = Fixture::new();
    let expected = f.metadata.clone();
    let pack = f.capture();
    assert_eq!(
        (
            pack.session_generation(),
            pack.device_count(),
            pack.fd_count()
        ),
        (7, 2, 4)
    );
    f.metadata.fill(SessionRenderEvent::default());
    assert_eq!(
        pack.events()
            .iter()
            .map(|e| *e.metadata())
            .collect::<Vec<_>>(),
        expected
    );
    for original in &f.files {
        assert_open(original);
    }
}
#[test]
fn actual_cloexec_duplicates_keep_original_flags_and_allow_distinct_render_rdev() {
    let _guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let f = Fixture::new();
    assert_ne!(f.metadata[1].rdev, f.metadata[2].rdev);
    assert_eq!(unsafe { fcntl(f.files[0].as_raw_fd(), 2, 0) }, 0);
    let before: Vec<_> = f.files.iter().map(|p| fd_flags(p.as_raw_fd())).collect();
    let pack = f.capture();
    for (i, event) in pack.events().iter().enumerate() {
        if i == 1 || i == 2 {
            let primary = event.primary().unwrap();
            let render = event.render().unwrap();
            assert!(
                fd_flags(primary.as_raw_fd()) & 1 != 0 && fd_flags(render.as_raw_fd()) & 1 != 0
            );
            assert!(!f.files.iter().any(
                |p| p.as_raw_fd() == primary.as_raw_fd() || p.as_raw_fd() == render.as_raw_fd()
            ));
            let p = File::from(primary.try_clone_to_owned().unwrap());
            let r = File::from(render.try_clone_to_owned().unwrap());
            assert_eq!(p.metadata().unwrap().rdev(), event.metadata().rdev);
            assert_ne!(r.metadata().unwrap().rdev(), event.metadata().rdev);
        }
    }
    assert_eq!(
        f.files
            .iter()
            .map(|p| fd_flags(p.as_raw_fd()))
            .collect::<Vec<_>>(),
        before
    );
}
#[test]
fn zero_devices_and_inactive_init_are_faithful_metadata_not_discovery() {
    let _guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let f = Fixture::new();
    let mut a = f.metadata[0];
    a.active = 0;
    let mut z = f.metadata[3];
    z.session_sequence = 2;
    let events = [
        BorrowedBootstrapEvent {
            metadata: &a,
            primary: None,
            render: None,
        },
        BorrowedBootstrapEvent {
            metadata: &z,
            primary: None,
            render: None,
        },
    ];
    let pack = OwnedBootstrap::capture(7, &events).unwrap();
    assert_eq!((pack.device_count(), pack.fd_count()), (0, 0));
    assert_eq!(pack.events()[0].metadata().active, 0);
}
#[test]
fn refuses_wrong_bootstrap_grammar_and_generation_before_consuming_originals() {
    let _guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let f = Fixture::new();
    assert!(matches!(
        OwnedBootstrap::capture(7, &[]),
        Err(BootstrapError::Invalid)
    ));
    assert!(matches!(
        OwnedBootstrap::capture(7, &f.borrowed()[..1]),
        Err(BootstrapError::Invalid)
    ));
    assert!(matches!(
        OwnedBootstrap::capture(0, &f.borrowed()),
        Err(BootstrapError::Invalid)
    ));
    for case in 0..7 {
        let mut f = Fixture::new();
        match case {
            0 => f.metadata[0].event_type = 2,
            1 => f.metadata[1].event_type = 4,
            2 => f.metadata[3].event_type = 2,
            3 => f.metadata[1].session_sequence = 3,
            4 => f.metadata[2].session_sequence = 0,
            5 => f.metadata[2].session_generation = 8,
            _ => f.metadata[0].session_generation = 0,
        };
        invalid(&f);
    }
}
#[test]
fn refuses_bad_reserved_boolean_version_and_nonterminated_or_empty_names() {
    let _guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    for case in 0..13 {
        let mut f = Fixture::new();
        match case {
            0 => f.metadata[1].version = 2,
            1 => f.metadata[1].active = 2,
            2 => f.metadata[1].boot_vga = 2,
            3 => f.metadata[1].reserved[0] = 1,
            4 => f.metadata[1].reserved[1] = 1,
            5 => f.metadata[1].reserved[2] = 1,
            6 => f.metadata[1].reserved[3] = 1,
            7 => f.metadata[0].seat_name[0] = 0,
            8 => f.metadata[2].device_path[0] = 0,
            9 => f.metadata[0].seat_name.fill(65),
            10 => f.metadata[3].device_path.fill(66),
            11 => f.metadata[1].device_lease_id = 0,
            _ => f.metadata[1].device_generation = 0,
        };
        invalid(&f);
    }
}
#[test]
fn refuses_duplicate_lease_or_primary_node_and_mismatched_primary_rdev() {
    let _guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let mut f = Fixture::new();
    f.metadata[2].device_lease_id = 31;
    invalid(&f);
    let mut f = Fixture::new();
    f.metadata[1].rdev += 1;
    invalid(&f);
    let mut f = Fixture::new();
    f.metadata[2].rdev = f.metadata[1].rdev;
    let mut events = f.borrowed();
    events[2].primary = Some(f.files[0].as_fd());
    assert!(matches!(
        OwnedBootstrap::capture(7, &events),
        Err(BootstrapError::Invalid)
    ));
}
#[test]
fn refuses_fd_shape_and_memfd_is_not_a_character_node() {
    let _guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let f = Fixture::new();
    let memory = memfd();
    for case in 0..4 {
        let mut events = f.borrowed();
        match case {
            0 => events[1].primary = None,
            1 => events[0].primary = Some(f.files[0].as_fd()),
            2 => events[3].render = Some(f.files[1].as_fd()),
            _ => events[1].primary = Some(memory.as_fd()),
        };
        assert!(matches!(
            OwnedBootstrap::capture(7, &events),
            Err(BootstrapError::Invalid)
        ));
    }
    assert_open(&memory);
    for original in &f.files {
        assert_open(original);
    }
}
#[test]
fn oversize_packet_refuses_before_duplication_and_preserves_originals() {
    let _guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let f = Fixture::new();
    let base = f.borrowed();
    let events = vec![base[0]; 35];
    let before = fd_set();
    assert!(matches!(
        OwnedBootstrap::capture(7, &events),
        Err(BootstrapError::Capacity)
    ));
    assert_eq!(fd_set(), before);
    for p in &f.files {
        assert_open(p);
    }
}
#[test]
fn real_late_memfd_refusal_cleans_all_earlier_and_current_duplicates() {
    let _guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let f = Fixture::new();
    let memory = memfd();
    let mut events = f.borrowed();
    events[2].render = Some(memory.as_fd());
    let before = fd_set();
    for _ in 0..8 {
        assert!(matches!(
            OwnedBootstrap::capture(7, &events),
            Err(BootstrapError::Invalid)
        ));
        assert_eq!(fd_set(), before);
    }
    assert_open(&memory);
    for original in &f.files {
        assert_open(original);
    }
}
#[test]
fn drop_closes_only_owned_duplicates_once_and_consuming_parts_transfer_exactly_once() {
    let _guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let f = Fixture::new();
    let pack = f.capture();
    let duplicates: Vec<_> = pack
        .events()
        .iter()
        .flat_map(|e| [e.primary(), e.render()])
        .flatten()
        .map(|fd| fd.as_raw_fd())
        .collect();
    drop(pack);
    for fd in duplicates {
        assert_closed(fd);
    }
    for original in &f.files {
        assert_open(original);
    }
    let mut owned = f.capture().into_events();
    let event = owned.remove(1);
    let (expected, primary, render) = event.into_parts();
    assert_eq!(expected, f.metadata[1]);
    drop(owned);
    let p = primary.unwrap();
    let r = render.unwrap();
    assert!(fd_flags(p.as_raw_fd()) >= 0 && fd_flags(r.as_raw_fd()) >= 0);
    let (rawp, rawr) = (p.as_raw_fd(), r.as_raw_fd());
    drop(p);
    drop(r);
    assert_closed(rawp);
    assert_closed(rawr);
    for original in &f.files {
        assert_open(original);
    }
}
#[test]
fn actual_joined_worker_receives_owned_pack_originals_remain_valid() {
    let _guard = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let f = Fixture::new();
    let pack = f.capture();
    let copies = f.metadata.clone();
    let thread = std::thread::spawn(move || {
        assert_eq!(
            pack.events()
                .iter()
                .map(|e| *e.metadata())
                .collect::<Vec<_>>(),
            copies
        );
        let count = pack.fd_count();
        for e in pack.events() {
            for fd in [e.primary(), e.render()].into_iter().flatten() {
                assert!(fd_flags(fd.as_raw_fd()) & 1 != 0);
            }
        }
        drop(pack);
        count
    });
    assert_eq!(thread.join().unwrap(), 4);
    for original in &f.files {
        assert_open(original);
    }
}
