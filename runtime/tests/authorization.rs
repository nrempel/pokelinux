#![cfg(target_os = "linux")]
use omarchy_phone_core::{Identity, authorization::Session};
use std::{io::Write, os::unix::net::UnixStream, time::Duration};
fn send(client: &mut UnixStream, session: &Identity) {
    let request = serde_json::json!({"v":1,"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","epoch":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","session":session,"command":{"kind":"dial","generation":"dddddddddddddddddddddddddddddddd","offer":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","number":"5550100"}});
    writeln!(client, "{request}").unwrap();
}
#[test]
fn phone_and_shell_share_a_session_without_replacing_each_other() {
    let session = Session::default();
    let (mut phone, mut phone_peer) = UnixStream::pair().unwrap();
    let (mut shell, mut shell_peer) = UnixStream::pair().unwrap();
    let phone_identity = session.identity_for(&phone_peer).unwrap();
    let shell_identity = session.identity_for(&shell_peer).unwrap();
    assert_eq!(phone_identity, shell_identity);
    send(&mut phone, &phone_identity);
    send(&mut shell, &shell_identity);
    assert_eq!(
        session
            .receive(&mut phone_peer, Duration::from_secs(1))
            .unwrap()
            .session(),
        &phone_identity
    );
    assert_eq!(
        session
            .receive(&mut shell_peer, Duration::from_secs(1))
            .unwrap()
            .session(),
        &shell_identity
    );
}
#[test]
fn restart_rejects_stale_session_on_a_new_connection() {
    let old = Session::default();
    let (_, peer) = UnixStream::pair().unwrap();
    let old_identity = old.identity_for(&peer).unwrap();
    let restarted = Session::default();
    let (mut client, mut peer) = UnixStream::pair().unwrap();
    assert_ne!(old_identity, restarted.identity_for(&peer).unwrap());
    send(&mut client, &old_identity);
    assert_eq!(
        restarted
            .receive(&mut peer, Duration::from_secs(1))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::PermissionDenied
    );
}

// Run only in the isolated credential-test container with SETUID/SETGID.
#[test]
#[ignore = "requires isolated root container and a second UID"]
fn foreign_uid_is_rejected_before_reading_or_revealing_session() {
    use std::os::unix::{fs::PermissionsExt, net::UnixListener, process::CommandExt};
    assert_eq!(rustix::process::geteuid().as_raw(), 0);
    let dir = std::env::temp_dir().join(format!("phone-peer-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = dir.join("socket");
    let listener = UnixListener::bind(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "foreign_uid_client_helper", "--ignored"])
        .env("PHONE_TEST_PEER_SOCKET", &path)
        .uid(65534)
        .gid(65534)
        .spawn()
        .unwrap();
    let (mut peer, _) = listener.accept().unwrap();
    let session = Session::default();
    assert_eq!(
        session.identity_for(&peer).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        session
            .receive(&mut peer, Duration::ZERO)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::PermissionDenied
    );
    drop(peer);
    assert!(child.wait().unwrap().success());
    drop(listener);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
#[ignore = "child process of the isolated credential test"]
fn foreign_uid_client_helper() {
    use std::io::Read;
    assert_eq!(rustix::process::geteuid().as_raw(), 65534);
    let path = std::env::var_os("PHONE_TEST_PEER_SOCKET").unwrap();
    let mut socket = UnixStream::connect(path).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    assert_eq!(socket.read(&mut [0; 1]).unwrap(), 0);
}
