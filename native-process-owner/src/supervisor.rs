//! Local process reclamation never establishes remote cleanup.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::PathBuf,
    process::{Child, ChildStdout, Command, ExitStatus, Stdio},
};

trait ProcessControl {
    fn kill(&mut self) -> io::Result<()>;
    fn pid(&self) -> Option<u32> {
        None
    }
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>>;
    fn read_output(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::ErrorKind::Unsupported.into())
    }
    fn configure_output(&self) -> io::Result<()> {
        Ok(())
    }
}
struct NativeChild {
    child: Child,
    output: ChildStdout,
    reaped: bool,
}
impl ProcessControl for NativeChild {
    fn pid(&self) -> Option<u32> {
        Some(self.child.id())
    }
    fn kill(&mut self) -> io::Result<()> {
        self.child.kill()
    }
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        let result = self.child.try_wait()?;
        self.reaped = result.is_some();
        Ok(result)
    }
    fn read_output(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.output.read(buffer)
    }
    fn configure_output(&self) -> io::Result<()> {
        use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
        let flags = fcntl_getfl(&self.output)?;
        fcntl_setfl(&self.output, flags | OFlags::NONBLOCK)?;
        Ok(())
    }
}
impl Drop for NativeChild {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            // Explicit fail-closed misuse. Never detach ownership or wait in Drop.
            // A deployed supervisor must itself be managed by a service manager.
            std::process::abort();
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessIdentity([u8; 16]);
impl ProcessIdentity {
    /// Codec data only. Constructing an identity does not transfer custody.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    fn new() -> io::Result<Self> {
        let mut bytes = [0; 16];
        File::open("/dev/urandom")?.read_exact(&mut bytes)?;
        Ok(Self(bytes))
    }
    pub fn hex(self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }
    pub fn from_hex(value: &str) -> Option<Self> {
        if value.len() != 32 || !value.is_ascii() {
            return None;
        }
        let mut bytes = [0; 16];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
        }
        Some(Self(bytes))
    }
}
/// Stored successful sync completion, captured before the spawn attempt.
/// A diagnostic fact only: neither power-loss proof nor remote cleanup authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionReceipt {
    pub identity: ProcessIdentity,
    pub sync_completed_before_spawn_at_boottime_ns: u64,
    pub journal_device: u64,
    pub journal_inode: u64,
    pub journal_uid: u32,
    pub journal_mode: u32,
}
#[derive(Debug, PartialEq, Eq)]
pub enum StartError {
    Occupied,
    AdmissionUncertain,
    Io,
}
/// Owns a single potentially registering process. The durable admission marker
/// is synced before spawn and never cleared here, including on spawn failure or
/// successful reap. A qualified remote cleanup/reset contract is still required.
/// Output is read without a reader thread; publication must call admits() with
/// both this connection's source identity and the frame's identity/attempt.
pub struct Supervisor {
    bootstrap: Option<crate::bootstrap::ParentBootstrap>,
    _same_thread: std::marker::PhantomData<std::rc::Rc<()>>,
    child: Option<Box<dyn ProcessControl>>,
    live: Option<ProcessIdentity>,
    admitted: bool,
    journal: PathBuf,
    admission_receipt: Option<AdmissionReceipt>,
}
impl Supervisor {
    /// The admission path must reside in an existing private directory owned by
    /// this service UID. Service installation and journal reset are not supplied.
    pub fn new(journal: PathBuf) -> Self {
        Self {
            bootstrap: None,
            _same_thread: std::marker::PhantomData,
            child: None,
            live: None,
            admitted: false,
            journal,
            admission_receipt: None,
        }
    }
    /// Receipt from this owner's successful file and directory sync only.
    /// A recreated owner never fabricates a receipt for a pre-existing marker.
    pub fn admission_receipt(&self) -> Option<AdmissionReceipt> {
        self.admission_receipt
    }
    pub fn occupied(&self) -> bool {
        self.child.is_some()
    }
    /// Diagnostic PID of the retained child. This is not a process capability
    /// and must not be used to transfer ownership or authorize an operation.
    pub fn child_pid(&self) -> Option<u32> {
        self.child.as_ref().and_then(|child| child.pid())
    }
    fn record_admission(&mut self, id: ProcessIdentity) -> Result<(), StartError> {
        let directory = self.journal.parent().ok_or(StartError::Io)?;
        let metadata = fs::symlink_metadata(directory).map_err(|_| StartError::Io)?;
        if !metadata.is_dir()
            || metadata.permissions().mode() & 0o077 != 0
            || metadata.uid() != rustix::process::geteuid().as_raw()
        {
            return Err(StartError::Io);
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&self.journal)
            .map_err(|error| {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    StartError::AdmissionUncertain
                } else {
                    StartError::Io
                }
            })?;
        self.admitted = true;
        writeln!(file, "admission-v1\n{}\n1", id.hex()).map_err(|_| StartError::Io)?;
        file.sync_all().map_err(|_| StartError::Io)?;
        File::open(directory)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| StartError::Io)?;
        let metadata = file.metadata().map_err(|_| StartError::Io)?;
        use crate::deadline::{BootClock, Clock};
        let sync_completed_before_spawn_at_boottime_ns = BootClock
            .now()
            .as_nanos()
            .try_into()
            .map_err(|_| StartError::Io)?;
        self.admission_receipt = Some(AdmissionReceipt {
            identity: id,
            sync_completed_before_spawn_at_boottime_ns,
            journal_device: metadata.dev(),
            journal_inode: metadata.ino(),
            journal_uid: metadata.uid(),
            journal_mode: metadata.mode(),
        });
        Ok(())
    }
    /// Low-level explicit-Stop process ownership only. Arbitrary commands have
    /// no parent-death/bootstrap guarantee. Cooperative workers use start_worker.
    pub fn start(&mut self, command: &mut Command) -> Result<ProcessIdentity, StartError> {
        if self.child.is_some() {
            return Err(StartError::Occupied);
        }
        if self.admitted {
            return Err(StartError::AdmissionUncertain);
        }
        let id = ProcessIdentity::new().map_err(|_| StartError::Io)?;
        self.record_admission(id)?;
        let mut child = command
            .env("POKELINUX_OWNER_INCARNATION", id.hex())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|_| StartError::Io)?;
        let output = child.stdout.take().expect("piped child stdout");
        self.child = Some(Box::new(NativeChild {
            child,
            output,
            reaped: false,
        }));
        self.live = Some(id);
        if self
            .child
            .as_ref()
            .expect("owned process")
            .configure_output()
            .is_err()
        {
            let _ = self.stop();
            return Err(StartError::Io);
        }
        Ok(id)
    }
    /// Launch only a selected cooperative worker. The main spawning thread must
    /// retain this non-Send/non-Sync owner through actual wait. Final credentials
    /// are inherited; service-manager/procfs/executable provisioning remain required.
    /// Returns Initializing, with no publication admission until poll_worker is Ready.
    /// Post-spawn errors revoke and kill but retain occupancy until poll_reap.
    pub fn start_worker(
        &mut self,
        worker: &mut crate::bootstrap::WorkerCommand,
    ) -> Result<ProcessIdentity, StartError> {
        if self.occupied() {
            return Err(StartError::Occupied);
        }
        if self.admitted {
            return Err(StartError::AdmissionUncertain);
        }
        let pending = crate::bootstrap::Prepared::new()
            .and_then(|p| p.install(worker))
            .map_err(|_| StartError::Io)?;
        let launched = self.start(&mut worker.command);
        worker.command.stdin(Stdio::null());
        let identity = launched?;
        self.live = None;
        let pid = self
            .child
            .as_ref()
            .and_then(|child| child.pid())
            .expect("owned native child PID");
        match pending.bind(pid, identity) {
            Ok(bootstrap) => {
                self.bootstrap = Some(bootstrap);
                Ok(identity)
            }
            Err(_) => {
                let _ = self.stop();
                Err(StartError::Io)
            }
        }
    }
    /// One nonblocking bounded packet per poll. Setup timeout counts suspend and
    /// never renews. Error still requires caller-owned Stop/poll_reap disposal.
    pub fn poll_worker(&mut self) -> io::Result<crate::bootstrap::WorkerState> {
        use crate::bootstrap::WorkerState;
        let Some(bootstrap) = self.bootstrap.as_mut() else {
            return Ok(WorkerState::Terminal);
        };
        match bootstrap.poll() {
            Ok(WorkerState::Ready) => {
                self.live = Some(bootstrap.identity);
                Ok(WorkerState::Ready)
            }
            Ok(state) => Ok(state),
            Err(error) => {
                let _ = self.stop();
                Err(error)
            }
        }
    }
    /// This comparison is only provenance fencing. It does not authenticate an
    /// app or grant permission, decode vendor data or establish call state.
    pub fn admits(&self, source: ProcessIdentity) -> bool {
        self.live == Some(source)
    }
    pub fn admits_frame(
        &self,
        source: ProcessIdentity,
        owner: ProcessIdentity,
        attempt: u64,
    ) -> bool {
        source == owner && attempt == 1 && self.admits(source)
    }
    /// Copy one opaque bounded message to the exact current Ready worker.
    /// No application permission, durable dispatch or remote delivery is implied.
    /// Use the original absolute operation/session deadline. A post-I/O error
    /// can follow delivery and must never be treated as safe retry authority.
    pub fn send_message(
        &mut self,
        source: ProcessIdentity,
        until: crate::deadline::Deadline,
        bytes: &[u8],
    ) -> io::Result<()> {
        if !self.admits(source) {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let result = self
            .bootstrap
            .as_mut()
            .ok_or(io::ErrorKind::PermissionDenied)?
            .send_message(until, bytes);
        if !self.admits(source) {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        result
    }
    /// Receive one owned post-GO record from the exact current Ready worker.
    /// None means actual EAGAIN. Stop/reap revoke admission before any I/O.
    /// This may discard bytes on error and never clears the admission marker.
    pub fn receive_message(
        &mut self,
        source: ProcessIdentity,
        until: crate::deadline::Deadline,
    ) -> io::Result<Option<Vec<u8>>> {
        if !self.admits(source) {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let result = self
            .bootstrap
            .as_mut()
            .ok_or(io::ErrorKind::PermissionDenied)?
            .receive_message(until);
        if !self.admits(source) {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        result
    }
    /// Read at most one caller-bounded buffer. WouldBlock is normal. No detached
    /// reader thread or deferred publication is created; stale output is discardable.
    pub fn read_output(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.child
            .as_mut()
            .ok_or(io::ErrorKind::NotConnected)?
            .read_output(buffer)
    }
    pub fn stop(&mut self) -> io::Result<()> {
        // Revocation precedes signal delivery, including kill failure.
        self.live = None;
        self.bootstrap = None;
        if let Some(child) = &mut self.child {
            child.kill()?;
        }
        Ok(())
    }
    pub fn poll_reap(&mut self) -> io::Result<Option<ExitStatus>> {
        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                self.live = None;
                self.bootstrap = None;
                self.child = None;
                Ok(Some(status))
            }
            Ok(None) => Ok(None),
            Err(error) => {
                self.live = None;
                self.bootstrap = None;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    struct HeldReap(Arc<AtomicBool>);
    impl ProcessControl for HeldReap {
        fn kill(&mut self) -> io::Result<()> {
            Ok(())
        }
        fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            use std::os::unix::process::ExitStatusExt;
            Ok(self
                .0
                .load(Ordering::SeqCst)
                .then(|| ExitStatus::from_raw(9)))
        }
    }
    struct FailedControl;
    impl ProcessControl for FailedControl {
        fn kill(&mut self) -> io::Result<()> {
            Err(io::ErrorKind::PermissionDenied.into())
        }
        fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            Err(io::ErrorKind::Interrupted.into())
        }
    }
    #[test]
    fn kill_and_reap_errors_revoke_output_but_keep_process_owned() {
        let id = ProcessIdentity([9; 16]);
        let mut supervisor = Supervisor {
            bootstrap: None,
            _same_thread: std::marker::PhantomData,
            child: Some(Box::new(FailedControl)),
            live: Some(id),
            admitted: true,
            journal: PathBuf::from("unused-test-journal"),
            admission_receipt: None,
        };
        assert!(supervisor.stop().is_err());
        assert!(!supervisor.admits(id));
        assert!(supervisor.poll_reap().is_err());
        assert!(supervisor.occupied());
        assert_eq!(
            supervisor.start(&mut Command::new("unreachable")),
            Err(StartError::Occupied)
        );
    }
    #[test]
    fn reap_error_permanently_revokes_cooperative_ready_admission() {
        let identity = ProcessIdentity([6; 16]);
        let mut owner = Supervisor {
            bootstrap: Some(crate::bootstrap::ParentBootstrap::ready_fixture(identity)),
            _same_thread: std::marker::PhantomData,
            child: Some(Box::new(FailedControl)),
            live: Some(identity),
            admitted: true,
            journal: PathBuf::from("unused-test-journal"),
            admission_receipt: None,
        };
        assert!(owner.admits(identity));
        assert!(owner.poll_reap().is_err());
        assert!(!owner.admits(identity));
        assert_eq!(
            owner.poll_worker().unwrap(),
            crate::bootstrap::WorkerState::Terminal
        );
        assert!(!owner.admits(identity));
        assert!(owner.occupied());
    }
    #[test]
    fn cooperative_non_main_thread_refuses_before_child_or_admission() {
        use std::os::unix::fs::DirBuilderExt;
        std::thread::spawn(|| {
            let path = std::env::temp_dir().join(format!(
                "radio-main-thread-{}-{}",
                std::process::id(),
                ProcessIdentity::new().unwrap().hex()
            ));
            fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
            let journal = path.join("admission");
            let mut owner = Supervisor::new(journal.clone());
            let mut worker =
                crate::bootstrap::WorkerCommand::new(std::path::Path::new("/bin/true")).unwrap();
            assert_eq!(owner.start_worker(&mut worker), Err(StartError::Io));
            assert!(!owner.occupied());
            assert!(!journal.exists());
            fs::remove_dir(path).unwrap();
        })
        .join()
        .unwrap();
    }
    #[test]
    fn frame_requires_connection_owner_and_attempt_identity() {
        let live = ProcessIdentity([8; 16]);
        let old = ProcessIdentity([7; 16]);
        let supervisor = Supervisor {
            bootstrap: None,
            _same_thread: std::marker::PhantomData,
            child: None,
            live: Some(live),
            admitted: true,
            journal: PathBuf::from("unused-test-journal"),
            admission_receipt: None,
        };
        assert!(supervisor.admits_frame(live, live, 1));
        assert!(!supervisor.admits_frame(old, old, 1));
        assert!(!supervisor.admits_frame(old, live, 1));
        assert!(!supervisor.admits_frame(live, old, 1));
        assert!(!supervisor.admits_frame(live, live, 0));
        assert!(!supervisor.admits_frame(live, live, 2));
    }
    #[test]
    #[cfg_attr(
        target_arch = "aarch64",
        ignore = "requires native ARM64 kernel process execution; run tools/radio-process-vm.py"
    )]
    fn spawn_failure_leaves_durable_admission_and_restart_refuses_launch() {
        use std::os::unix::fs::DirBuilderExt;
        let path = std::env::temp_dir().join(format!(
            "pokelinux-admission-{}-{}",
            std::process::id(),
            ProcessIdentity::new().unwrap().hex()
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        let journal = path.join("admission");
        let mut supervisor = Supervisor::new(journal.clone());
        let mut command = Command::new(path.join("missing-executable"));
        assert_eq!(supervisor.start(&mut command), Err(StartError::Io));
        assert!(!supervisor.occupied());
        let metadata = fs::metadata(&journal).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        let record = fs::read_to_string(&journal).unwrap();
        let fields: Vec<_> = record.lines().collect();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], "admission-v1");
        assert!(ProcessIdentity::from_hex(fields[1]).is_some());
        assert_eq!(fields[2], "1");
        let mut restarted = Supervisor::new(journal.clone());
        assert_eq!(
            restarted.start(&mut command),
            Err(StartError::AdmissionUncertain)
        );
        fs::remove_file(journal).unwrap();
        fs::remove_dir(path).unwrap();
    }
    #[test]
    fn durable_admission_survives_supervisor_recreation_without_process_emulation() {
        use std::os::unix::fs::DirBuilderExt;
        let path = std::env::temp_dir().join(format!(
            "pokelinux-journal-{}-{}",
            std::process::id(),
            ProcessIdentity::new().unwrap().hex()
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        let journal = path.join("admission");
        let id = ProcessIdentity::new().unwrap();
        let mut supervisor = Supervisor::new(journal.clone());
        supervisor.record_admission(id).unwrap();
        assert_eq!(
            fs::read_to_string(&journal).unwrap(),
            format!("admission-v1\n{}\n1\n", id.hex())
        );
        assert_eq!(
            fs::metadata(&journal).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let mut restarted = Supervisor::new(journal.clone());
        assert_eq!(
            restarted.start(&mut Command::new("unreachable")),
            Err(StartError::AdmissionUncertain)
        );
        fs::remove_file(journal).unwrap();
        fs::remove_dir(path).unwrap();
    }
    #[test]
    fn stop_revokes_output_before_reap_and_keeps_capacity() {
        let release = Arc::new(AtomicBool::new(false));
        let id = ProcessIdentity([7; 16]);
        let mut supervisor = Supervisor {
            bootstrap: None,
            _same_thread: std::marker::PhantomData,
            child: Some(Box::new(HeldReap(release.clone()))),
            live: Some(id),
            admitted: true,
            journal: std::path::PathBuf::from("unused-test-journal"),
            admission_receipt: None,
        };
        assert!(supervisor.admits(id));
        supervisor.stop().unwrap();
        assert!(!supervisor.admits(id), "stop retained publication rights");
        assert!(supervisor.poll_reap().unwrap().is_none());
        assert_eq!(
            supervisor.start(&mut Command::new("unreachable")),
            Err(StartError::Occupied)
        );
        release.store(true, Ordering::SeqCst);
        assert!(supervisor.poll_reap().unwrap().is_some());
        assert_eq!(
            supervisor.start(&mut Command::new("unreachable")),
            Err(StartError::AdmissionUncertain)
        );
    }
    #[test]
    fn delayed_reap_never_discards_owned_process() {
        let mut supervisor = Supervisor {
            bootstrap: None,
            _same_thread: std::marker::PhantomData,
            child: Some(Box::new(HeldReap(Arc::new(AtomicBool::new(false))))),
            live: Some(ProcessIdentity([7; 16])),
            admitted: true,
            journal: std::path::PathBuf::from("unused-test-journal"),
            admission_receipt: None,
        };
        assert!(supervisor.poll_reap().unwrap().is_none());
        assert!(supervisor.child.is_some(), "unreaped process was discarded");
    }
    #[test]
    #[cfg_attr(
        target_arch = "aarch64",
        ignore = "requires native kernel process execution"
    )]
    fn synced_receipt_survives_failed_spawn_without_fabricating_restart_receipt() {
        use crate::deadline::{BootClock, Clock};
        use std::os::unix::fs::DirBuilderExt;
        let path = std::env::temp_dir().join(format!(
            "pokelinux-sync-receipt-{}-{}",
            std::process::id(),
            ProcessIdentity::new().unwrap().hex()
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        let journal = path.join("admission");
        let mut owner = Supervisor::new(journal.clone());
        assert_eq!(owner.admission_receipt(), None);
        let before: u64 = BootClock.now().as_nanos().try_into().unwrap();
        assert_eq!(
            owner.start(&mut Command::new(path.join("missing-executable"))),
            Err(StartError::Io)
        );
        let after: u64 = BootClock.now().as_nanos().try_into().unwrap();
        assert!(!owner.occupied());
        let metadata = fs::symlink_metadata(&journal).unwrap();
        assert!(metadata.is_file());
        assert_eq!(metadata.uid(), rustix::process::geteuid().as_raw());
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        assert_eq!(metadata.nlink(), 1);
        let marker = fs::read_to_string(&journal).unwrap();
        let fields: Vec<_> = marker.lines().collect();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], "admission-v1");
        assert_eq!(fields[2], "1");
        let identity = ProcessIdentity::from_hex(fields[1]).unwrap();
        println!("RECEIPT-CONTROL actual private0600 journal/valid marker, failed spawn, no child");
        let receipt = owner
            .admission_receipt()
            .expect("MISSING_POST_SYNC_ADMISSION_RECEIPT");
        assert_eq!(receipt.identity, identity);
        assert!(before <= receipt.sync_completed_before_spawn_at_boottime_ns);
        assert!(receipt.sync_completed_before_spawn_at_boottime_ns <= after);
        assert_eq!(receipt.journal_device, metadata.dev());
        assert_eq!(receipt.journal_inode, metadata.ino());
        assert_eq!(receipt.journal_uid, metadata.uid());
        assert_eq!(receipt.journal_mode, metadata.mode());
        assert_eq!(
            owner.start(&mut Command::new("unreachable")),
            Err(StartError::AdmissionUncertain)
        );
        assert_eq!(owner.admission_receipt(), Some(receipt));
        owner.stop().unwrap();
        assert_eq!(owner.poll_reap().unwrap(), None);
        assert_eq!(owner.admission_receipt(), Some(receipt));
        let mut restarted = Supervisor::new(journal.clone());
        assert_eq!(
            restarted.start(&mut Command::new("unreachable")),
            Err(StartError::AdmissionUncertain)
        );
        assert_eq!(restarted.admission_receipt(), None);
        assert_eq!(fs::read_to_string(&journal).unwrap(), marker);
        fs::remove_file(journal).unwrap();
        fs::remove_dir(path).unwrap();
    }
}
