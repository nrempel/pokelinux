#!/usr/bin/env python3
"""Author a diagnostic-only diff from the immutable selected Source610aa bytes."""
import difflib
import hashlib
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'artifacts/native-display-app-generations'
SOURCE = BASE / 'app-generation-candidate-build-1/source/native-session/src/lib.rs'
EXPECTED = '610aa41e9c290ee1184ef994b74039485d4b1d485e553124e829312b12c5a220'

TRACE = r'''
// DIAGNOSTIC SOURCE COPY ONLY. No production hook or Source-frame alteration.
// Fixed112-byte nonblocking records, emitted only on rejection/terminal stop.
struct Diagnostic {
    fd: OwnedFd,
    attempted: u64,
    sent: u64,
    dropped: u64,
    summary: bool,
}
#[derive(Clone, Copy, Default)]
struct TraceFields {
    previous_sequence: u64,
    previous_sample: u64,
    sequence: u64,
    sample: u64,
    examined: u64,
    drain: u32,
    errno: i32,
    bytes: u32,
    rights_messages: u32,
    rights_handles: u32,
    recv_flags: u32,
    flags: u32,
}
impl Diagnostic {
    fn new() -> io::Result<Self> {
        let path = std::env::var_os("POKELINUX_SESSION_DIAGNOSTIC_TRACE")
            .ok_or(io::ErrorKind::InvalidInput)?;
        let fd = net::socket_with(
            net::AddressFamily::UNIX, net::SocketType::DGRAM,
            net::SocketFlags::CLOEXEC | net::SocketFlags::NONBLOCK, None,
        )?;
        net::connect(&fd, &net::SocketAddrUnix::new(std::path::Path::new(&path))?)?;
        Ok(Self { fd, attempted: 0, sent: 0, dropped: 0, summary: false })
    }
    fn record(&mut self, category: u16, f: TraceFields) {
        self.attempted = self.attempted.saturating_add(1);
        let mut bytes = [0u8; 112];
        bytes[..8].copy_from_slice(b"PKSDIAG1");
        bytes[8..10].copy_from_slice(&1u16.to_le_bytes());
        bytes[10..12].copy_from_slice(&category.to_le_bytes());
        bytes[12..16].copy_from_slice(&f.flags.to_le_bytes());
        let mut offset = 16;
        for value in [boottime_ns(), f.previous_sequence, f.previous_sample,
            f.sequence, f.sample, f.examined] {
            bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes()); offset += 8;
        }
        for value in [f.drain, f.errno as u32, f.bytes, f.rights_messages,
            f.rights_handles, f.recv_flags] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes()); offset += 4;
        }
        for value in [self.attempted, self.sent, self.dropped] {
            bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes()); offset += 8;
        }
        match net::send(&self.fd, &bytes, net::SendFlags::DONTWAIT | net::SendFlags::NOSIGNAL) {
            Ok(112) => self.sent = self.sent.saturating_add(1),
            _ => self.dropped = self.dropped.saturating_add(1),
        }
    }
    fn finish(&mut self) {
        if !self.summary { self.summary = true; self.record(0, TraceFields::default()); }
    }
}
'''


