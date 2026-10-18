struct Peer {
    child: Child,
    out: Vec<u8>,
    err: Vec<u8>,
    out_eof: bool,
    err_eof: bool,
    exit: Option<i32>,
    wait_at: Option<u64>,
    until: u64,
}
impl Peer {
    fn spawn(case: &str, path: &Path) -> Self {
        let child = Command::new(std::env::current_exe().unwrap())
            .arg("client")
            .arg(case)
            .arg(path)
            .uid(11050)
            .gid(11050)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        for fd in [
            child.stdout.as_ref().unwrap() as &dyn std::os::fd::AsFd,
            child.stderr.as_ref().unwrap() as &dyn std::os::fd::AsFd,
        ] {
            let flags = rustix::fs::fcntl_getfl(fd).unwrap();
            rustix::fs::fcntl_setfl(fd, flags | rustix::fs::OFlags::NONBLOCK).unwrap();
        }
        Self {
            child,
            out: vec![],
            err: vec![],
            out_eof: false,
            err_eof: false,
            exit: None,
            wait_at: None,
            until: now() + 5_000_000_000,
        }
    }
    fn pump(&mut self) {
        fn read(stream: &mut impl Read, b: &mut Vec<u8>, eof: &mut bool) {
            if *eof {
                return;
            }
            let mut bytes = [0; 1024];
            loop {
                match stream.read(&mut bytes) {
                    Ok(0) => {
                        *eof = true;
                        break;
                    }
                    Ok(n) => {
                        b.extend_from_slice(&bytes[..n]);
                        assert!(b.len() < 32768)
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(e) => panic!("child PIPE {e}"),
                }
            }
        }
        read(
            self.child.stdout.as_mut().unwrap(),
            &mut self.out,
            &mut self.out_eof,
        );
        read(
            self.child.stderr.as_mut().unwrap(),
            &mut self.err,
            &mut self.err_eof,
        );
        if self.exit.is_none() {
            if let Some(s) = self.child.try_wait().unwrap() {
                self.exit = Some(s.code().unwrap_or(-1));
                self.wait_at = Some(now());
            }
        }
    }
    fn finish(&mut self) -> bool {
        loop {
            self.pump();
            if self.exit.is_some() && self.out_eof && self.err_eof {
                break;
            }
            assert!(now() < self.until, "original child wait/EOF cutoff");
            pause();
        }
        println!(
            "CHILD_RECEIPT {}",
            json!({"pid":self.child.id(),"exit":self.exit,"actual_wait_ns":self.wait_at,
            "stdout_eof":self.out_eof,"stderr_eof":self.err_eof,"original_until_ns":self.until,"closed_ns":now()})
        );
        for line in String::from_utf8_lossy(&self.out).lines() {
            if line.starts_with("SDK_JOIN ") || line.starts_with("CHILD_CAPS ") {
                println!("{line}");
            }
        }
        if self.exit != Some(0) {
            eprintln!("{}", String::from_utf8_lossy(&self.err));
        }
        self.exit == Some(0)
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        if self.exit.is_none() {
            let _ = self.child.kill();
        }
        while !(self.exit.is_some() && self.out_eof && self.err_eof) && now() < self.until {
            self.pump();
            pause();
        }
        if !(self.exit.is_some() && self.out_eof && self.err_eof) {
            std::process::abort();
        }
    }
}
struct Socket {
    root: PathBuf,
    path: PathBuf,
    listener: OwnedFd,
    current: std::cell::RefCell<Option<OwnedFd>>,
    generation: std::cell::Cell<u64>,
    first: std::cell::Cell<bool>,
}
impl Socket {
    fn new(id: usize) -> Self {
        let root = PathBuf::from(format!(
            "/tmp/settings-boundary-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        let path = root.join("app.sock");
        let listener = net::socket_with(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();
        net::sockopt::set_socket_passcred(&listener, true).unwrap();
        net::bind(&listener, &SocketAddrUnix::new(&path).unwrap()).unwrap();
        net::listen(&listener, 8).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
        Self {
            root,
            path,
            listener,
            current: std::cell::RefCell::new(None),
            generation: std::cell::Cell::new(0),
            first: std::cell::Cell::new(true),
        }
    }
    fn request(&self, p: &mut Peer) -> Option<(OwnedFd, Value)> {
        let mut resets = 0;
        loop {
            p.pump();
            if p.exit.is_some() {
                return None;
            }
            assert!(now() < p.until);
            let needs_accept = self.current.borrow().is_none();
            if needs_accept {
                let fd = match net::accept_with(
                    &self.listener,
                    SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
                ) {
                    Ok(fd) => fd,
                    Err(rustix::io::Errno::AGAIN) => {
                        pause();
                        continue;
                    }
                    Err(e) => panic!("accept {e}"),
                };
                let c = net::sockopt::socket_peercred(&fd).unwrap();
                assert_eq!(c.pid.as_raw_nonzero().get() as u32, p.child.id());
                assert_eq!(c.uid.as_raw(), 11050);
                assert_eq!(c.gid.as_raw(), 11050);
                net::sockopt::set_socket_passcred(&fd, true).unwrap();
                *self.current.borrow_mut() = Some(fd);
                self.generation.set(self.generation.get() + 1);
                self.first.set(true);
            }
            let current = self.current.borrow();
            let fd = current.as_ref().unwrap();
            let peer = net::sockopt::socket_peercred(fd).unwrap();
            let mut b = [0; 4096];
            let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1))];
            let mut c = RecvAncillaryBuffer::new(&mut space);
            let m = match net::recvmsg(
                fd,
                &mut [IoSliceMut::new(&mut b)],
                &mut c,
                RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC,
            ) {
                Ok(m) => m,
                Err(rustix::io::Errno::CONNRESET) => {
                    resets += 1;
                    println!(
                        "SOCKET_RESET {}",
                        json!({"phase":"request","pid":p.child.id(),"generation":self.generation(),"count":resets})
                    );
                    drop(current);
                    continue;
                }
                Err(rustix::io::Errno::AGAIN) => {
                    drop(current);
                    pause();
                    continue;
                }
                Err(e) => panic!("request {e}"),
            };
            if m.bytes == 0 {
                println!(
                    "SOCKET_EOF {}",
                    json!({"phase":"request","pid":p.child.id(),"generation":self.generation(),"resets":resets,"zero_byte_EOF":1})
                );
                drop(current);
                self.current.borrow_mut().take();
                continue;
            }
            assert!(!m.flags.intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC));
            let entries: Vec<_> = c.drain().collect();
            assert_eq!(entries.len(), 1);
            match entries[0] {
                RecvAncillaryMessage::ScmCredentials(c) => assert_eq!(c, peer),
                _ => panic!("request credentials"),
            }
            let v: Value = serde_json::from_slice(&b[..m.bytes]).unwrap();
            if self.first.replace(false) {
                assert_eq!(
                    v,
                    json!({"op":"connectivity_status"}),
                    "reconnect is readonly only"
                );
            }
            return Some((fd.try_clone().unwrap(), v));
        }
    }
    fn generation(&self) -> u64 {
        self.generation.get()
    }
}
impl Drop for Socket {
    fn drop(&mut self) {
        self.current.get_mut().take();
        fs::remove_file(&self.path).unwrap();
        fs::remove_dir(&self.root).unwrap();
    }
}
