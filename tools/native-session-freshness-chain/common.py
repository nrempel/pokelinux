"""Shared offline freeze and exact-path helpers for the freshness process lane."""
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MAIN = Path('/home/ubuntu/Projects/pokelinux')
BASE = ROOT / 'artifacts/native-session-freshness-chain'
DOCKER = ['env', '-u', 'DOCKER_HOST', '-u', 'DOCKER_CONTEXT', '-u', 'DOCKER_TLS',
          '-u', 'DOCKER_TLS_VERIFY', '-u', 'DOCKER_CERT_PATH',
          'docker', '--host=unix:///var/run/docker.sock']
RUST_IMAGE = 'pixel-phone-rust:20261012-b472'
VM_IMAGE = 'pixel-graphics-sim:20261011-b472'
CRATES = ('native-session', 'native-lifecycle', 'native-kernel-identity')


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def write_json(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')


def hashes(root):
    return {str(p.relative_to(root)): sha(p) for p in sorted(root.rglob('*')) if p.is_file()}


def verify(root, mapping):
    for name, value in mapping.items():
        path = root / name
        assert path.is_file() and sha(path) == value, f'Frozen hash mismatch: {path}'


def label(value):
    if not value or any(c not in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in value):
        raise ValueError('Labels must contain only lowercase letters, numbers and hyphens')
    return value


def image_id(tag):
    return subprocess.check_output(DOCKER + ['image', 'inspect', tag, '--format', '{{.Id}}'], text=True).strip()
