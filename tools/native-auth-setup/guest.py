#!/usr/bin/env python3
"""Private no-workload guest setup: no account, PAM, worker or module changes."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import platform
import resource
import shutil
import subprocess

R = Path('/results')
P = Path('/workspace')
I = Path('/run/pokelinux-auth-scope-test')
result = {'schema': 1, 'passed': False, 'qualified_setup_GREEN': False,
          'scope': 'generic Arch ARM injected absent pidfd parent setup boundary; no workload',
          'host_changes': False, 'credentials_created_or_exported': False,
          'worker_or_PAM_executed': False, 'phone_accessed': False, 'direct_waits': []}


def require(value, label):
    if not value:
        raise RuntimeError(label)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


try:
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    libc = ctypes.CDLL(None, use_errno=True)
    require(os.getuid() == os.geteuid() == 0 and libc.prctl(36, 1, 0, 0, 0) == 0,
            'guest_root_subreaper')
    require(platform.release() == '7.1.6-1-aarch64-ARCH', 'selected_kernel')
    result['kernel_release'] = platform.release()
    selected = json.loads((R / 'selection.json').read_text())
    require(set(selected['workspace_files']) == {'auth-scope-fixture'}, 'only_no_workload_fixture_selected')
    for name, digest in selected['workspace_files'].items():
        require(sha(P / name) == digest, 'workspace_pin')
    I.mkdir(mode=0o755)
    shutil.copyfile(P / 'auth-scope-fixture', I / 'auth-scope-fixture')
    (I / 'auth-scope-fixture').chmod(0o755)
    require(sha(I / 'auth-scope-fixture') == selected['workspace_files']['auth-scope-fixture'], 'installed_ELF_pin')
    child = subprocess.Popen(['/usr/bin/systemd-run', '--unit=pokelinux-auth-scope-test.service',
        '--property=Type=exec', '--property=Delegate=yes', '--property=NoNewPrivileges=no',
        '--property=ProtectControlGroups=no', '--property=KillMode=control-group', '--wait', '--pipe',
        '/usr/bin/python3', '/results/source/delegated.py'], stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        stdout, stderr = child.communicate(timeout=35)
    except subprocess.TimeoutExpired:
        child.kill()
        stdout, stderr = child.communicate(timeout=3)
        raise RuntimeError('delegated_service_CLI_timeout')
    finally:
        result['direct_waits'].append({'label': 'explicit_private_delegated_service_CLI',
            'pid': child.pid, 'actual_wait': True, 'exit': child.returncode})
    require(child.returncode == 0, 'delegated_service_CLI_failed')
    delegated = json.loads((R / 'delegated-result.json').read_text())
    result['delegated'] = delegated
    for name, digest in selected['workspace_files'].items():
        require(sha(P / name) == digest, 'workspace_post_pin')
    result['selected_input_closure'] = True
    try:
        os.waitpid(-1, os.WNOHANG)
        raise RuntimeError('expected_outer_global_ECHILD')
    except ChildProcessError:
        result['outer_final_ECHILD'] = True
    result['qualified_setup_GREEN'] = bool(delegated.get('qualified_setup_GREEN')
        and delegated.get('passed') and result['selected_input_closure'] and result['outer_final_ECHILD'])
    result['passed'] = result['qualified_setup_GREEN']
    result['classification'] = ('QUALIFIED_SETUP_FAULT_RESPONSIVE_QUARANTINE_GREEN'
        if result['passed'] else 'UNQUALIFIED_SETUP_FAULT_GREEN_ATTEMPT')
except BaseException as error:
    result['error_type'] = type(error).__name__
    if isinstance(error, RuntimeError):
        result['error_code'] = str(error)
finally:
    with (R / 'result.json').open('x') as output:
        json.dump(result, output, indent=2)
        output.write('\n')
    print('NATIVE_AUTH_SCOPE_SETUP_TERMINAL ' + json.dumps({
        'qualified_setup_GREEN': result.get('qualified_setup_GREEN', False),
        'error_code': result.get('error_code')}), flush=True)
    # Existing disposable guest service owns poweroff.
