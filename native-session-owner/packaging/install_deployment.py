#!/usr/bin/env python3
"""Copy one explicit authenticated deployment bundle into a build-only DESTDIR.

No policy defaults, network access, user creation or unit activation. All manifest,
source and destination-shape checks precede writes. Subsequent I/O failure may
leave a partial staging tree; this is not a whole-tree transaction.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import secrets
import stat

MAX_FILE = 8 * 1024 * 1024
MAX_TOTAL = 64 * 1024 * 1024
MAX_FILES = 4096


def reject(message):
    raise ValueError(message)


def absolute(value):
    path = Path(value)
    if not value or not path.is_absolute() or '..' in path.parts:
        reject('explicit absolute path required')
    return path


def no_links(path):
    for part in reversed([path, *path.parents]):
        if part.exists() or part.is_symlink():
            if stat.S_ISLNK(part.lstat().st_mode):
                reject('symlink in selected path')


def identity(info):
    return (info.st_dev, info.st_ino, info.st_size, info.st_mode,
            info.st_uid, info.st_gid, info.st_mtime_ns, info.st_nlink)


def regular(path, maximum):
    no_links(path)
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1 or before.st_size > maximum:
            reject('bounded unlinked regular input required')
        chunks = []
        length = 0
        while True:
            block = os.read(fd, min(65536, maximum + 1 - length))
            if not block:
                break
            chunks.append(block)
            length += len(block)
            if length > maximum:
                reject('input exceeds byte bound')
        if length != before.st_size or identity(os.fstat(fd)) != identity(before):
            reject('input changed during read')
        return b''.join(chunks), before
    finally:
        os.close(fd)


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            reject('duplicate JSON key')
        result[key] = value
    return result


def directory(path):
    info = path.lstat()
    if (not stat.S_ISDIR(info.st_mode) or info.st_uid != os.geteuid()
            or info.st_mode & 0o022):
        reject('staging directory must be real, owned and nonwritable by others')


def prepare(root, manifest, destdir):
    no_links(root)
    if not root.is_dir() or destdir == Path('/'):
        reject('explicit bundle directory and build-only DESTDIR required')
    no_links(destdir)
    body, manifest_info = regular(manifest, 2 * 1024 * 1024)
    document = json.loads(body, object_pairs_hook=unique)
    if type(document.get('schema')) is not int or document['schema'] != 1:
        reject('manifest schema1 required')
    files = document.get('files')
    if not isinstance(files, dict) or not 1 <= len(files) <= MAX_FILES:
        reject('bounded nonempty files manifest required')
    plan = []
    names = set()
    total = 0
    for name, row in files.items():
        path = PurePosixPath(name)
        if (not name or path.is_absolute() or '..' in path.parts
                or path.as_posix() != name or '\\' in name or '\0' in name
                or len(path.parts) > 32 or len(name) > 4096):
            reject('canonical relative file path required')
        if (not isinstance(row, dict) or type(row.get('bytes')) is not int
                or not 0 <= row['bytes'] <= MAX_FILE
                or type(row.get('mode')) is not int or not 0 <= row['mode'] <= 0o777
                or row['mode'] & 0o022 or not row['mode'] & 0o400
                or not isinstance(row.get('sha256'), str)
                or not re.fullmatch('[0-9a-f]{64}', row['sha256'])):
            reject('bounded byte/hash/safe-mode row required')
        total += row['bytes']
        if total > MAX_TOTAL:
            reject('bundle exceeds aggregate byte bound')
        source = root / path
        data, info = regular(source, row['bytes'])
        if (len(data) != row['bytes'] or hashlib.sha256(data).hexdigest() != row['sha256']
                or stat.S_IMODE(info.st_mode) != row['mode']):
            reject('source bytes/hash/mode differ from manifest')
        names.add(name)
        plan.append((path, data, row['mode'], source, identity(info)))
    for path, *_ in plan:
        if any(parent.as_posix() in names for parent in path.parents if parent.as_posix() != '.'):
            reject('file and directory path collision')
        target = destdir / path
        for parent in reversed([target.parent, *target.parent.parents]):
            if parent == destdir or destdir in parent.parents:
                if parent.exists() or parent.is_symlink():
                    directory(parent)
        if target.exists() or target.is_symlink():
            info = target.lstat()
            if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
                reject('destination must be absent or an unlinked regular file')
    # Recheck every selected pathname before the first output mutation. Copying
    # subsequently uses only owned, fully authenticated bytes, not source paths.
    if identity(manifest.lstat()) != identity(manifest_info):
        reject('manifest identity changed')
    for _, _, _, source, expected in plan:
        if identity(source.lstat()) != expected:
            reject('source identity changed')
    return plan


def parents(path):
    if path.exists():
        directory(path)
        return
    parents(path.parent)
    path.mkdir(mode=0o755)


def install(root, manifest, destdir):
    plan = prepare(root, manifest, destdir)
    parents(destdir)
    for relative, data, mode, _, _ in plan:
        target = destdir / relative
        parents(target.parent)
        parent = os.open(target.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW)
        pending = '.pokelinux-install-' + secrets.token_hex(16)
        try:
            fd = os.open(pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC,
                         0o600, dir_fd=parent)
            try:
                view = memoryview(data)
                while view:
                    written = os.write(fd, view)
                    if written <= 0:
                        raise OSError('short staging write')
                    view = view[written:]
                os.fchmod(fd, mode)
                os.fsync(fd)
            finally:
                os.close(fd)
            os.replace(pending, relative.name, src_dir_fd=parent, dst_dir_fd=parent)
            os.fsync(parent)
        finally:
            try:
                os.unlink(pending, dir_fd=parent)
            except FileNotFoundError:
                pass
            os.close(parent)
    print(json.dumps({'schema': 1, 'files': len(plan), 'bytes': sum(len(v[1]) for v in plan),
                      'service_activation_performed': False, 'runtime_qualified': False}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['root', 'manifest', 'destdir']:
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    install(absolute(args.root), absolute(args.manifest), absolute(args.destdir))


if __name__ == '__main__':
    main()
