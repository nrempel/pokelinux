#!/usr/bin/env python3
"""Package the native Phone app and Rust service for Arch; no installation or enrollment."""
import argparse
import gzip
import hashlib
import io
import os
import struct
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = '0.1.4-1'


def build(destination, binaries, architecture, epoch=0):
    machine = {'aarch64': 183, 'x86_64': 62}.get(architecture)
    if machine is None or epoch < 0:
        raise ValueError('unsupported architecture or build timestamp')
    files = {}
    for name in ('omarchy-phoned', 'omarchy-phone', 'omarchy-phone-app'):
        data = (Path(binaries) / name).read_bytes()
        if (len(data) < 64 or data[:7] != b'\x7fELF\x02\x01\x01'
                or struct.unpack_from('<H', data, 16)[0] not in (2, 3)
                or struct.unpack_from('<H', data, 18)[0] != machine):
            raise ValueError('native ELF64 executable with matching architecture required')
        files['usr/bin/' + name] = (data, 0o755)
    files['usr/lib/systemd/user/omarchy-phone.service'] = (
        (ROOT / 'transport/omarchy-phone.service').read_bytes(), 0o644)
    files['usr/share/applications/org.omarchy.Phone.desktop'] = (
        (ROOT / 'native-app/org.omarchy.Phone.desktop').read_bytes(), 0o644)
    files['usr/share/dbus-1/services/org.omarchy.Phone.service'] = (
        (ROOT / 'native-app/org.omarchy.Phone.service.in').read_text().replace(
            '@CMAKE_INSTALL_FULL_BINDIR@', '/usr/bin').encode(), 0o644)
    size = sum(len(data) for data, _ in files.values())
    info = f'''pkgname = omarchy-phone
pkgbase = omarchy-phone
pkgver = {VERSION}
pkgdesc = Native Phone app and Rust service for the Omarchy Android bridge
builddate = {epoch}
packager = Local offline build
size = {size}
arch = {architecture}
depend = glibc
depend = libgcc
depend = libstdc++
depend = systemd
depend = qt6-base>=6.8
depend = qt6-declarative>=6.8
depend = qt6-wayland>=6.8
'''
    files['.PKGINFO'] = (info.encode(), 0o644)
    mtree = '#mtree\n' + ''.join(
        f'./{name} type=file uid=0 gid=0 mode={mode:o} time={epoch}.0 '
        f'size={len(data)} sha256digest={hashlib.sha256(data).hexdigest()}\n'
        for name, (data, mode) in sorted(files.items()))
    files['.MTREE'] = (gzip.compress(mtree.encode(), mtime=0), 0o644)
    destination = Path(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_suffix(destination.suffix + '.tmp')
    with tarfile.open(temporary, 'w', format=tarfile.USTAR_FORMAT) as archive:
        for name, (data, mode) in sorted(files.items()):
            member = tarfile.TarInfo(name)
            member.size, member.mode, member.mtime = len(data), mode, epoch
            member.uid = member.gid = 0
            member.uname = member.gname = 'root'
            archive.addfile(member, io.BytesIO(data))
    temporary.replace(destination)
    return destination


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binaries', type=Path, required=True)
    parser.add_argument('--arch', choices=('aarch64', 'x86_64'), required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    print(build(args.output, args.binaries, args.arch,
                int(os.environ.get('SOURCE_DATE_EPOCH', '0'))))


if __name__ == '__main__':
    main()
