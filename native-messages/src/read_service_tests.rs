//! Real seqpacket ancillary refusal; no daemon or phone qualification.
use super::receive;
use rustix::{fd::AsFd, net};
use std::{fs, io::IoSlice, mem::MaybeUninit, path::Path};

fn descriptors_for(path: &Path) -> usize {
    fs::read_dir("/proc/self/fd")
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| fs::read_link(entry.path()).ok())
        .filter(|target| target == path)
        .count()
}

#[test]
fn received_rights_are_rejected_and_closed() {
    let path = std::env::temp_dir().join(format!(
        "pokelinux-private-rights-{}",
        pokelinux_lifecycle::fresh_id().unwrap()
    ));
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let (sender, receiver) = net::socketpair(
        net::AddressFamily::UNIX,
        net::SocketType::SEQPACKET,
        net::SocketFlags::CLOEXEC | net::SocketFlags::NONBLOCK,
        None,
    )
    .unwrap();
    net::sockopt::set_socket_passcred(&receiver, true).unwrap();
    assert_eq!(descriptors_for(&path), 1);
    let handles = [file.as_fd()];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
    let mut ancillary = net::SendAncillaryBuffer::new(&mut space);
    assert!(ancillary.push(net::SendAncillaryMessage::ScmRights(&handles)));
    let body = b"{\"op\":\"open\",\"version\":1}";
    assert_eq!(
        net::sendmsg(
            &sender,
            &[IoSlice::new(body)],
            &mut ancillary,
            net::SendFlags::DONTWAIT | net::SendFlags::NOSIGNAL,
        )
        .unwrap(),
        body.len()
    );
    let error = receive(&receiver).unwrap_err();
    let remaining = descriptors_for(&path);
    drop(file);
    fs::remove_file(path).unwrap();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(remaining, 1, "received descriptor must close on refusal");
}
