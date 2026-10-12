#![cfg(target_os = "linux")]
use omarchy_phone_core::{
    local_client::{ExchangeError, exchange},
    local_reply::Body,
};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};
const REQUEST: &[u8] = b"{\"v\":1,\"id\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"op\":\"hello\"}\n";
struct Socket(PathBuf, UnixListener);
impl Socket {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("phone-client-{}", uuid::Uuid::new_v4().simple()));
        let listener = UnixListener::bind(&path).unwrap();
        Self(path, listener)
    }
}
impl Drop for Socket {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
#[test]
fn validates_reply_from_actual_unix_peer() {
    let f = Socket::new();
    thread::scope(|scope| {
        scope.spawn(|| { let (mut socket,_)=f.1.accept().unwrap(); let mut line=String::new(); BufReader::new(socket.try_clone().unwrap()).read_line(&mut line).unwrap(); assert_eq!(line.as_bytes(),REQUEST); socket.write_all(b"{\"v\":1,\"id\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"result\":{\"kind\":\"hello\",\"epoch\":\"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\",\"session\":\"cccccccccccccccccccccccccccccccc\"}}\n").unwrap(); });
        assert!(matches!(
            exchange(&f.0, REQUEST, Duration::from_secs(1))
                .unwrap()
                .body(),
            Body::Hello { .. }
        ));
    });
}
#[test]
fn invalid_request_is_not_sent() {
    let f = Socket::new();
    f.1.set_nonblocking(true).unwrap();
    assert!(matches!(
        exchange(&f.0, b"{}\n", Duration::from_secs(1)),
        Err(ExchangeError::NotSent(_))
    ));
    assert_eq!(
        f.1.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
#[test]
fn lost_reply_is_unknown_and_never_reconnected() {
    let f = Socket::new();
    thread::scope(|scope| {
        scope.spawn(|| {
            let (socket, _) = f.1.accept().unwrap();
            let mut line = String::new();
            BufReader::new(socket).read_line(&mut line).unwrap();
            assert_eq!(line.as_bytes(), REQUEST);
        });
        assert!(matches!(
            exchange(&f.0, REQUEST, Duration::from_secs(1)),
            Err(ExchangeError::Unknown(_))
        ));
    });
    f.1.set_nonblocking(true).unwrap();
    assert_eq!(
        f.1.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
#[test]
fn stalled_reply_has_bounded_deadline() {
    let f = Socket::new();
    thread::scope(|scope| {
        scope.spawn(|| {
            let (socket, _) = f.1.accept().unwrap();
            thread::sleep(Duration::from_millis(250));
            drop(socket);
        });
        let start = Instant::now();
        assert!(matches!(
            exchange(&f.0, REQUEST, Duration::from_millis(30)),
            Err(ExchangeError::Unknown(_))
        ));
        assert!(start.elapsed() < Duration::from_millis(200));
    });
}
