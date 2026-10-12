#!/usr/bin/env python3
"""Own the isolated original NetworkManager qualification container."""
import argparse
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--output', required=True)
parser.add_argument('--client-arch', choices=['x86_64', 'aarch64_user'], default='x86_64')
args = parser.parse_args()
repo = Path(__file__).resolve().parent.parent
output = (repo / args.output).resolve()
output.mkdir(parents=True, exist_ok=False)
output.chmod(0o770)
name = 'pokelinux-connectivity-' + str(os.getpid())
command = ['docker', 'run', '--rm', '--name', name, '--network', 'none', '--read-only',
           '--env', 'POKELINUX_CONNECTIVITY_CLIENT_ARCH=' + args.client_arch, '--cpus', '2', '--memory', '512m', '--pids-limit', '128', '--cap-drop', 'ALL',
           '--user', '0:1000', '--cap-add', 'SETUID', '--cap-add', 'SETGID', '--cap-add', 'KILL', '--security-opt', 'no-new-privileges',
           '--tmpfs', '/tmp:rw,nosuid,nodev,size=32m,mode=1777',
           '--tmpfs', '/run:rw,nosuid,nodev,size=16m',
           '--tmpfs', '/var/lib/NetworkManager:rw,nosuid,nodev,size=16m',
           '--mount', 'type=bind,src=' + str(repo / 'tools/native-connectivity-fixture.py') + ',dst=/fixture.py,readonly',
           '--mount', 'type=bind,src=' + str(repo / 'artifacts/native-connectivity/target') + ',dst=/target,readonly',
           '--mount', 'type=bind,src=' + str(output) + ',dst=/results',
           'pokelinux-native-connectivity:20261012', 'python3', '-u', '/fixture.py']
try:
    completed = subprocess.run(command, timeout=60)
finally:
    # This unique name is owned solely by this runner; preserve unrelated containers.
    exists = subprocess.run(['docker', 'container', 'inspect', name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0
    if exists:
        subprocess.run(['docker', 'rm', '-f', name], check=True, stdout=subprocess.DEVNULL)
result = json.loads((output / 'result.json').read_text())
result['owned_container_absent'] = subprocess.run(['docker', 'container', 'inspect', name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode != 0
(output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result, indent=2))
assert completed.returncode == 0 and result['passed'] and result['owned_container_absent'], result
