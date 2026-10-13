#!/usr/bin/env python3
"""Install the fixed Rust entry while retaining the exact original Omarchy action."""
import argparse
import hashlib
import os
from pathlib import Path
import shutil
import stat

ORIGINAL_SHA256 = 'f1953110658d819597d3c9d8df616350c362ee4c257227fcbdefac8275bbd596'
WRAPPER = b'#!/bin/sh\nexec /usr/libexec/pokelinux/lifecycle-lock\n'


def trusted(path):
    info = path.lstat()
    assert stat.S_ISREG(info.st_mode) and info.st_uid == 0
    assert info.st_mode & 0o6022 == 0 and info.st_mode & 0o111
    for parent in path.parents:
        info = parent.lstat()
        assert stat.S_ISDIR(info.st_mode) and info.st_uid == 0 and info.st_mode & 0o022 == 0


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary', required=True, type=Path)
    a = p.parse_args()
    assert os.getuid() == os.geteuid() == 0
    entry = Path('/usr/share/omarchy/bin/omarchy-system-lock')
    retained = Path('/usr/libexec/pokelinux/omarchy-system-lock-original')
    client = retained.with_name('lifecycle-lock')
    trusted(entry)
    assert hashlib.sha256(entry.read_bytes()).hexdigest() == ORIGINAL_SHA256
    assert a.binary.is_file() and not retained.exists() and not client.exists()
    retained.parent.mkdir(mode=0o755, parents=True, exist_ok=True)
    for parent in retained.parents:
        info = parent.lstat()
        assert stat.S_ISDIR(info.st_mode) and info.st_uid == 0 and info.st_mode & 0o022 == 0
    shutil.copyfile(entry, retained)
    retained.chmod(0o755)
    shutil.copyfile(a.binary, client)
    client.chmod(0o755)
    trusted(retained)
    trusted(client)
    assert hashlib.sha256(retained.read_bytes()).hexdigest() == ORIGINAL_SHA256
    # Publish only after both fixed destinations exist. The retained action is
    # outside the lookup entry, so its original shell commands cannot recurse.
    pending = entry.with_name('omarchy-system-lock.pending')
    with pending.open('xb') as output:
        output.write(WRAPPER)
        output.flush()
        os.fsync(output.fileno())
    pending.chmod(0o755)
    pending.replace(entry)
    assert entry.read_bytes() == WRAPPER


if __name__ == '__main__':
    main()
