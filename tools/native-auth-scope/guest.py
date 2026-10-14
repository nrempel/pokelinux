#!/usr/bin/env python3
"""Private disposable-guest setup; never changes host accounts/PAM/cgroups."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import platform
import resource
import shutil
import stat
import subprocess

R = Path('/results')
P = Path('/workspace')
I = Path('/run/pokelinux-auth-scope-test')
result = {'schema': 2, 'passed': False, 'feature_GREEN': False,
          'scope': 'generic Arch ARM selected trusted synthetic PAM recursive cleanup; Cancel/Stop and original deadline',
          'host_changes': False, 'credentials_created_or_exported': False,
          'migration_containment_qualified': False, 'phone_accessed': False,
          'direct_waits': [], 'setup_complete': False}


def require(value, label):
    if not value:
        raise RuntimeError(label)


def sha(path):
    with Path(path).open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def direct(argv, label, timeout):
    child = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                             stderr=subprocess.PIPE)
    try:
        out, err = child.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        child.kill()
        out, err = child.communicate(timeout=3)
        result['direct_waits'].append({'label': label, 'pid': child.pid,
                                      'actual_wait': True, 'exit': child.returncode,
                                      'timed_out': True})
        raise RuntimeError(label + '_timeout')
    result['direct_waits'].append({'label': label, 'pid': child.pid,
                                  'actual_wait': True, 'exit': child.returncode,
                                  'stdout_bytes': len(out), 'stderr_bytes': len(err)})
    require(child.returncode == 0, label + '_failed')


try:
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    libc = ctypes.CDLL(None, use_errno=True)
    require(os.getuid() == os.geteuid() == 0, 'guest_root_required')
    require(libc.prctl(36, 1, 0, 0, 0) == 0, 'outer_subreaper')
    require(platform.release() == '7.1.6-1-aarch64-ARCH', 'selected_kernel')
    result['kernel_release'] = platform.release()
    selected = json.loads((R / 'selection.json').read_text())
    for name, digest in selected['workspace_files'].items():
        require(sha(P / name) == digest, 'workspace_pin')
    pins = json.loads((P / 'selected-package-stack-pins.json').read_text())
    for item in pins['files']:
        path = Path(item['path'])
        require(sha(path) == item['sha256'], 'installed_public_stack_pin')
        metadata = path.stat()
        require(metadata.st_uid == 0 and not metadata.st_mode & 0o022,
                'installed_public_stack_owner')
    require(stat.S_IMODE(Path('/usr/bin/unix_chkpwd').stat().st_mode) == 0o6755,
            'unchanged_original_helper_mode')
    policy = Path('/proc/sys/fs/suid_dumpable')
    prior = policy.read_text().strip()
    require(prior in ('0', '1', '2'), 'guest_dumpable_policy')
    written = policy.write_text('0\n')
    require(policy.read_text().strip() == '0', 'guest_dumpable_policy_readback')
    result['guest_only_native_dumpable_policy'] = {'prior': prior, 'after': '0',
                                                   'actual_write_bytes': written}
    # The fixed worker loads PAM after dropping to UID1000. Root-owned readable
    # policy is safe here; 0700 ancestry would prevent the selected module load.
    I.mkdir(mode=0o755)
    (I / 'pam.d').mkdir(mode=0o755)
    for name in ('pam-worker', 'auth-scope-fixture'):
        shutil.copyfile(P / name, I / name)
        (I / name).chmod(0o755)
        require(sha(I / name) == selected['workspace_files'][name], 'installed_ELF_pin')
    for name in ('pam_owned_descendants.c', 'fixture_contract.h'):
        shutil.copyfile(P / name, I / name)
    direct(['/usr/bin/cc', '-std=c11', '-O2', '-fPIC', '-shared', '-Wall', '-Wextra',
            '-Werror', '-I' + str(I), str(I / 'pam_owned_descendants.c'), '-lpam',
            '-o', str(I / 'pam_owned_descendants.so')], 'actual_public_PAM_module_build', 20)
    (I / 'pam_owned_descendants.so').chmod(0o755)
    service = I / 'pam.d/pokelinux-auth-ownership-test'
    service.write_text('auth required /run/pokelinux-auth-scope-test/pam_owned_descendants.so '
                       'mode=descendant-block ready_fd=2\n')
    service.chmod(0o644)
    result['synthetic_service_sha256'] = sha(service)
    result['authored_module_ELF_sha256'] = sha(I / 'pam_owned_descendants.so')
    result['selected_public_headers'] = {
        name: sha(name) for name in ('/usr/include/security/pam_modules.h',
                                    '/usr/include/security/_pam_types.h')}
    result['compiler'] = {'path': str(Path('/usr/bin/cc').resolve()),
                          'sha256': sha(Path('/usr/bin/cc').resolve())}
    result['setup_complete'] = True
    direct(['/usr/bin/systemd-run', '--unit=pokelinux-auth-scope-test.service',
            '--property=Type=exec', '--property=Delegate=yes',
            '--property=NoNewPrivileges=no', '--property=ProtectControlGroups=no',
            '--property=KillMode=control-group', '--wait', '--pipe',
            '/usr/bin/python3', '/results/source/delegated.py'],
           'explicit_private_delegated_service_CLI', 35)
    delegated = json.loads((R / 'delegated-result.json').read_text())
    result['delegated'] = delegated
    result['feature_GREEN'] = bool(delegated.get('feature_GREEN') and delegated.get('passed')
        and len(delegated.get('groups', [])) == 2
        and all(group.get('passed') and group.get('closed') for group in delegated['groups'])
        and not delegated.get('outer_pidfd_cleanup_used')
        and not delegated.get('error_type') and not delegated.get('cleanup_error_type'))
    for item in pins['files']:
        require(sha(item['path']) == item['sha256'], 'installed_stack_post_pin')
    for name, digest in selected['workspace_files'].items():
        require(sha(P / name) == digest, 'workspace_post_pin')
    result['selected_input_closure'] = True
    try:
        os.waitpid(-1, os.WNOHANG)
        raise RuntimeError('outer_expected_ECHILD')
    except ChildProcessError:
        result['outer_final_ECHILD'] = True
    result['passed'] = bool(result['feature_GREEN'] and result['selected_input_closure']
                            and result['outer_final_ECHILD'])
    result['qualified_scope_GREEN'] = result['passed']
    result['classification'] = ('QUALIFIED_SYNTHETIC_PAM_RECURSIVE_SCOPE_TWO_GROUP_GREEN'
                                if result['passed'] else 'UNQUALIFIED_RECURSIVE_SCOPE_GREEN_ATTEMPT')
except BaseException as error:
    result['error_type'] = type(error).__name__
    if isinstance(error, RuntimeError):
        result['error_code'] = str(error)
finally:
    with (R / 'result.json').open('x') as output:
        json.dump(result, output, indent=2)
        output.write('\n')
    print('NATIVE_AUTH_SCOPE_TERMINAL ' + json.dumps({
        'qualified_scope_GREEN': result.get('qualified_scope_GREEN', False),
        'error_code': result.get('error_code')}), flush=True)
    # Existing pixel-sim.service owns poweroff after this directly waited setup.
