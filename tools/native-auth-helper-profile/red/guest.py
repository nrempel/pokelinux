#!/usr/bin/env python3
"""Provision this disposable guest only; no PAM or credentials are changed."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import platform
import pwd
import resource
import shutil
import stat
import subprocess
import time

R = Path('/results')
P = Path('/workspace')
I = Path('/run/pokelinux-auth-containment-test')
UNIT = 'pokelinux-auth-containment-test.service'
out = {'schema': 1, 'passed': False, 'qualified_RED': False, 'direct_waits': [],
       'scope': 'actual Arch ARM authored privileged helper-profile control only',
       'credentials_used': False, 'PAM_executed': False, 'phone_accessed': False}
unit_launch_attempted = False

def require(ok, code):
    if not ok:
        raise RuntimeError(code)

def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

def call(argv, label, seconds=10):
    child = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        stdout, stderr = child.communicate(timeout=seconds)
    except subprocess.TimeoutExpired:
        child.kill()
        stdout, stderr = child.communicate(timeout=3)
        out['direct_waits'].append({'label': label, 'pid': child.pid, 'actual_wait': True, 'exit': child.returncode, 'timed_out': True})
        raise RuntimeError(label + '_timeout')
    out['direct_waits'].append({'label': label, 'pid': child.pid, 'actual_wait': True, 'exit': child.returncode, 'timed_out': False})
    require(child.returncode == 0, label + '_failed')
    return stdout, stderr

try:
    require(os.getuid() == os.geteuid() == 0 and platform.machine() == 'aarch64' and platform.release() == '7.1.6-1-aarch64-ARCH', 'fixed_disposable_ARM_guest_only')
    selected = json.loads((R / 'selection.json').read_text())
    require(selected['label'] == 'pokelinux-native-auth-helper-migration-red-root-1' and selected['network'] == 'none', 'fixed_selection')
    require(set(selected['workspace_files']) == {'helper-profile-launcher', 'helper-profile-probe'}, 'only_selected_authored_fixture_ELFs')
    require(not I.exists(), 'fresh_owned_install_path')
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    libc = ctypes.CDLL(None, use_errno=True)
    require(libc.prctl(36, 1, 0, 0, 0) == 0, 'outer_subreaper')
    account = pwd.getpwuid(1000)
    require(account.pw_uid == account.pw_gid == 1000, 'existing_fixed_account_NSS')
    del account
    out['existing_fixed_account_UID_GID'] = [1000, 1000]
    sysctl = Path('/proc/sys/fs/suid_dumpable')
    out['guest_suid_dumpable_before'] = sysctl.read_text().strip()
    sysctl.write_text('0\n')
    require(sysctl.read_text().strip() == '0', 'guest_only_protected_dump_policy')
    I.mkdir(mode=0o755)
    call(['/usr/bin/mount', '-t', 'tmpfs', '-o', 'suid,nodev,mode=0755,size=16m', 'tmpfs', str(I)], 'private_install_tmpfs_mount')
    for name, digest in selected['workspace_files'].items():
        require(sha(P / name) == digest, 'selected_ELF_input_pin')
        shutil.copyfile(P / name, I / name)
        os.chown(I / name, 0, 0)
        (I / name).chmod(0o6755 if name == 'helper-profile-probe' else 0o755)
    (I / 'armed').write_text('PKHC-VM-ONLY1\n')
    (I / 'armed').chmod(0o400)
    call(['/usr/bin/mount', '-o', 'remount,ro,suid,nodev', str(I)], 'private_install_readonly_remount')
    entries = []
    for line in Path('/proc/self/mountinfo').read_text().splitlines():
        left, right = line.split(' - ', 1)
        fields = left.split()
        if fields[4] == str(I):
            entries.append({'mountpoint': fields[4], 'options': fields[5].split(','), 'filesystem': right.split()[0]})
    require(len(entries) == 1 and entries[0]['filesystem'] == 'tmpfs' and 'ro' in entries[0]['options']
            and not {'nosuid', 'noexec'} & set(entries[0]['options']), 'actual_setID_enabled_readonly_private_mount')
    out['private_install_mount'] = entries[0]
    out['installed_files'] = []
    for name, digest in selected['workspace_files'].items():
        p = I / name
        m = p.lstat()
        mode = 0o6755 if name == 'helper-profile-probe' else 0o755
        require(m.st_uid == m.st_gid == 0 and stat.S_ISREG(m.st_mode) and stat.S_IMODE(m.st_mode) == mode and sha(p) == digest, 'actual_owned_installed_ELF')
        require('security.capability' not in os.listxattr(p), 'no_file_capability_attribute')
        out['installed_files'].append({'name': name, 'sha256': digest, 'uid': 0, 'gid': 0, 'mode': oct(mode), 'identity': [m.st_dev, m.st_ino], 'file_capabilities': False})
    observed, _ = call(['/usr/bin/systemctl', 'show', UNIT, '--property=LoadState', '--value'], 'prior_owned_unit_absence')
    require(observed.strip() == b'not-found', 'private_unit_absent_before_launch')
    unit_launch_attempted = True
    call(['/usr/bin/systemd-run', '--unit=' + UNIT, '--property=Type=exec', '--property=Delegate=yes',
          '--property=NoNewPrivileges=no', '--property=ProtectControlGroups=no', '--property=KillMode=control-group',
          '--property=RuntimeMaxSec=30s', '--property=TimeoutStopSec=2s', '--property=SendSIGKILL=yes',
          '--wait', '--pipe', '/usr/bin/python3', '/results/source/delegated.py'], 'private_delegated_service_CLI', 35)
    delegated = json.loads((R / 'delegated-result.json').read_text())
    out['delegated'] = delegated
    require(delegated.get('qualified_RED') and not delegated.get('error_type')
            and not delegated.get('cleanup_error_type'), 'delegated_actual_RED_and_cleanup_required')
    for name, digest in selected['workspace_files'].items():
        require(sha(P / name) == sha(I / name) == digest, 'postrun_ELF_closure')
    out['selected_inputs_unchanged'] = True
    try:
        os.waitpid(-1, os.WNOHANG)
        raise RuntimeError('unexpected_outer_child')
    except ChildProcessError:
        out['outer_final_ECHILD'] = True
    out['qualified_RED'] = bool(delegated.get('qualified_RED') and not delegated.get('passed'))
    out['classification'] = 'QUALIFIED_ROOT_DAC_SELF_MIGRATION_RED' if out['qualified_RED'] else 'UNQUALIFIED_HELPER_PROFILE_ATTEMPT'
except BaseException as error:
    out['error_type'] = type(error).__name__
    if isinstance(error, RuntimeError):
        out['error_code'] = str(error)
    # On any failure stop only this explicit guest unit. This assistance cannot
    # qualify the feature RED or native cleanup, even if fixture resources close.
    if unit_launch_attempted:
        try:
            cleanup = subprocess.run(['/usr/bin/systemctl', 'kill', '--kill-whom=all', '--signal=KILL', UNIT],
                                     capture_output=True, timeout=5)
            out['failure_only_owned_unit_kill_exit'] = cleanup.returncode
        except Exception as cleanup_error:
            out['failure_cleanup_error_type'] = type(cleanup_error).__name__
finally:
    out['completed_BOOTTIME_ns'] = time.clock_gettime_ns(time.CLOCK_BOOTTIME)
    with (R / 'result.json').open('x') as stream:
        json.dump(out, stream, indent=2)
        stream.write('\n')
    print('NATIVE_AUTH_HELPER_PROFILE_TERMINAL ' + json.dumps({'passed': False, 'qualified_RED': out['qualified_RED'], 'error_code': out.get('error_code')}), flush=True)
    # Existing pixel-sim.service owns disposable guest shutdown.
