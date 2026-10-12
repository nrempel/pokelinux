#!/usr/bin/env python3
"""Original NetworkManager on a private bus/network namespace, read-only Rust observer."""
import json
import os
import pathlib
import select
import signal
import subprocess
import time

os.umask(0o002)
ROOT = pathlib.Path('/tmp/pokelinux-connectivity')
ROOT.mkdir(mode=0o777)
ROOT.chmod(0o777)
OUT = pathlib.Path('/results')
children = []
logs = []
stages = []
result = {'passed': False, 'scope': 'original NetworkManager, private real D-Bus, no managed physical interface',
          'hardware_verified': False, 'network': 'none', 'stages': stages,
          'original_daemon_architecture': 'x86_64',
          'client_execution': os.environ.get('POKELINUX_CONNECTIVITY_CLIENT_ARCH', 'x86_64')}


def launch(command, label, **kwargs):
    log = open(OUT / (label + '.log'), 'wb')
    logs.append(log)
    process = subprocess.Popen(command, stderr=log, **kwargs)
    children.append((label, process))
    return process


def line(process, timeout=3):
    assert select.select([process.stdout], [], [], timeout)[0], 'owned client response timeout'
    value = process.stdout.readline()
    assert value, 'owned client exited before response'
    return json.loads(value)


def ask(operation):
    observer.stdin.write(json.dumps({'op': operation}) + '\n')
    observer.stdin.flush()
    value = line(observer)
    with open(OUT / 'observations.jsonl', 'a') as output:
        output.write(json.dumps({'op': operation, 'result': value}) + '\n')
    return value


def start_nm(label):
    log = open(OUT / (label + '-stdout.log'), 'wb')
    logs.append(log)
    child = launch(['/usr/sbin/NetworkManager', '--no-daemon', '--config', str(ROOT / 'NetworkManager.conf'),
                    '--pid-file', str(ROOT / 'nm.pid'), '--state-file', str(ROOT / 'nm.state')], label,
                   stdout=log, env={'PATH': '/usr/bin:/usr/sbin:/bin', 'DBUS_SYSTEM_BUS_ADDRESS': address})
    deadline = time.monotonic() + 15
    while True:
        assert child.poll() is None, 'original NetworkManager startup failed'
        pin = ask('pin')
        if pin['ok']:
            observation = ask('snapshot')
            if observation['ok']:
                return child, observation['snapshot']
        assert time.monotonic() < deadline, 'original daemon readiness timeout'
        time.sleep(0.05)


try:
    (ROOT / 'bus.conf').write_text('''<busconfig>
<type>system</type><listen>unix:path=/tmp/pokelinux-connectivity/bus</listen>
<auth>EXTERNAL</auth><policy context="default"><allow user="*"/>
<allow own="*"/><allow send_destination="*"/><allow receive_sender="*"/>
</policy></busconfig>''')
    (ROOT / 'NetworkManager.conf').write_text('''[main]
plugins=keyfile
dns=none
rc-manager=unmanaged
auth-polkit=false
no-auto-default=*
[device-isolated]
match-device=*
managed=0
[connectivity]
enabled=false
''')
    bus = launch(['dbus-daemon', '--nofork', '--print-address=1', '--config-file=' + str(ROOT / 'bus.conf')],
                 'dbus', stdout=subprocess.PIPE, text=True)
    assert select.select([bus.stdout], [], [], 3)[0], 'private bus startup timeout'
    address = bus.stdout.readline().strip()
    assert address.startswith('unix:'), address
    observer_command = (['qemu-aarch64', '-L', '/usr/aarch64-linux-gnu', '/target/aarch64-unknown-linux-gnu/release/nm-boundary', address]
                        if result['client_execution'] == 'aarch64_user' else ['/target/debug/nm-boundary', address])
    observer = launch(observer_command, 'rust-observer', user=1000, group=1000,
                      stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    assert not ask('pin')['ok']
    stages.append('absence_does_not_autostart_or_report_network')
    nm, observed = start_nm('nm-first')
    assert observed['state'] == 20 and observed['connectivity'] == 0, observed
    assert all(device['device_type'] != 2 for device in observed['devices']), observed
    result['original_version'] = observed['version']
    result['initial_devices'] = observed['devices']
    first_owner = observed['owner']
    stages.append('actual_properties_no_wifi_or_reachability_claim')
    expired = ask('snapshot_expired')
    assert not expired['ok'], 'successful daemon response published after boot-time deadline: ' + str(expired)
    stages.append('late_success_expires_with_injected_boot_time')
    nm.send_signal(signal.SIGSTOP)
    start = time.monotonic()
    paused = ask('snapshot')
    elapsed = time.monotonic() - start
    assert not paused['ok'] and elapsed < 1, (paused, elapsed)
    result['paused_read_elapsed_seconds'] = elapsed
    nm.send_signal(signal.SIGCONT)
    assert ask('snapshot')['ok']
    stages.append('actual_stopped_daemon_read_timeout')
    nm.kill()
    assert nm.wait(timeout=3) == -9
    assert not ask('snapshot')['ok']
    stages.append('actual_daemon_death_refuses_old_snapshot')
    # Start the replacement without pinning a new Rust owner. Readiness is independent.
    nm_second = launch(['/usr/sbin/NetworkManager', '--no-daemon', '--config', str(ROOT / 'NetworkManager.conf'),
                        '--pid-file', str(ROOT / 'nm.pid'), '--state-file', str(ROOT / 'nm.state')], 'nm-second',
                       stdout=subprocess.DEVNULL,
                       env={'PATH': '/usr/bin:/usr/sbin:/bin', 'DBUS_SYSTEM_BUS_ADDRESS': address})
    deadline = time.monotonic() + 15
    while True:
        assert nm_second.poll() is None, 'replacement original daemon failed'
        probe = subprocess.run(['dbus-send', '--bus=' + address, '--print-reply', '--reply-timeout=500',
                                '--dest=org.freedesktop.DBus', '/org/freedesktop/DBus',
                                'org.freedesktop.DBus.GetNameOwner', 'string:org.freedesktop.NetworkManager'],
                               capture_output=True, text=True, timeout=1)
        if probe.returncode == 0:
            # Name acquisition precedes object readiness by a few scheduler turns.
            time.sleep(0.2)
            break
        assert time.monotonic() < deadline
        time.sleep(0.05)
    stale = ask('snapshot')
    assert not stale['ok'], 'old pinned endpoint published replacement daemon under prior identity: ' + str(stale)
    stages.append('restart_requires_explicit_owner_replacement')
    replacement = ask('pin')
    assert replacement['ok'] and replacement['owner'] != first_owner, replacement
    refreshed = ask('snapshot')
    assert refreshed['ok'] and refreshed['snapshot']['owner'] == replacement['owner'], refreshed
    stages.append('explicit_new_owner_observation')
    bus.kill()
    assert bus.wait(timeout=3) == -9
    assert not ask('snapshot')['ok']
    stages.append('bus_death_refuses_cached_success')
    result['passed'] = True
except BaseException as error:
    result['failure'] = repr(error)
    raise
finally:
    for label, child in reversed(children):
        if child.poll() is None:
            child.kill()
        child.wait(timeout=5)
    result['owned_children_reaped'] = {label: child.returncode for label, child in children}
    (OUT / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    for log in logs:
        log.close()
