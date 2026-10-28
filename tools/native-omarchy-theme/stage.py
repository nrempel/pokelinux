#!/usr/bin/env python3
"""Stage pinned genuine theme provisioning into a normal build-only deployment.

No network, home initialization, service activation, palette authoring or shell
patching occurs here. Merge the emitted rows into the normal bundle manifest;
install_deployment.py remains the sole generic build-only installer.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import tarfile
import tempfile

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
spec = importlib.util.spec_from_file_location('deployment', REPO / 'native-session-owner/packaging/install_deployment.py')
deployment = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deployment)

def stage(archive, provisioner, root, manifest):
    pins = json.loads((HERE / 'assets.json').read_text(), object_pairs_hook=deployment.unique)
    data, _ = deployment.regular(archive, 256 * 1024 * 1024)
    if hashlib.sha256(data).hexdigest() != pins['archive_sha256']:
        raise ValueError('pinned public Omarchy archive required')
    binary, _ = deployment.regular(provisioner, deployment.MAX_FILE)
    if binary[:6] != b'\x7fELF\x02\x01' or int.from_bytes(binary[18:20], 'little') not in (62, 183):
        raise ValueError('normal x86_64 or ARM64 native provisioner ELF required')
    prefix = 'omarchy-' + pins['commit'] + '/'
    plan = {}
    with tarfile.open(archive) as source:
        for name, row in pins['files'].items():
            item = source.getmember(prefix + name)
            if not item.isfile() or item.size != row['bytes']:
                raise ValueError('pinned regular theme asset required')
            body = source.extractfile(item).read(row['bytes'] + 1)
            if len(body) != row['bytes'] or hashlib.sha256(body).hexdigest() != row['sha256']:
                raise ValueError('theme source pin differs')
            plan['usr/share/pokelinux/omarchy/' + name] = (body, row['mode'])
        reference = pins['default_policy_reference']
        if hashlib.sha256(source.extractfile(prefix + reference['path']).read()).hexdigest() != reference['sha256']:
            raise ValueError('upstream initial-home policy reference differs')
    plan['usr/libexec/pokelinux/omarchy-theme-provision'] = (binary, 0o755)
    packaging = REPO / 'native-appearance/packaging'
    for source, target in [
        ('pokelinux-omarchy-theme.service', 'usr/lib/systemd/system/pokelinux-omarchy-theme.service'),
        ('normal-session-theme.conf', 'usr/lib/systemd/system/pokelinux-normal-session.target.d/theme.conf'),
        ('normal-session-theme.conf', 'usr/lib/systemd/system/pokelinux-session-owner.service.d/theme.conf'),
        ('normal-session-theme.conf', 'usr/lib/systemd/system/pokelinux-normal-appearance.service.d/theme.conf'),
        ('normal-session-theme.conf', 'usr/lib/systemd/system/pokelinux-omarchy-shell.service.d/theme.conf'),
    ]:
        plan[target] = (deployment.regular(packaging / source, deployment.MAX_FILE)[0], 0o644)
    rows = {name: {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest(), 'mode': mode}
            for name, (body, mode) in plan.items()}
    deployment.no_links(root)
    deployment.no_links(manifest)
    # Authenticate the complete selection before constructing private staging.
    with tempfile.TemporaryDirectory(prefix='pokelinux-theme-stage-') as temporary:
        private = Path(temporary)
        selected = private / 'source'
        selected.mkdir()
        for name, (body, mode) in plan.items():
            target = selected / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(body)
            target.chmod(mode)
        document = private / 'manifest.json'
        document.write_text(json.dumps({'schema': 1, 'files': rows}, indent=2) + '\n')
        root.mkdir(parents=True, exist_ok=True)
        deployment.install(selected, document, root)
        manifest.write_bytes(document.read_bytes())
    return rows

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['archive', 'provisioner', 'root', 'manifest']:
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    stage(*(deployment.absolute(getattr(args, name)) for name in ['archive', 'provisioner', 'root', 'manifest']))

if __name__ == '__main__':
    main()
