#!/usr/bin/env python3
"""Owned filesystem/stager control; authored ELF bytes are never executed."""
import argparse
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import stat
import sys

sys.dont_write_bytecode = True
REPO = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('profile_recipe', REPO / 'tools/native-installed-services/stage_graphical.py')
recipe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(recipe)
PROVIDERS = ['usr/bin/quickshell', 'usr/bin/seatd',
    'usr/libexec/pokelinux-fixture/pixel10-output-discovery',
    'usr/libexec/pokelinux-fixture/pixel10-physical-size-adapter.so',
    'usr/libexec/pokelinux-fixture/profile-ready-bootstrap.so',
    'usr/libexec/pokelinux-fixture/settings-layout-metrics']
TOOLS = ['usr/bin/' + n for n in ['python3', 'systemctl', 'hostapd', 'hostapd_cli', 'iw', 'ip',
    'nsenter', 'ldd', 'modprobe', 'modinfo', 'udevadm', 'dmesg', 'cat', 'busctl']]
PROFILE_FILES = ['etc/pokelinux/connectivity.json', 'etc/pokelinux/two-boot-hostapd.conf',
    'usr/lib/systemd/system/pokelinux-hwsim-provision.service',
    'usr/libexec/pokelinux-test/hwsim/owner.py', *[
    'usr/libexec/pokelinux-test/hwsim/source/' + n for n in
    ['consumer.py', 'namespace_peer.py', 'owned.py', 'wifi_peer.py']]]
GENERATED = 'etc/pokelinux/hwsim-provision.json'

def pin(body, mode=None):
    row = {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()}
    if mode is not None:
        row['mode'] = mode
    return row

def write(path, body, mode=0o644):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(body)
    path.chmod(mode)
    return pin(body, mode)

def dump(path, value):
    return write(path, (json.dumps(value, indent=2) + '\n').encode())

def elf(label):
    return b'\x7fELF\x02\x01' + bytes(12) + (183).to_bytes(2, 'little') + label.encode()

def fixture(root):
    root.mkdir()
    profile, base, selected = root / 'profile', root / 'base', root / 'inputs'
    profile.mkdir(); base.mkdir(); selected.mkdir()
    services = {n: (elf(n), 0o755) for n, _ in recipe.ROLES.values()}
    for n in PROFILE_FILES:
        services[n] = ((b'{"authored":true}\n' if n.endswith('.json') else (n + '\n').encode()), 0o600 if n.startswith('etc/') else 0o644)
    while len(services) < 77:
        n = 'usr/share/authored-service/' + str(len(services))
        services[n] = (n.encode(), 0o644)
    graphical = {'usr/share/authored-graphical/' + str(i): (str(i).encode(), 0o644) for i in range(241)}
    rows = {}
    for label, plan in [('graphical', graphical), ('services', services)]:
        source = root / (label + '-input')
        manifest = recipe.inputs(source, plan)
        recipe.deployment.install(source, manifest, profile / label)
        (profile / (label + '.json')).write_bytes(manifest.read_bytes())
        rows.update(json.loads(manifest.read_text())['files'])
    hyprland = write(base / 'usr/bin/Hyprland', elf('base-hyprland'), 0o755)
    rows['usr/bin/Hyprland'] = hyprland
    dump(profile / 'selected-profile.json', {'schema': 1, 'files': rows})
    composition = {'schema': 1, 'profile': 'generic-arm-graphical-hwsim', 'files': 319,
        'layers': {'graphical': {'files': 241}, 'services': {'files': 77}},
        'base_required': {'usr/bin/Hyprland': hyprland},
        'artifact_roles': {n: {'declared_origin': {'source_reference': 'authored unexecuted bytes'}} for n in recipe.ROLES},
        'generated_or_retained_guest_rows': [GENERATED, *PROVIDERS]}
    dump(profile / 'composition.json', composition)
    providers = {}
    for n in PROVIDERS:
        providers[n] = dict(write(base / n, elf(n), 0o755),
            provenance={'source_reference': 'authored filesystem fixture; not compiled'})
    tools = {}
    for n in TOOLS:
        canonical = 'usr/bin/python3.14' if n.endswith('/python3') else n
        info = write(base / canonical, elf(canonical), 0o755)
        links = {}
        if canonical != n:
            (base / n).symlink_to('python3.14')
            links[n] = 'python3.14'
        tools[n] = dict(info, resolved_path='/' + canonical, links=links)
    template = {'schema': 1, 'guest_work_seconds': 660, 'guest_close_seconds': 700,
        'installed_files': {n: {k: v for k, v in rows[n].items() if k != 'mode'} for n in PROFILE_FILES},
        'protected_config_pins': {'/' + n: {k: v for k, v in rows[n].items() if k != 'mode'} for n in PROFILE_FILES[:2]},
        'kernel_release': 'authored-kernel', 'kernel_config': pin(b'authored config'),
        'module_files': {'authored.ko': pin(b'authored module')},
        'module_vermagic': 'authored; not loaded', 'regdb_files': {}, 'wireless_policy_files': {},
        'exclusive_fixture_contract': {'actor_grant_Connect_or_production_process_launch_admitted': False},
        'adopted_terminal_closure_policy': {'max_terminal_waits': 512},
        'conflicting_service_units': ['authored-conflict.service'],
        'original_hostapd_version_source_reference': {'source_reference': 'authored metadata only'}}
    template['installed_files'].update({n: {'bytes': None, 'sha256': None, 'resolved_path': None} for n in TOOLS})
    template_path = selected / 'hwsim-template.json'
    dump(template_path, template)
    index = {'schema': 1, 'profile': {n: pin((profile / n).read_bytes()) for n in
        ['selected-profile.json', 'composition.json', 'graphical.json', 'services.json']},
        'providers': providers, 'hwsim_template': dict(pin(template_path.read_bytes()), path=template_path.name), 'tools': tools}
    index_path = selected / 'completion-index.json'
    dump(index_path, index)
    return profile, base, selected, index_path, rows, template

