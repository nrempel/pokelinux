#!/usr/bin/env python3
"""Fetch pinned public source; never download proprietary factory inputs."""
import argparse
import hashlib
import json
from pathlib import Path
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


def download(url):
    with urllib.request.urlopen(url, timeout=60) as response:
        return response.read()


def fetch(url, target, expected, *, git_blob=False, read=download):
    target = Path(target)
    data = target.read_bytes() if target.exists() else read(url)
    digest = (hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
              if git_blob else hashlib.sha256(data).hexdigest())
    if digest != expected:
        raise ValueError(f'Checksum mismatch: {target}')
    if not target.exists():
        target.parent.mkdir(parents=True, exist_ok=True)
        temporary = target.with_name(target.name + '.partial')
        temporary.write_bytes(data)
        temporary.replace(target)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--omarchy', action='store_true', help='Also fetch the exact selected Omarchy source files')
    args = parser.parse_args()
    for name, source in json.loads((ROOT / 'sources.lock.json').read_text()).items():
        url = source['repository'].replace('https://github.com/', 'https://codeload.github.com/') + '/tar.gz/' + source['commit']
        fetch(url, ROOT / 'artifacts/sources' / (name + '.tar.gz'), source['archive_sha256'])
        print(f'Verified {name} at {source["commit"]}')
    if args.omarchy:
        manifest = json.loads((ROOT / 'dependencies/omarchy-files.lock.json').read_text())
        base = 'https://raw.githubusercontent.com/omacom/omarchy/' + manifest['commit'] + '/'
        for name, digest in manifest['files'].items():
            relative = Path(name)
            if relative.is_absolute() or '..' in relative.parts:
                raise ValueError('Unsafe source path')
            target = ROOT / 'sim/omarchy-source' / relative
            fetch(base + name, target, digest, git_blob=True)
            target.chmod(int(manifest['modes'][name], 8) & 0o777)
        (ROOT / 'sim/omarchy-source/source-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
        print(f'Verified {len(manifest["files"])} Omarchy files at {manifest["commit"]}')


if __name__ == '__main__':
    main()
