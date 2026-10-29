#!/usr/bin/env python3
"""Stage the explicit native menu adaptation into a fresh build-only overlay.

The original wrapper and default menu are unchanged. AppLibrary uses the pinned
Quickshell desktop-entry API; the installed Rust client requests actor-owned
launch. Merge these rows into a fresh complete bundle before generic installation.
This does not activate services or qualify arbitrary third-party desktop entries.
"""
import argparse
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile

REPO = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location(
    'deployment', REPO / 'native-session-owner/packaging/install_deployment.py')
deployment = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deployment)

ORIGINAL_LAUNCH = '''  function launch(desktopId, name) {
    var id = String(desktopId || "")
    if (!id) return
    root.beginLaunchFeedback(name)
    // Start gtk-launch inside a scope under app-graphical.slice so apps do not
    // inherit wayland-wm@.service. Keeping gtk-launch as the desktop-entry
    // resolver supports IDs with spaces and entries that UWSM rejects.
    // Keep the .desktop suffix or ids like org.telegram.desktop won't resolve.
    Util.execDetached("uwsm-app -- gtk-launch " + Util.shellQuote(id + ".desktop"))
  }
'''
NATIVE_LAUNCH = '''  function launch(desktopId, name) {
    var id = String(desktopId || "")
    if (!id) return
    var entry = DesktopEntries.byId(id)
    if (!entry || entry.id !== id || entry.noDisplay || root.isHiddenEntry(entry)) return
    root.beginLaunchFeedback(name)
    // Use Quickshell's parsed command and working directory. Native desktop
    // entries invoke fixed Rust clients; their services own app lifetime.
    entry.execute()
  }
'''


def stage(archive, root, manifest):
    lock = json.loads((REPO / 'dependencies/omarchy-files.lock.json').read_text(),
                      object_pairs_hook=deployment.unique)
    archive_pin = json.loads((REPO / 'tools/native-omarchy-theme/assets.json').read_text(),
                             object_pairs_hook=deployment.unique)
    data, _ = deployment.regular(archive, 256 * 1024 * 1024)
    if (lock['commit'] != archive_pin['commit']
            or hashlib.sha256(data).hexdigest() != archive_pin['archive_sha256']):
        raise ValueError('pinned public Omarchy archive required')
    names = {
        'bin/omarchy-shell': 'usr/bin/omarchy-shell',
        'default/omarchy/omarchy-menu.jsonc':
            'usr/share/pokelinux/omarchy/default/omarchy/omarchy-menu.jsonc',
        'shell/services/AppLibrary.qml':
            'usr/share/pokelinux/omarchy/shell/services/AppLibrary.qml',
    }
    plan = {}
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:*') as source:
        for name, target in names.items():
            item = source.getmember('omarchy-' + lock['commit'] + '/' + name)
            mode = int(lock['modes'][name][-3:], 8)
            # GitHub tar headers carry group-write bits; install canonical Git
            # modes, retaining the archive's authenticated executable-bit class.
            if (not item.isfile() or item.size > deployment.MAX_FILE
                    or mode not in (0o644, 0o755)
                    or item.mode & 0o111 != mode & 0o111):
                raise ValueError('pinned regular menu source and executable class required')
            body = source.extractfile(item).read(item.size + 1)
            blob = b'blob ' + str(len(body)).encode() + b'\0' + body
            if len(body) != item.size or hashlib.sha1(blob).hexdigest() != lock['files'][name]:
                raise ValueError('menu source differs from pinned Git blob')
            if name == 'shell/services/AppLibrary.qml':
                text = body.decode()
                if text.count(ORIGINAL_LAUNCH) != 1:
                    raise ValueError('exact pinned launch function required')
                body = text.replace(ORIGINAL_LAUNCH, NATIVE_LAUNCH, 1).encode()
            plan[target] = (body, mode)
    rows = {name: {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest(),
                   'mode': mode} for name, (body, mode) in plan.items()}
    deployment.no_links(root)
    deployment.no_links(manifest)
    if root.exists() or manifest.exists():
        raise ValueError('fresh overlay root and manifest required')
    document = {'schema': 1, 'files': rows, 'source': {
        'commit': lock['commit'], 'archive_sha256': archive_pin['archive_sha256'],
        'original_git_blobs': {name: lock['files'][name] for name in names},
        'adaptation': 'AppLibrary.launch uses DesktopEntry.execute; wrapper and default menu unchanged',
    }}
    with tempfile.TemporaryDirectory(prefix='pokelinux-menu-stage-') as temporary:
        selected = Path(temporary) / 'source'
        selected.mkdir()
        for name, (body, mode) in plan.items():
            target = selected / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(body)
            target.chmod(mode)
        private_manifest = Path(temporary) / 'manifest.json'
        private_manifest.write_text(json.dumps(document, indent=2) + '\n')
        root.mkdir(mode=0o755)
        deployment.install(selected, private_manifest, root)
        with manifest.open('x') as output:
            output.write(private_manifest.read_text())
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['archive', 'root', 'manifest']:
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    rows = stage(*(deployment.absolute(getattr(args, name))
                   for name in ['archive', 'root', 'manifest']))
    print(json.dumps({'files': rows, 'services_activated': False}))


if __name__ == '__main__':
    main()