def snapshot(root):
    return {str(p.relative_to(root)): ('link:' + str(p.readlink()) if p.is_symlink() else pin(p.read_bytes(), stat.S_IMODE(p.stat().st_mode)))
        for p in root.rglob('*') if p.is_file() or p.is_symlink()}

def required(files, root):
    missing = sorted(set([GENERATED, *PROVIDERS]) - set(files))
    assert not missing, 'missing seven image-completion rows: ' + ', '.join(missing)
    assert len(files) == 326
    for n in [GENERATED, *PROVIDERS]:
        p = root / n
        assert p.is_file() and pin(p.read_bytes(), stat.S_IMODE(p.stat().st_mode)) == files[n], n

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--baseline', action='store_true')
    a = p.parse_args(); a.output.mkdir(mode=0o700)
    profile, base, inputs, index_path, rows, template = fixture(a.output / 'fixture')
    before = snapshot(a.output / 'fixture')
    record = {'scope': 'Actual owned filesystem/install boundary with authored unexecuted ELF bytes and authored kernel metadata; no actual provider/compiler/loader/kernel/runtime claim.', 'passed': False}
    try:
        if a.baseline:
            required(rows, profile / 'services')
        spec = importlib.util.spec_from_file_location('completion', REPO / 'tools/native-installed-services/complete_graphical.py')
        completion = importlib.util.module_from_spec(spec); spec.loader.exec_module(completion)
        def call(out, index=index_path, expected=None):
            return completion.complete(profile, base, inputs, index,
                expected or hashlib.sha256(index.read_bytes()).hexdigest(), out)
        output = a.output / 'completed'
        value = call(output)
        files = json.loads((output / 'completed-profile.json').read_text())['files']
        required(files, output / 'completion')
        assert {n: files[n] for n in rows} == rows
        actual = json.loads((output / 'completion' / GENERATED).read_text())
        expected = copy.deepcopy(template)
        for n, row in json.loads(index_path.read_text())['tools'].items():
            expected['installed_files'][n] = {k: row[k] for k in ['bytes', 'sha256', 'resolved_path']}
        assert actual == expected
        assert value['runtime_qualified'] is False and value['compiled_source_provenance_revalidated'] is False
        assert snapshot(a.output / 'fixture') == before
        assert not (output / 'var').exists()
        # Same seven-row layer is accepted by the unchanged public installer.
        dest = a.output / 'destdir'
        recipe.deployment.install(output / 'completion', output / 'completion.json', dest)
        required(files, dest)
        cases = {}
        index = json.loads(index_path.read_text())
        mutations = {
            'missing-provider': lambda x: x['providers'].pop(PROVIDERS[0]),
            'extra-provider': lambda x: x['providers'].update({'var/lib/pokelinux/session/grant': x['providers'][PROVIDERS[0]]}),
            'wrong-provider-hash': lambda x: x['providers'][PROVIDERS[0]].update(sha256='0' * 64),
            'wrong-provider-mode': lambda x: x['providers'][PROVIDERS[0]].update(mode=0o777),
            'wrong-template-hash': lambda x: x['hwsim_template'].update(sha256='0' * 64),
            'missing-tool': lambda x: x['tools'].pop(TOOLS[0]),
            'wrong-canonical-tool': lambda x: x['tools'][TOOLS[0]].update(resolved_path='/usr/bin/cat'),
            'wrong-declared-link': lambda x: x['tools'][TOOLS[0]]['links'].update({'usr/bin/python3': '../escape'}),
            'escaped-template': lambda x: x['hwsim_template'].update(path='../hwsim-template.json'),
            'wrong-profile-hash': lambda x: x['profile']['selected-profile.json'].update(sha256='0' * 64),
        }
        for label, mutate in mutations.items():
            changed = copy.deepcopy(index); mutate(changed)
            path = inputs / (label + '.json'); dump(path, changed)
            target = a.output / label
            try:
                call(target, path)
            except (ValueError, OSError) as e:
                assert not target.exists(); cases[label] = type(e).__name__
            else:
                raise AssertionError('accepted ' + label)
        for label, mutate in {
            'wrong-source-pin': lambda x: x['installed_files'][PROFILE_FILES[2]].update(sha256='0' * 64),
            'wrong-protected-profile': lambda x: x['protected_config_pins']['/' + PROFILE_FILES[0]].update(sha256='0' * 64),
            'extra-installed-source': lambda x: x['installed_files'].update({'var/lib/policy': pin(b'bad')}),
        }.items():
            changed = copy.deepcopy(template); mutate(changed)
            path = inputs / (label + '-template.json'); dump(path, changed)
            ci = copy.deepcopy(index); ci['hwsim_template'] = dict(pin(path.read_bytes()), path=path.name)
            ip = inputs / (label + '.json'); dump(ip, ci); target = a.output / label
            try:
                call(target, ip)
            except ValueError as e:
                assert not target.exists(); cases[label] = type(e).__name__
            else:
                raise AssertionError('accepted ' + label)
        # Authenticate changed bytes so these reach the intended header/link checks.
        provider = base / PROVIDERS[0]
        saved = provider.read_bytes()
        bad = bytearray(saved); bad[18:20] = (62).to_bytes(2, 'little')
        write(provider, bytes(bad), 0o755)
        ci = copy.deepcopy(index); ci['providers'][PROVIDERS[0]].update(pin(bytes(bad)))
        ip = inputs / 'wrong-ELF.json'; dump(ip, ci)
        target = a.output / 'wrong-ELF'
        try:
            call(target, ip)
        except ValueError:
            assert not target.exists(); cases['wrong-ELF'] = 'ValueError'
        else:
            raise AssertionError('accepted wrong ELF machine')
        write(provider, saved, 0o755)
        alias = base / TOOLS[0]
        alias.unlink(); outside = a.output / 'outside-base-tool'
        write(outside, elf('outside'), 0o755); alias.symlink_to(outside)
        ci = copy.deepcopy(index)
        ci['tools'][TOOLS[0]].update(pin(outside.read_bytes()),
            resolved_path=str(outside), links={TOOLS[0]: str(outside)})
        ip = inputs / 'host-alias-escape.json'; dump(ip, ci)
        target = a.output / 'host-alias-escape'
        try:
            call(target, ip)
        except (ValueError, OSError):
            assert not target.exists(); cases['host-alias-escape'] = 'refused before output'
        else:
            raise AssertionError('followed alias outside prepared base')
        alias.unlink(); alias.symlink_to('python3.14')
        for label, target, expected_hash in [
                ('wrong-index-hash', a.output / 'wrong-index-hash', '0' * 64),
                ('input-output-overlap', inputs / 'overlap', None),
                ('existing-output', output, None)]:
            before_output = snapshot(output)
            try:
                call(target, expected=expected_hash)
            except ValueError:
                assert snapshot(output) == before_output
                assert target == output or not target.exists()
                cases[label] = 'ValueError'
            else:
                raise AssertionError('accepted ' + label)
        record.update(passed=True, completion_rows=7, accounted_rows=326, original319_rows_unchanged=True,
            all_seven_actual_readbacks_and_installer=True, complete_hwsim_value_equal=True, source_inputs_unchanged=True,
            negative_cases=cases)
    except BaseException as e:
        record['error'] = type(e).__name__ + ': ' + str(e)
        raise
    finally:
        dump(a.output / 'result.json', record)

if __name__ == '__main__':
    main()
