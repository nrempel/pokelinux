#!/usr/bin/env python3
"""Actual graphical file staging controls; no build, services, GUI or VM."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import stat
import sys
import tarfile

sys.dont_write_bytecode = True
REPO = Path(__file__).resolve().parents[3]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def pin(path):
    data = path.read_bytes()
    return {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest(),
            'mode': stat.S_IMODE(path.stat().st_mode)}


def manifest(path, rows):
    path.write_text(json.dumps({'schema': 1, 'files': rows}, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['archive', 'provisioner', 'normal-config', 'output']:
        parser.add_argument('--' + name, required=True, type=Path)
    parser.add_argument('--baseline', action='store_true',
                        help='exercise the existing manual merge and generic installer')
    args = parser.parse_args()
    args.output.mkdir(mode=0o700)
    deployment = load('deployment_test', REPO / 'native-session-owner/packaging/install_deployment.py')
    theme = load('theme_test', REPO / 'tools/native-omarchy-theme/stage.py')
    menu = load('menu_test', REPO / 'tools/native-omarchy-menu/stage.py')
    composer = None if args.baseline else load('composer_test', REPO / 'tools/native-omarchy-menu/compose.py')
    report = {'passed': False, 'baseline': args.baseline, 'cases': {},
              'scope': 'actual host file staging; no build, services, GUI, ARM execution or VM',
              'inputs': {'archive': pin(args.archive), 'provisioner': pin(args.provisioner),
                         'normal_config': pin(args.normal_config)}}
    try:
        base = args.output / 'base'
        rows = theme.stage(args.archive, args.provisioner, base, args.output / 'theme.json')
        lock = json.loads((REPO / 'dependencies/omarchy-files.lock.json').read_text())
        prefix = 'omarchy-' + lock['commit'] + '/'
        with tarfile.open(args.archive) as archive:
            selected = [p for p in lock['files'] if p.startswith('shell/')]
            selected.append('config/omarchy/shell.json')
            for name in selected:
                member = archive.getmember(prefix + name)
                assert member.isfile() and member.size <= deployment.MAX_FILE
                data = archive.extractfile(member).read(deployment.MAX_FILE + 1)
                blob = b'blob ' + str(len(data)).encode() + b'\0' + data
                assert hashlib.sha1(blob).hexdigest() == lock['files'][name]
                target = 'usr/share/pokelinux/omarchy/' + name
                path = base / target
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
                path.chmod(int(lock['modes'][name][-3:], 8))
                rows[target] = pin(path)
        normal = 'etc/pokelinux/normal-hypr.lua'
        config = args.normal_config.read_bytes()
        fragment_file = REPO / 'tools/native-omarchy-menu/window-bindings.lua'
        fragment = fragment_file.read_bytes()
        assert fragment not in config, 'the supplied actual missing-binding control must lack the fragment'
        assert pin(args.normal_config)['mode'] == pin(fragment_file)['mode'] == 0o644
        closed = config + (b'' if config.endswith(b'\n') else b'\n') + fragment
        (base / normal).parent.mkdir(parents=True, exist_ok=True)
        (base / normal).write_bytes(config)
        (base / normal).chmod(0o644)
        rows[normal] = pin(base / normal)
        report['inputs']['window_bindings'] = pin(fragment_file)
        manifest(args.output / 'base.json', rows)
        menu_root = args.output / 'native-menu'
        menu_rows = menu.stage(args.archive, menu_root, args.output / 'native-menu.json')
        app = 'usr/share/pokelinux/omarchy/shell/services/AppLibrary.qml'
        original = (base / app).read_bytes()
        assert menu_rows[app] != rows[app]
        report['base_files'] = len(rows)
        report['required_menu_rows'] = menu_rows
        cases = ['omitted-three', 'unadapted-app-library', 'unexpected-collision']
        if not args.baseline:
            cases.extend(['changed-manifest-between-reads', 'omitted-normal-config',
                          'duplicate-close-fragment', 'conflicting-literal-close', 'already-closed'])
        for case in [*cases, 'valid']:
            folder = args.output / case
            folder.mkdir(mode=0o700)
            candidate_base, candidate_menu = folder / 'base', folder / 'menu'
            shutil.copytree(base, candidate_base)
            shutil.copytree(menu_root, candidate_menu)
            base_rows, component_rows = dict(rows), dict(menu_rows)
            if case == 'omitted-three':
                for name in menu_rows:
                    (candidate_base / name).unlink()
                    (candidate_menu / name).unlink()
                    base_rows.pop(name)
                component_rows = {}
            elif case == 'unadapted-app-library':
                (candidate_menu / app).write_bytes(original)
                component_rows[app] = pin(candidate_menu / app)
            elif case == 'unexpected-collision':
                wrapper = candidate_base / 'usr/bin/omarchy-shell'
                wrapper.write_text('#!/bin/sh\nexit 0\n')
                base_rows['usr/bin/omarchy-shell'] = pin(wrapper)
            elif case == 'omitted-normal-config':
                (candidate_base / normal).unlink()
                base_rows.pop(normal)
            elif case in ('duplicate-close-fragment', 'conflicting-literal-close', 'already-closed'):
                body = closed
                if case == 'duplicate-close-fragment':
                    body += fragment
                elif case == 'conflicting-literal-close':
                    body = config + b'\nhl.bind("SUPER + W", hl.dsp.window.maximize())\n'
                (candidate_base / normal).write_bytes(body)
                base_rows[normal] = pin(candidate_base / normal)
            base_manifest, menu_manifest = folder / 'base.json', folder / 'menu.json'
            manifest(base_manifest, base_rows)
            manifest(menu_manifest, component_rows)
            dest, output_manifest = folder / 'composed', folder / 'composed.json'
            before = {name: pin(candidate_base / name) for name in base_rows}
            entry = {'desired_refusal': case not in ('valid', 'already-closed'), 'refused': False}
            try:
                if args.baseline:
                    # The documented pre-guard procedure merges selected rows;
                    # the generic installer has no graphical component policy.
                    merged = folder / 'manual-merge'
                    shutil.copytree(candidate_base, merged)
                    for name in component_rows:
                        target = merged / name
                        target.parent.mkdir(parents=True, exist_ok=True)
                        shutil.copyfile(candidate_menu / name, target)
                        target.chmod(component_rows[name]['mode'])
                    merged_rows = {**base_rows, **component_rows}
                    manifest(output_manifest, merged_rows)
                    deployment.install(merged, output_manifest, dest)
                else:
                    if case == 'valid':
                        try:
                            composer.compose(args.archive, candidate_base, base_manifest,
                                             candidate_menu, menu_manifest, dest, output_manifest)
                        except ValueError as error:
                            assert 'explicit original AppLibrary replacement' in str(error)
                        else:
                            raise AssertionError('original AppLibrary replacement did not require acknowledgment')
                        assert not dest.exists() and not output_manifest.exists()
                        entry['replacement_without_ack_refused'] = True
                    prepare = composer.deployment.prepare
                    swaps = []

                    def swap_before_prepare(source_root, selected_manifest, target_root):
                        if source_root == candidate_menu and selected_manifest == menu_manifest:
                            assert not swaps, 'manifest substitution must trigger once'
                            swaps.append(True)
                            (candidate_menu / app).write_bytes(original)
                            changed = {**component_rows, app: pin(candidate_menu / app)}
                            manifest(menu_manifest, changed)
                        return prepare(source_root, selected_manifest, target_root)

                    if case == 'changed-manifest-between-reads':
                        composer.deployment.prepare = swap_before_prepare
                    try:
                        composer.compose(args.archive, candidate_base, base_manifest,
                                         candidate_menu, menu_manifest, dest, output_manifest,
                                         replace_original_app_library=True)
                    finally:
                        composer.deployment.prepare = prepare
                        if case == 'changed-manifest-between-reads':
                            assert swaps == [True], 'manifest substitution did not execute'
                            entry['manifest_substitutions_after_validation'] = len(swaps)
            except ValueError as error:
                entry.update(refused=True, refusal=str(error))
                assert not dest.exists() and not output_manifest.exists(), 'refusal mutated output'
            assert before == {name: pin(candidate_base / name) for name in base_rows}
            entry['selected_base_unchanged'] = True
            if not entry['refused']:
                entry['actual_installed_files'] = len(list(p for p in dest.rglob('*') if p.is_file()))
                entry['installed_menu_rows'] = {
                    name: pin(dest / name) for name in menu_rows if (dest / name).is_file()}
            entry['passed'] = entry['refused'] == entry['desired_refusal']
            if case in ('valid', 'already-closed') and not entry['refused']:
                assert entry['installed_menu_rows'] == menu_rows
                expected_rows = {**base_rows, **menu_rows}
                if not args.baseline:
                    expected_rows[normal] = {'bytes': len(closed), 'sha256': hashlib.sha256(closed).hexdigest(), 'mode': 0o644}
                    body = (dest / normal).read_bytes()
                    entry['exact_single_close_fragment'] = body == closed and body.count(fragment) == 1
                    entry['close_fragment_idempotent'] = case == 'already-closed' and body == (candidate_base / normal).read_bytes()
                    entry['normal_config_row'] = pin(dest / normal)
                    entry['passed'] &= entry['exact_single_close_fragment']
                entry['all_selected_byte_hash_mode_checks_passed'] = all(pin(dest / name) == row for name, row in expected_rows.items())
                entry['passed'] &= entry['all_selected_byte_hash_mode_checks_passed']
            report['cases'][case] = entry
        report['passed'] = all(row['passed'] for row in report['cases'].values())
    except BaseException as error:
        report['error'] = type(error).__name__ + ': ' + str(error)
    finally:
        (args.output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'passed': report['passed'], 'cases': {
        name: row['passed'] for name, row in report['cases'].items()}, 'error': report.get('error')}))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