def transform(original):
    text = original

    def replace(old, new):
        nonlocal text
        assert text.count(old) == 1, old
        text = text.replace(old, new)

    replace('pub struct Observer {', TRACE + '\npub struct Observer {')
    replace('    failed: bool,\n}', '    failed: bool,\n    diagnostic: Diagnostic,\n}')
    replace('            failed: false,', '            failed: false,\n            diagnostic: Diagnostic::new()?,')
    replace('            let packet = match receive(self.fd.as_ref().ok_or(io::ErrorKind::NotConnected)?) {', '''            let context = TraceFields { previous_sequence: self.sequence,
                previous_sample: self.current.as_ref().map_or(0, |v| v.sampled_at_ns),
                drain: count, ..TraceFields::default() };
            let packet = match receive(self.fd.as_ref().ok_or(io::ErrorKind::NotConnected)?,
                &mut self.diagnostic, context) {''')
    replace('                Err(_) => return self.fail(io::ErrorKind::InvalidData),\n            };\n            let Some', '''                Err(e) => {
                    self.diagnostic.record(1, TraceFields { errno: e.raw_os_error().unwrap_or(0), ..context });
                    return self.fail(io::ErrorKind::InvalidData);
                }
            };
            let Some''')
    replace('            if count > MAX_DRAIN {\n                return self.fail(io::ErrorKind::InvalidData);', '''            if count > MAX_DRAIN {
                self.diagnostic.record(2, TraceFields { drain: count, bytes: bytes.len() as u32, ..context });
                return self.fail(io::ErrorKind::InvalidData);''')
    replace('                Err(_) => return self.fail(io::ErrorKind::InvalidData),', '''                Err(_) => {
                    self.diagnostic.record(3, TraceFields { drain: count, bytes: bytes.len() as u32, ..context });
                    return self.fail(io::ErrorKind::InvalidData);
                }''')
    replace('            if frame.source != self.source', '            let mut examined = 0;\n            if frame.source != self.source')
    replace('                || !Self::live_sample(&frame, boottime_ns())', '''                || !Self::live_sample(&frame, { examined = boottime_ns(); examined })''')
    replace('''            {
                return self.fail(io::ErrorKind::InvalidData);
            }
            if frame.focus.is_some() != peer.is_some()''', '''            {
                // Classify only after the unchanged composite guard rejected.
                // Static fields and the exact original live_sample examination
                // time are reused; no second live clock/authority check occurs.
                let category = if frame.source != self.source { 4 }
                else if self.sequence.checked_add(1) != Some(frame.sequence) { 5 }
                else if frame.locked && frame.focus.is_some() { 6 }
                else if !frame.locked && frame.lock_completed { 7 }
                else if frame.focus.as_ref().is_some_and(|f| f.pid == 0 || f.pid > i32::MAX as u32) { 8 }
                else if self.current.as_ref().is_some_and(|p| frame.sampled_at_ns < p.sampled_at_ns) { 9 }
                else if self.current.as_ref().is_some_and(|p| p.sampled_at_ns.checked_add(SAMPLE_BUDGET_NS).is_none_or(|until| frame.sampled_at_ns > until)) { 10 }
                else if frame.sampled_at_ns > examined { 11 } else { 12 };
                self.diagnostic.record(category, TraceFields { sequence: frame.sequence,
                    sample: frame.sampled_at_ns, examined, drain: count,
                    bytes: bytes.len() as u32, ..context });
                return self.fail(io::ErrorKind::InvalidData);
            }
            let mut type_examined = false;
            let mut typed = false;
            if frame.focus.is_some() != peer.is_some()''')
    replace('                    .is_some_and(|fd| !self.view.is_pidfd(fd).unwrap_or(false))', '''                    .is_some_and(|fd| { type_examined = true;
                        typed = self.view.is_pidfd(fd).unwrap_or(false); !typed })''')
    replace('''            {
                return self.fail(io::ErrorKind::InvalidData);
            }
            let focus = frame''', '''            {
                self.diagnostic.record(if type_examined { 14 } else { 13 }, TraceFields {
                    sequence: frame.sequence, sample: frame.sampled_at_ns, drain: count,
                    flags: u32::from(frame.focus.is_some()) | (u32::from(peer.is_some()) << 1)
                        | (u32::from(type_examined) << 2) | (u32::from(typed) << 3), ..context });
                return self.fail(io::ErrorKind::InvalidData);
            }
            let focus = frame''')
    replace('''                let Some(revision) = self.revision.checked_add(1) else {
                    return self.fail(io::ErrorKind::InvalidData);''', '''                let Some(revision) = self.revision.checked_add(1) else {
                    self.diagnostic.record(15, TraceFields { sequence: frame.sequence,
                        sample: frame.sampled_at_ns, drain: count, ..context });
                    return self.fail(io::ErrorKind::InvalidData);''')
    replace('fn receive(fd: &OwnedFd) -> io::Result<Option<SourcePacket>> {', 'fn receive(fd: &OwnedFd, diagnostic: &mut Diagnostic, context: TraceFields) -> io::Result<Option<SourcePacket>> {')
    replace('    let mut credentials = None;', '    let mut shape = u32::from(result.bytes == 0) | (u32::from(result.flags.contains(ReturnFlags::TRUNC)) << 1) | (u32::from(result.flags.contains(ReturnFlags::CTRUNC)) << 2);\n    let mut handles_count = 0;\n    let mut credentials = None;')
    replace('                for handle in handles {', '                for handle in handles {\n                    handles_count += 1;')
    replace('''                    } else {
                        invalid = true;
                    }''', '''                    } else {
                        invalid = true; shape |= 1 << 3;
                    }''')
    replace('            _ => invalid = true,', '            _ => { invalid = true; shape |= 1 << 4; },')
    replace('''    if invalid || rights_messages > 1 {
        return Err(io::ErrorKind::InvalidData.into());''', '''    if invalid || rights_messages > 1 {
        diagnostic.record(16, TraceFields { bytes: result.bytes as u32,
            rights_messages, rights_handles: handles_count, recv_flags: result.flags.bits(),
            flags: shape | (u32::from(rights_messages > 1) << 5), ..context });
        return Err(io::ErrorKind::InvalidData.into());''')
    replace('cred: credentials.ok_or(io::ErrorKind::PermissionDenied)?,', '''cred: credentials.ok_or_else(|| {
            diagnostic.record(17, TraceFields { bytes: result.bytes as u32,
                rights_messages, rights_handles: handles_count,
                recv_flags: result.flags.bits() as u32, ..context });
            io::ErrorKind::PermissionDenied
        })?,''')
    replace('''    fn stop(&mut self) {
        self.failed = true;''', '''    fn stop(&mut self) {
        self.diagnostic.finish();
        self.failed = true;''')
    # Fixed trace widths do not change the original usize drain counter.
    text = text.replace('drain: count', 'drain: count as u32')
    text = text.replace('recv_flags: result.flags.bits(),', 'recv_flags: result.flags.bits() as u32,')
    return text


def main():
    original = SOURCE.read_text()
    assert hashlib.sha256(SOURCE.read_bytes()).hexdigest() == EXPECTED
    current = transform(original)
    docker = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
              '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH', 'docker', '--host=unix:///var/run/docker.sock']
    image = 'sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6'
    current = subprocess.check_output(docker + ['run', '--rm', '-i', '--pull=never',
        '--network', 'none', '--cpus', '1', '--memory', '256m', '--pids-limit', '32',
        '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges', image,
        'rustfmt', '--edition', '2024', '--emit', 'stdout'], input=current, text=True)
    path = Path(__file__).with_name('diagnostic-session.rs')
    path.write_text(current)
    Path(__file__).with_name('diagnostic-source.patch').write_text(''.join(difflib.unified_diff(
        original.splitlines(True), current.splitlines(True),
        fromfile='a/native-session/src/lib.rs', tofile='b/native-session/src/lib.rs')))
    print('original', EXPECTED)
    print('diagnostic', hashlib.sha256(path.read_bytes()).hexdigest())


if __name__ == '__main__':
    main()
