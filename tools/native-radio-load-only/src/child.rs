//! Public harmless controls only. Never names, links or loads vendor inputs.
#![no_main]
use std::{fs, io::Write, os::unix::net::UnixStream, time::Duration};

// The containment contract deliberately closes FD0 and exposes no /dev/null.
// Enter through the public C ABI without Rust's standard executable startup;
// the controls use explicit argv and do not request stdin sanitization.
#[unsafe(no_mangle)]
/// # Safety
/// The pinned public C runtime supplies `argc` valid nul-terminated argv
/// pointers. This entry is called only by that executable's startup code.
pub unsafe extern "C" fn main(argc: libc::c_int, argv: *const *const libc::c_char) -> libc::c_int {
    let args: Vec<String> = (0..argc as usize)
        .map(|index| {
            unsafe { std::ffi::CStr::from_ptr(*argv.add(index)) }
                .to_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    run(&args);
    0
}
fn run(args: &[String]) {
    if args[1] != "outside-unix" {
        sandbox(args);
        return;
    }
    assert_eq!(args[1], "outside-unix");
    let outside = fs::read_to_string(&args[2]);
    let mut connection = UnixStream::connect(&args[3]);
    if let Ok(ref mut stream) = connection {
        stream.write_all(b"public-harmless-probe\n").unwrap();
    }
    println!(
        "{}",
        serde_json::json!({"control":"outside-unix", "outside_read":outside.as_deref().ok()==Some("public-outside-root-marker\n"), "unix_connect":connection.is_ok(), "pid":std::process::id(), "uid":unsafe {libc::geteuid()}, "owner":std::env::var("POKELINUX_OWNER_INCARNATION").ok()})
    );
    std::io::stdout().flush().unwrap();
    loop {
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn checkpoint(nonce: &str, event: &str, fields: serde_json::Value) {
    let mut value = fields.as_object().cloned().unwrap();
    value.insert("nonce".into(), nonce.into());
    value.insert("event".into(), event.into());
    let packet = serde_json::to_vec(&value).unwrap();
    assert!(packet.len() <= 256);
    assert_eq!(
        unsafe { libc::write(3, packet.as_ptr().cast(), packet.len()) },
        packet.len() as isize
    );
}
fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap()
}
fn sandbox(args: &[String]) {
    use std::ffi::CString;
    let c = |s: &str| CString::new(s).unwrap();
    let nonce = &args[2];
    match args[1].as_str() {
        "held" | "queued-late" => {
            checkpoint(
                nonce,
                "ready",
                serde_json::json!({"uid":unsafe{libc::geteuid()},"gid":unsafe{libc::getegid()},"inner_pid":std::process::id()}),
            );
        }
        "probes" => {
            let outside =
                unsafe { libc::open(c("/run/public-outside-marker").as_ptr(), libc::O_RDONLY) };
            let outside_errno = errno();
            if outside >= 0 {
                unsafe { libc::close(outside) };
            }
            checkpoint(
                nonce,
                "outside-root",
                serde_json::json!({"denied":outside<0,"errno":outside_errno}),
            );
            let ip = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
            let ip_errno = errno();
            if ip >= 0 {
                unsafe { libc::close(ip) };
            }
            checkpoint(
                nonce,
                "ip-socket",
                serde_json::json!({"denied":ip<0,"errno":ip_errno}),
            );
            let unix = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
            assert!(unix >= 0);
            let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
            addr.sun_family = libc::AF_UNIX as u16;
            for (i, b) in b"/run/public-outside.sock".iter().enumerate() {
                addr.sun_path[i] = *b as libc::c_char;
            }
            let connected = unsafe {
                libc::connect(
                    unix,
                    (&addr as *const libc::sockaddr_un).cast(),
                    std::mem::size_of_val(&addr) as u32,
                )
            };
            let unix_errno = errno();
            unsafe { libc::close(unix) };
            checkpoint(
                nonce,
                "unix-connect",
                serde_json::json!({"socket_created":true,"denied":connected<0,"errno":unix_errno}),
            );
            for path in ["/dev/binder", "/dev/null", "/sys", "/workspace", "/results"] {
                let open = unsafe { libc::open(c(path).as_ptr(), libc::O_RDONLY) };
                let e = errno();
                if open >= 0 {
                    unsafe { libc::close(open) };
                }
                checkpoint(
                    nonce,
                    "forbidden-path",
                    serde_json::json!({"path":path,"denied":open<0,"errno":e}),
                );
            }
            let mount = unsafe {
                libc::mount(
                    c("none").as_ptr(),
                    c("/tmp").as_ptr(),
                    c("tmpfs").as_ptr(),
                    0,
                    std::ptr::null(),
                )
            };
            let mount_errno = errno();
            let ns = unsafe { libc::unshare(libc::CLONE_NEWNET) };
            let ns_errno = errno();
            checkpoint(
                nonce,
                "namespace",
                serde_json::json!({"mount_denied":mount<0,"mount_errno":mount_errno,"unshare_denied":ns<0,"unshare_errno":ns_errno}),
            );
            let write =
                unsafe { libc::open(c("/fixture").as_ptr(), libc::O_WRONLY | libc::O_TRUNC) };
            let write_errno = errno();
            if write >= 0 {
                unsafe { libc::close(write) };
            }
            checkpoint(
                nonce,
                "filesystem-write",
                serde_json::json!({"denied":write<0,"errno":write_errno}),
            );
            let memory = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    536870912,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                )
            };
            let memory_errno = errno();
            let failed = memory == libc::MAP_FAILED;
            if !failed {
                unsafe { libc::munmap(memory, 536870912) };
            }
            checkpoint(
                nonce,
                "memory-limit",
                serde_json::json!({"denied":failed,"errno":memory_errno,"requested_bytes":536870912}),
            );
            let mut fds = Vec::new();
            let exhausted = loop {
                let f = unsafe { libc::open(c("/fixture").as_ptr(), libc::O_RDONLY) };
                if f < 0 {
                    break errno();
                }
                fds.push(f);
                if fds.len() > 64 {
                    break 0;
                }
            };
            let count = fds.len();
            for f in fds {
                unsafe { libc::close(f) };
            }
            checkpoint(
                nonce,
                "fd-limit",
                serde_json::json!({"opened":count,"errno":exhausted,"denied":exhausted==libc::EMFILE}),
            );
        }
        "second-exec" => {
            checkpoint(nonce, "ready", serde_json::json!({"second_exec_next":true}));
            let exec = c("/bin/radio-harmless-child");
            let mode = c("held");
            let until = c(&args[3]);
            let nonce = c(nonce);
            let argv = [
                exec.as_ptr(),
                mode.as_ptr(),
                nonce.as_ptr(),
                until.as_ptr(),
                std::ptr::null(),
            ];
            unsafe { libc::execve(exec.as_ptr(), argv.as_ptr(), [std::ptr::null()].as_ptr()) };
            checkpoint(
                &args[2],
                "second-exec-returned",
                serde_json::json!({"errno":errno()}),
            );
        }
        "task-limit" => {
            let mut threads = Vec::new();
            let error = loop {
                match std::thread::Builder::new().spawn(|| {
                    loop {
                        std::thread::sleep(Duration::from_millis(25));
                    }
                }) {
                    Ok(thread) => threads.push(thread),
                    Err(e) => break e.raw_os_error().unwrap_or(0),
                }
                if threads.len() > 32 {
                    break 0;
                }
            };
            checkpoint(
                nonce,
                "task-limit",
                serde_json::json!({"threads":threads.len(),"errno":error,"denied":error==libc::EAGAIN}),
            );
            loop {
                std::thread::sleep(Duration::from_millis(25));
            }
        }
        _ => panic!("unknown public control"),
    }
    loop {
        std::thread::sleep(Duration::from_millis(25));
    }
}
