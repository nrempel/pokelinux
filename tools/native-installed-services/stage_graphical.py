#!/usr/bin/env python3
"""Stage the fixed generic-ARM graphical+hwsim profile; never build or activate it."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path, PurePosixPath
import re
import stat
import tarfile
import tempfile

REPO = Path(__file__).resolve().parents[2]
PROFILE = Path(__file__).resolve().parent / 'graphical-profile'


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


composer = module('graphical_menu', REPO / 'tools/native-omarchy-menu/compose.py')
deployment = composer.deployment
theme = module('graphical_theme', REPO / 'tools/native-omarchy-theme/stage.py')
namespace = module('graphical_namespace', REPO / 'tools/omarchy_theme_patch.py')
ROLES = {
    'session-owner': ('usr/libexec/pokelinux/pokelinux-session-owner', 'native-session-owner/Cargo.toml'),
    'presentation-environment': ('usr/libexec/pokelinux/presentation-environment', 'native-session-owner/Cargo.toml'),
    'sounds-launch': ('usr/libexec/pokelinux/sounds-launch', 'native-lifecycle/Cargo.toml'),
    'settings-launch': ('usr/libexec/pokelinux/settings-launch', 'native-lifecycle/Cargo.toml'),
    'permissions-launch': ('usr/libexec/pokelinux/permissions-launch', 'native-lifecycle/Cargo.toml'),
    'connectivity-main': ('usr/libexec/pokelinux/pokelinux-connectivity-main', 'native-connectivity-control/Cargo.toml'),
    'connectivity-worker': ('usr/libexec/pokelinux/pokelinux-connectivity-service', 'native-connectivity-control/Cargo.toml'),
    'audio-owner': ('usr/libexec/pokelinux/pokelinux-audio', 'native-audio/Cargo.toml'),
    'audio-backend-ready': ('usr/libexec/pokelinux/pokelinux-audio-backend-ready', 'native-audio/Cargo.toml'),
    'appearance-owner': ('usr/libexec/pokelinux/native-appearance', 'native-appearance/Cargo.toml'),
    'theme-provisioner': ('usr/libexec/pokelinux/omarchy-theme-provision', 'native-appearance/Cargo.toml'),
    'permissions-ui': ('usr/libexec/pokelinux/pokelinux-permissions', 'native-permissions-ui/CMakeLists.txt'),
    'settings-ui': ('usr/libexec/pokelinux/pokelinux-settings', 'native-settings-ui/CMakeLists.txt'),
    'sounds-ui': ('usr/libexec/pokelinux/pokelinux-sounds', 'native-audio-ui/CMakeLists.txt'),
    'session-source': ('usr/lib/pokelinux/session-source.so', 'tools/native-installed-services/graphical-profile/plugins/CMakeLists.txt'),
    'focus-observer': ('usr/lib/pokelinux/audio-focus-observer.so', 'tools/native-installed-services/graphical-profile/plugins/CMakeLists.txt'),
    'native-capture': ('usr/libexec/pokelinux-fixture/pixel10-raw-output-capture', 'tools/native-phone-display-profile/raw-output-capture.cpp'),
}


def pin(data, mode):
    return {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest(), 'mode': mode}


def document(path):
    data, _ = deployment.regular(path, 2 * 1024 * 1024)
    return json.loads(data, object_pairs_hook=deployment.unique)


def relative(name):
    path = PurePosixPath(name)
    if (not name or path.is_absolute() or '..' in path.parts or path.as_posix() != name
            or '\\' in name or '\0' in name or len(path.parts) > 32):
        raise ValueError('canonical relative artifact path required')
    return path


def verify(root, name, expected, maximum):
    data, info = deployment.regular(root / relative(name), maximum)
    if (type(expected.get('bytes')) is not int or len(data) != expected['bytes']
            or hashlib.sha256(data).hexdigest() != expected.get('sha256')):
        raise ValueError('artifact bytes/hash mismatch: ' + name)
    return data, stat.S_IMODE(info.st_mode)


def inputs(root, plan):
    root.mkdir()
    for name, (data, mode) in plan.items():
        relative(name)
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        path.chmod(mode)
    selected = root.with_suffix('.json')
    selected.write_text(json.dumps({'schema': 1, 'files': {
        n: pin(d, m) for n, (d, m) in plan.items()}}, indent=2) + '\n')
    return selected


def stage(index_path, artifact_root, base_root, archive, output):
    for path in [index_path, artifact_root, base_root, archive, output]:
        deployment.absolute(path)
        deployment.no_links(path)
    if output.exists() or not output.parent.is_dir():
        raise ValueError('fresh output and existing parent required')
    for path in [index_path, artifact_root, base_root, archive]:
        if output == path or output in path.parents or path in output.parents:
            raise ValueError('output must not overlap inputs')
    index_bytes, _ = deployment.regular(index_path, 2 * 1024 * 1024)
    index = json.loads(index_bytes, object_pairs_hook=deployment.unique)
    if type(index.get('schema')) is not int or index['schema'] != 1:
        raise ValueError('artifact index schema1 required')
    if set(index.get('artifacts', {})) != set(ROLES):
        raise ValueError('exact fixed artifact role set required')
    parameters = index.get('parameters', {})
    if set(parameters) != {'ssid'} or not isinstance(parameters['ssid'], str) or not re.fullmatch(r'[A-Za-z0-9_-]{1,32}', parameters['ssid']):
        raise ValueError('explicit bounded authored open-AP SSID required')
    base = index.get('base_providers', {})
    if set(base) != {'hyprland'} or base['hyprland'].get('path') != 'usr/bin/Hyprland':
        raise ValueError('explicit Hyprland base provider required')
    base_origin = base['hyprland'].get('provenance', {})
    if not isinstance(base_origin.get('source_reference'), str) or not base_origin['source_reference']:
        raise ValueError('base provider provenance required')
    hyprland, base_mode = verify(base_root, 'usr/bin/Hyprland', base['hyprland'], 32 * 1024 * 1024)
    if base_mode != 0o755 or hyprland[:6] != b'\x7fELF\x02\x01' or int.from_bytes(hyprland[18:20], 'little') != 183:
        raise ValueError('installed ARM64 Hyprland base provider mode0755 required')
    abi = index.get('abi', {})
    if (set(abi) != {'hyprland_sha256', 'sdk_sha256'} or abi['hyprland_sha256'] != base['hyprland']['sha256']
            or not isinstance(abi['sdk_sha256'], str) or not re.fullmatch('[0-9a-f]{64}', abi['sdk_sha256'])):
        raise ValueError('explicit matching compositor/SDK ABI pair required')
    if abi != document(PROFILE / 'plugins/abi.json'):
        raise ValueError('ABI pair differs from the fixed public plugin build definition')
    artifacts, provenance = {}, {}
    for role, (target, definition) in ROLES.items():
        selected = index['artifacts'][role]
        origin = selected.get('provenance', {})
        if origin.get('role') != role or not isinstance(origin.get('source_reference'), str) or not origin['source_reference']:
            raise ValueError('named artifact role provenance required: ' + role)
        data, _ = verify(artifact_root, selected['path'], selected, deployment.MAX_FILE)
        if data[:6] != b'\x7fELF\x02\x01' or int.from_bytes(data[18:20], 'little') != 183:
            raise ValueError('actual matching ARM64 ELF required: ' + role)
        if role in ('session-source', 'focus-observer') and selected.get('abi') != abi:
            raise ValueError('plugin ABI differs from base: ' + role)
        artifacts[target] = (data, 0o755)
        provenance[role] = {'target': target, 'artifact': pin(data, 0o755),
                            'public_build_definition': definition, 'declared_origin': origin}
    source_pins = {}

    def source(path):
        data, info = deployment.regular(REPO / path, deployment.MAX_FILE)
        source_pins[path] = pin(data, stat.S_IMODE(info.st_mode))
        return data

    for _, definition in ROLES.values():
        source(definition)
    for name in ['focus-observer.cpp', 'startup_event_ring.hpp']:
        source('tools/native-installed-services/graphical-profile/plugins/' + name)
    source('tools/native-installed-services/graphical-profile/plugins/abi.json')
    for name in ['tools/native-installed-services/stage_graphical.py', 'tools/omarchy_theme_patch.py',
                 'tools/native-omarchy-menu/stage.py', 'tools/native-omarchy-menu/compose.py',
                 'tools/native-omarchy-menu/window-bindings.lua', 'tools/native-omarchy-theme/stage.py',
                 'tools/native-omarchy-theme/assets.json', 'native-session-owner/packaging/install_deployment.py']:
        source(name)
    with tempfile.TemporaryDirectory(prefix='pokelinux-graphical-profile-') as temporary:
        private = Path(temporary)
        theme_root, theme_manifest = private / 'theme', private / 'theme.json'
        theme.stage(archive, artifact_root / index['artifacts']['theme-provisioner']['path'], theme_root, theme_manifest)
        lock = json.loads(source('dependencies/omarchy-files.lock.json'), object_pairs_hook=deployment.unique)
        original = private / 'original'
        original.mkdir()
        with tarfile.open(archive) as bundle:
            for name, expected in lock['files'].items():
                item = bundle.getmember('omarchy-' + lock['commit'] + '/' + name)
                if not item.isfile() or item.size > deployment.MAX_FILE:
                    raise ValueError('bounded pinned original Omarchy file required')
                data = bundle.extractfile(item).read(deployment.MAX_FILE + 1)
                if hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest() != expected:
                    raise ValueError('original Omarchy Git blob mismatch')
                path = original / relative(name)
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
                path.chmod(int(lock['modes'][name][-3:], 8))
        namespace.apply(original, lock)
        graphical = {'usr/share/pokelinux/omarchy/' + n:
                     ((original / n).read_bytes(), int(lock['modes'][n][-3:], 8)) for n in lock['files']}
        theme_rows = document(theme_manifest)['files']
        for name, info in theme_rows.items():
            if (name.startswith(('usr/share/pokelinux/omarchy/', 'usr/share/fonts/omarchy/', 'usr/share/doc/pokelinux/omarchy-font/'))
                    or name == 'usr/bin/omarchy-shell'):
                incoming = ((theme_root / name).read_bytes(), info['mode'])
                if name in graphical and graphical[name] != incoming:
                    raise ValueError('unexpected original/theme collision')
                graphical[name] = incoming
        graphical['etc/pokelinux/normal-hypr.lua'] = (
            source('tools/native-installed-services/graphical-profile/vm-display.lua').rstrip(b'\n') + b'\n\n'
            + source('native-session-owner/packaging/presentation/normal-presentation.lua') + b'\n'
            + source('native-session-owner/packaging/presentation/phone-windows.lua'), 0o644)
        base_inputs = private / 'graphical-inputs'
        base_manifest = inputs(base_inputs, graphical)
        menu_root, menu_manifest = private / 'menu', private / 'menu.json'
        composer.menu.stage(archive, menu_root, menu_manifest)
        composed, composed_manifest = private / 'composed', private / 'composed.json'
        composer.compose(archive, base_inputs, base_manifest, menu_root, menu_manifest,
                         composed, composed_manifest, replace_original_app_library=True)
        graphical = {n: ((composed / n).read_bytes(), v['mode'])
                     for n, v in document(composed_manifest)['files'].items()}
        services = dict(artifacts)
        public_files = json.loads(source('tools/native-installed-services/graphical-profile/public-files.json'), object_pairs_hook=deployment.unique)
        for target, entry in public_files.items():
            if target in services:
                raise ValueError('public source collides with artifact role')
            services[target] = (source(entry['source']), entry['mode'])
        template_rows = json.loads(source('tools/native-installed-services/graphical-profile/templates.json'), object_pairs_hook=deployment.unique)
        actual_templates = {p.relative_to(PROFILE / 'templates').as_posix()
                            for p in (PROFILE / 'templates').rglob('*') if p.is_file()}
        if set(template_rows) != actual_templates:
            raise ValueError('authored template target set changed')
        for target, entry in template_rows.items():
            data = source('tools/native-installed-services/graphical-profile/templates/' + target)
            if target == 'etc/pokelinux/connectivity.json':
                value = json.loads(data, object_pairs_hook=deployment.unique)
                value['ssid'] = list(parameters['ssid'].encode('ascii'))
                data = (json.dumps(value, indent=2) + '\n').encode()
            elif target == 'etc/pokelinux/two-boot-hostapd.conf':
                if data.count(b'@SSID@') != 1:
                    raise ValueError('exact authored AP parameter marker required')
                data = data.replace(b'@SSID@', parameters['ssid'].encode('ascii'))
            if target in services or target in graphical:
                raise ValueError('authored template collision')
            services[target] = (data, entry['mode'])
        if set(graphical) & set(services):
            raise ValueError('profile layers overlap')
        layers = {'graphical': graphical, 'services': services}
        prepared = {}
        for name, plan in layers.items():
            root = private / (name + '-source')
            manifest = inputs(root, plan)
            deployment.prepare(root, manifest, output / name)
            prepared[name] = (root, manifest)
        selected = {n: pin(d, m) for plan in layers.values() for n, (d, m) in plan.items()}
        if 'usr/bin/Hyprland' in selected:
            raise ValueError('base compositor must not be generically copied')
        selected['usr/bin/Hyprland'] = pin(hyprland, 0o755)
        # All input, collision, byte, path and aggregate bounds precede output creation.
        output.mkdir(mode=0o755)
        for name, (root, manifest) in prepared.items():
            deployment.install(root, manifest, output / name)
            (output / (name + '.json')).write_bytes(manifest.read_bytes())
        (output / 'selected-profile.json').write_text(json.dumps({'schema': 1, 'files': selected}, indent=2) + '\n')
        result = {'schema': 1, 'profile': 'generic-arm-graphical-hwsim', 'files': len(selected),
                  'artifact_index_sha256': hashlib.sha256(index_bytes).hexdigest(),
                  'layers': {n: {'files': len(p), 'bytes': sum(len(d) for d, _ in p.values())} for n, p in layers.items()},
                  'base_required': {'usr/bin/Hyprland': pin(hyprland, 0o755)}, 'abi': abi,
                  'base_provider_origin': base_origin,
                  'artifact_roles': provenance, 'public_source_files': source_pins,
                  'runtime_qualified': False, 'compiled_source_provenance_revalidated': False,
                  'provenance_scope': 'Artifact origin is recorded from the caller index; this file stager does not rebuild or attest compiler receipts.',
                  'service_activation_performed': False,
                  'generated_or_retained_guest_rows': ['etc/pokelinux/hwsim-provision.json', 'usr/bin/quickshell', 'usr/bin/seatd',
                    'usr/libexec/pokelinux-fixture/pixel10-output-discovery', 'usr/libexec/pokelinux-fixture/pixel10-physical-size-adapter.so',
                    'usr/libexec/pokelinux-fixture/profile-ready-bootstrap.so', 'usr/libexec/pokelinux-fixture/settings-layout-metrics']}
        (output / 'composition.json').write_text(json.dumps(result, indent=2) + '\n')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['artifact-index', 'artifact-root', 'base-root', 'omarchy-archive', 'output']:
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    result = stage(*[deployment.absolute(getattr(args, n)) for n in
                     ['artifact_index', 'artifact_root', 'base_root', 'omarchy_archive', 'output']])
    print(json.dumps({'files': result['files'], 'layers': result['layers'], 'runtime_qualified': False}))


if __name__ == '__main__':
    main()
