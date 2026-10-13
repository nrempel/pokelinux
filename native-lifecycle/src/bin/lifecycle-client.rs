//! A bounded command client. Driver mode is isolated qualification infrastructure.
use pokelinux_lifecycle::{Reply, boottime_ns, ipc};
use rustix::fd::AsFd;
use rustix::net::{self, SocketFlags};
use std::{io, path::Path, time::Duration};
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let listener = ipc::listener(Path::new(&args[1]), 0o600)?;
    let mut second = None;
    let mut service = ipc::connect(Path::new(&args[0]))?;
    let deadline = boottime_ns().saturating_add(30_000_000_000);
    loop {
        if boottime_ns() >= deadline {
            return Err(io::ErrorKind::TimedOut.into());
        }
        match net::accept_with(&listener, SocketFlags::CLOEXEC | SocketFlags::NONBLOCK) {
            Ok(driver) => loop {
                if boottime_ns() >= deadline {
                    return Err(io::ErrorKind::TimedOut.into());
                }
                match ipc::receive(&driver) {
                    Ok(Some((data, cred))) => {
                        if cred.uid.as_raw() != 0 {
                            break;
                        }
                        let value: serde_json::Value = serde_json::from_slice(&data)?;
                        if matches!(
                            value.get("driver").and_then(serde_json::Value::as_str),
                            Some("send" | "receive")
                        ) {
                            let channel = value
                                .get("channel")
                                .and_then(serde_json::Value::as_u64)
                                .ok_or(io::ErrorKind::InvalidInput)?;
                            if channel > 1 {
                                return Err(io::ErrorKind::InvalidInput.into());
                            }
                            if channel == 1 && second.is_none() {
                                second = Some(ipc::connect(Path::new(&args[0]))?);
                            }
                            let fd = if channel == 0 {
                                &service
                            } else {
                                second.as_ref().unwrap()
                            };
                            let reply = if value["driver"] == "send" {
                                let bytes = serde_json::to_vec(
                                    value.get("request").ok_or(io::ErrorKind::InvalidInput)?,
                                )?;
                                ipc::send(fd, &bytes)?;
                                serde_json::json!({"driver_result":"sent"})
                            } else {
                                match ipc::receive(fd)? {
                                    None => serde_json::json!({"driver_result":"pending"}),
                                    Some((wire, cred)) => {
                                        if cred.uid.as_raw() != 0 {
                                            return Err(io::ErrorKind::PermissionDenied.into());
                                        }
                                        serde_json::from_slice::<serde_json::Value>(&wire)?
                                    }
                                }
                            };
                            ipc::send(&driver, &serde_json::to_vec(&reply)?)?;
                            continue;
                        }
                        if value.get("driver") == Some(&serde_json::json!("exit")) {
                            return Ok(());
                        }
                        if value.get("driver") == Some(&serde_json::json!("export_socket")) {
                            let fds = [service.as_fd()];
                            let mut space = [std::mem::MaybeUninit::uninit();
                                rustix::cmsg_space!(ScmRights(1))];
                            let mut cmsg = net::SendAncillaryBuffer::new(&mut space);
                            if !cmsg.push(net::SendAncillaryMessage::ScmRights(&fds)) {
                                return Err(io::ErrorKind::InvalidData.into());
                            }
                            net::sendmsg(
                                &driver,
                                &[std::io::IoSlice::new(b"socket")],
                                &mut cmsg,
                                net::SendFlags::NOSIGNAL | net::SendFlags::DONTWAIT,
                            )?;
                            continue;
                        }
                        if value.get("driver") == Some(&serde_json::json!("reconnect")) {
                            service = ipc::connect(Path::new(&args[0]))?;
                            ipc::send(&driver, &serde_json::to_vec(&Reply::Ok)?)?;
                            continue;
                        }
                        let reply: Reply = ipc::call(&service, &value)?;
                        ipc::send(&driver, &serde_json::to_vec(&reply)?)?;
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(2)),
                    Err(_) => break,
                }
            },
            Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {
                std::thread::sleep(Duration::from_millis(2))
            }
            Err(e) => return Err(e.into()),
        }
    }
}
