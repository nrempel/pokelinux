#!/usr/bin/env python3
"""Compose one graphical bundle with the authenticated native menu overlay.

This graphical-only admission precedes the unchanged generic installer. It
does not solve packages, initialize a home, activate services or qualify a UI.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import stat
import tempfile

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('native_menu_stage', HERE / 'stage.py')
menu = importlib.util.module_from_spec(spec)
spec.loader.exec_module(menu)
deployment = menu.deployment
APP_LIBRARY = 'usr/share/pokelinux/omarchy/shell/services/AppLibrary.qml'
NORMAL_CONFIG = 'etc/pokelinux/normal-hypr.lua'


def compose(archive, base_root, base_manifest, menu_root, menu_manifest,
            root, manifest, *, replace_original_app_library=False):
    inputs = [archive, base_root, base_manifest, menu_root, menu_manifest]
    for path in [*inputs, root, manifest]:
        deployment.absolute(path)
        deployment.no_links(path)
    if root.exists() or manifest.exists():
        raise ValueError('fresh composed root and manifest required')
    if root == manifest or root in manifest.parents or manifest in root.parents:
        raise ValueError('composed root and manifest must be separate')
    for output in [root, manifest]:
        if not output.parent.is_dir():
            raise ValueError('existing output parent directory required')
        if any(output == source or output in source.parents or source in output.parents
               for source in inputs):
            raise ValueError('composition outputs must not overlap inputs')
    with tempfile.TemporaryDirectory(prefix='pokelinux-menu-compose-') as temporary:
        private = Path(temporary)
        expected_root, expected_manifest = private / 'menu', private / 'menu.json'
        expected = menu.stage(archive, expected_root, expected_manifest)
        data, _ = deployment.regular(menu_manifest, 2 * 1024 * 1024)
        component = json.loads(data, object_pairs_hook=deployment.unique)
        if (type(component.get('schema')) is not int or component['schema'] != 1
                or component.get('files') != expected):
            raise ValueError('native menu component must contain the exact three staged rows')
        base = deployment.prepare(base_root, base_manifest, root)
        overlay = deployment.prepare(menu_root, menu_manifest, root)
        prepared = {path.as_posix(): {'bytes': len(body),
                    'sha256': hashlib.sha256(body).hexdigest(), 'mode': mode}
                    for path, body, mode, *_ in overlay}
        if prepared != expected:
            raise ValueError('prepared native menu differs from authenticated three rows')
        combined = {path.as_posix(): (body, mode) for path, body, mode, *_ in base}
        if APP_LIBRARY not in combined:
            raise ValueError('graphical base must contain original or native AppLibrary')
        native = (expected_root / APP_LIBRARY).read_text()
        if native.count(menu.NATIVE_LAUNCH) != 1:
            raise ValueError('exact staged native AppLibrary required')
        original = native.replace(menu.NATIVE_LAUNCH, menu.ORIGINAL_LAUNCH, 1).encode()
        equal, replaced = [], []
        for path, body, mode, *_ in overlay:
            name = path.as_posix()
            incoming = (body, mode)
            if name in combined:
                if combined[name] == incoming:
                    equal.append(name)
                elif (name == APP_LIBRARY and combined[name] == (original, mode)):
                    if not replace_original_app_library:
                        raise ValueError('explicit original AppLibrary replacement required')
                    replaced.append(name)
                else:
                    raise ValueError('unexpected graphical composition collision: ' + name)
            combined[name] = incoming
        if NORMAL_CONFIG not in combined or combined[NORMAL_CONFIG][1] != 0o644:
            raise ValueError('explicit normal compositor config row mode0644 required')
        config, config_mode = combined[NORMAL_CONFIG]
        fragment, fragment_info = deployment.regular(HERE / 'window-bindings.lua', deployment.MAX_FILE)
        fragment_mode = stat.S_IMODE(fragment_info.st_mode)
        if not fragment or fragment_mode != 0o644:
            raise ValueError('nonempty source-owned window bindings mode0644 required')
        copies = config.count(fragment)
        if copies > 1:
            raise ValueError('duplicate standard close fragment')
        # Deliberately a literal-profile check, not an arbitrary Lua parser.
        other = config.replace(fragment, b'')
        if re.search(rb"""hl\.bind\s*\(\s*(['"])SUPER\s*\+\s*W\1""", other):
            raise ValueError('conflicting literal Super+W binding')
        compositor = {'path': NORMAL_CONFIG, 'input': {
            'bytes': len(config), 'sha256': hashlib.sha256(config).hexdigest(), 'mode': config_mode},
            'window_bindings': {'source': 'tools/native-omarchy-menu/window-bindings.lua',
                'bytes': len(fragment), 'sha256': hashlib.sha256(fragment).hexdigest(), 'mode': fragment_mode},
            'fragment_appended': copies == 0}
        if copies == 0:
            config += (b'' if config.endswith(b'\n') else b'\n') + fragment
        combined[NORMAL_CONFIG] = (config, config_mode)
        # Keep the source-path recheck used by the generic installer across both
        # authenticated selections, before constructing any public output.
        for _, _, _, source, identity in [*base, *overlay]:
            if deployment.identity(source.lstat()) != identity:
                raise ValueError('composition source identity changed')
        rows = {name: {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest(),
                       'mode': mode} for name, (body, mode) in combined.items()}
        selected = private / 'selected'
        selected.mkdir()
        for name, (body, mode) in combined.items():
            target = selected / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(body)
            target.chmod(mode)
        document = {'schema': 1, 'files': rows, 'composition': {
            'native_menu': json.loads(expected_manifest.read_text())['source'],
            'equal_collisions': equal, 'replaced_original_app_library': replaced,
            'normal_compositor': compositor}}
        complete = private / 'complete.json'
        complete.write_text(json.dumps(document, indent=2) + '\n')
        # This validates the combined byte/file/path bounds before root.mkdir.
        deployment.prepare(selected, complete, root)
        root.mkdir(mode=0o755)
        deployment.install(selected, complete, root)
        with manifest.open('x') as output:
            output.write(complete.read_text())
    return document


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['archive', 'base-root', 'base-manifest', 'menu-root',
                 'menu-manifest', 'root', 'manifest']:
        parser.add_argument('--' + name, required=True)
    parser.add_argument('--replace-original-app-library', action='store_true')
    args = parser.parse_args()
    paths = [deployment.absolute(getattr(args, name)) for name in
             ['archive', 'base_root', 'base_manifest', 'menu_root', 'menu_manifest', 'root', 'manifest']]
    result = compose(*paths, replace_original_app_library=args.replace_original_app_library)
    print(json.dumps({'files': len(result['files']), 'native_menu_rows': 3,
                      'service_activation_performed': False, 'runtime_qualified': False}))


if __name__ == '__main__':
    main()
