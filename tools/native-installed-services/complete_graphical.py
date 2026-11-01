#!/usr/bin/env python3
"""Account for seven fixed image rows; no build, policy write or activation."""
import argparse
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import posixpath
import re
import stat
import tempfile

spec = importlib.util.spec_from_file_location('completion_recipe', Path(__file__).with_name('stage_graphical.py'))
recipe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(recipe)

deployment = recipe.deployment
PROVIDERS = {'usr/bin/quickshell', 'usr/bin/seatd',
    'usr/libexec/pokelinux-fixture/pixel10-output-discovery',
    'usr/libexec/pokelinux-fixture/pixel10-physical-size-adapter.so',
    'usr/libexec/pokelinux-fixture/profile-ready-bootstrap.so',
    'usr/libexec/pokelinux-fixture/settings-layout-metrics'}
TOOLS = {'usr/bin/' + n for n in ['python3', 'systemctl', 'hostapd', 'hostapd_cli', 'iw', 'ip',
    'nsenter', 'ldd', 'modprobe', 'modinfo', 'udevadm', 'dmesg', 'cat', 'busctl']}
PROFILE_FILES = {'etc/pokelinux/connectivity.json', 'etc/pokelinux/two-boot-hostapd.conf',
    'usr/lib/systemd/system/pokelinux-hwsim-provision.service',
    'usr/libexec/pokelinux-test/hwsim/owner.py', *{
    'usr/libexec/pokelinux-test/hwsim/source/' + n for n in
    ['consumer.py', 'namespace_peer.py', 'owned.py', 'wifi_peer.py']}}
GENERATED = 'etc/pokelinux/hwsim-provision.json'
PROFILE_METADATA = {'selected-profile.json', 'composition.json', 'graphical.json', 'services.json'}
TEMPLATE_KEYS = {'schema', 'guest_work_seconds', 'guest_close_seconds', 'installed_files',
    'protected_config_pins', 'kernel_release', 'kernel_config', 'module_files', 'module_vermagic',
    'regdb_files', 'wireless_policy_files', 'exclusive_fixture_contract',
    'adopted_terminal_closure_policy', 'conflicting_service_units',
    'original_hostapd_version_source_reference'}


def require(value, message):
    if not value:
        raise ValueError(message)


def no_constant(value):
    raise ValueError('nonfinite JSON constant: ' + value)


def document(body):
    value = json.loads(body, object_pairs_hook=deployment.unique, parse_constant=no_constant)
    require(isinstance(value, dict), 'JSON object required')
    return value


def simple_pin(row):
    require(isinstance(row, dict) and type(row.get('bytes')) is int
        and 0 <= row['bytes'] <= deployment.MAX_FILE
        and isinstance(row.get('sha256'), str)
        and re.fullmatch('[0-9a-f]{64}', row['sha256']), 'bounded exact byte/hash pin required')
    return {k: row[k] for k in ['bytes', 'sha256']}


def arm(data):
    require(len(data) >= 20 and data[:6] == b'\x7fELF\x02\x01'
        and int.from_bytes(data[18:20], 'little') == 183, 'ARM64 ELF header required; no execution attested')


def complete(profile, base, inputs, index_path, index_sha256, output):
    """Return one bounded completion layer and a manifest accounting for all326 rows.

    Template kernel/module/policy metadata and provider origins remain declared
    authenticated inputs. This copier does not inspect a live image or reattest
    compiler, package, loader, alias execution or kernel behavior.
    """
    for path in [profile, base, inputs, index_path, output]:
        deployment.absolute(path)
        deployment.no_links(path)
    require(all(p.is_dir() for p in [profile, base, inputs]), 'explicit input directories required')
    require(not output.exists() and output.parent.is_dir(), 'fresh output with existing parent required')
    require(all(output != p and output not in p.parents and p not in output.parents
        for p in [profile, base, inputs, index_path]), 'output must not overlap inputs')
    require(isinstance(index_sha256, str) and re.fullmatch('[0-9a-f]{64}', index_sha256), 'reviewed index SHA256 required')
    observed = []

    def take(path, maximum, expected=None, mode=None):
        data, info = deployment.regular(path, maximum)
        if expected is not None:
            require(simple_pin(expected) == {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}, 'input bytes/hash mismatch: ' + str(path.name))
        if mode is not None:
            require(stat.S_IMODE(info.st_mode) == mode, 'input mode mismatch: ' + str(path.name))
        observed.append((path, deployment.identity(info)))
        return data

    index_body = take(index_path, 2 * 1024 * 1024)
    require(hashlib.sha256(index_body).hexdigest() == index_sha256, 'completion index hash mismatch')
    index = document(index_body)
    require(set(index) == {'schema', 'profile', 'providers', 'hwsim_template', 'tools'}
        and type(index['schema']) is int and index['schema'] == 1, 'fixed completion index schema1 required')
    require(set(index['profile']) == PROFILE_METADATA, 'exact profile metadata pins required')
    metadata = {n: document(take(profile / n, 2 * 1024 * 1024, index['profile'][n])) for n in PROFILE_METADATA}
    selected = metadata['selected-profile.json']
    require(set(selected) == {'schema', 'files'} and type(selected['schema']) is int and selected['schema'] == 1
        and isinstance(selected['files'], dict) and len(selected['files']) == 319, 'original319-row profile required')
    rows = selected['files']
    composition = metadata['composition.json']
    require(composition.get('schema') == 1 and composition.get('profile') == 'generic-arm-graphical-hwsim'
        and composition.get('files') == 319 and set(composition.get('artifact_roles', {})) == set(recipe.ROLES)
        and set(composition.get('generated_or_retained_guest_rows', [])) == PROVIDERS | {GENERATED}, 'fixed composition contract required')
    require(not (set(rows) & (PROVIDERS | {GENERATED})), 'completion rows collide with original profile')
    require(PROFILE_FILES | {target for target, _ in recipe.ROLES.values()} <= set(rows), 'profile roles or hwsim source rows missing')
    all_rows, data_by_name = {}, {}
    for label, count in [('graphical', 241), ('services', 77)]:
        layer = metadata[label + '.json']
        require(set(layer) == {'schema', 'files'} and len(layer['files']) == count
            and composition['layers'][label]['files'] == count, 'original bounded layer shape required')
        plan = deployment.prepare(profile / label, profile / (label + '.json'), output / label)
        require(not (set(all_rows) & set(layer['files'])), 'profile layers overlap')
        all_rows.update(layer['files'])
        for path, data, _, source, identity in plan:
            data_by_name[path.as_posix()] = data
            observed.append((source, identity))
    hyprland = 'usr/bin/Hyprland'
    # The existing stager keeps the large base compositor outside generic layers.
    body = take(base / hyprland, 32 * 1024 * 1024, mode=0o755)
    arm(body)
    hyprland_row = recipe.pin(body, 0o755)
    require(composition.get('base_required') == {hyprland: hyprland_row}, 'base compositor differs from composition')
    all_rows[hyprland] = hyprland_row
    require(all_rows == rows, 'profile selected rows differ from actual bounded layers/base')

    require(set(index['providers']) == PROVIDERS, 'exact six completion providers required')
    plan, origins = {}, {}
    for name, row in index['providers'].items():
        require(set(row) == {'bytes', 'sha256', 'mode', 'provenance'} and row['mode'] == 0o755
            and isinstance(row['provenance'], dict)
            and isinstance(row['provenance'].get('source_reference'), str)
            and 0 < len(row['provenance']['source_reference']) <= 4096, 'explicit provider mode/origin required')
        body = take(base / name, deployment.MAX_FILE, row, 0o755)
        arm(body)
        plan[name] = (body, 0o755)
        origins[name] = row['provenance']

    require(set(index['tools']) == TOOLS, 'exact14 selected tool rows required')
    tools = {}
    for name, row in index['tools'].items():
        require(set(row) == {'bytes', 'sha256', 'mode', 'resolved_path', 'links'}
            and row['mode'] == 0o755 and isinstance(row['resolved_path'], str)
            and row['resolved_path'].startswith('/') and isinstance(row['links'], dict)
            and len(row['links']) <= 8, 'canonical tool pin and explicit bounded alias chain required')
        resolved = recipe.relative(row['resolved_path'][1:]).as_posix()
        current, used = name, set()
        while True:
            path = base / recipe.relative(current)
            deployment.no_links(path.parent)
            if not path.is_symlink():
                break
            require(current in row['links'] and current not in used, 'undeclared or cyclic tool alias')
            info = path.lstat(); actual = str(path.readlink())
            require(actual == row['links'][current] and 0 < len(actual) <= 4096
                and '\x00' not in actual and '\\' not in actual, 'tool alias differs from declared chain')
            observed.append((path, deployment.identity(info)))
            used.add(current)
            require(len(used) <= 8, 'tool alias chain too long')
            current = posixpath.normpath(actual if actual.startswith('/') else '/' + posixpath.dirname(current) + '/' + actual).lstrip('/')
        require(used == set(row['links']) and current == resolved, 'canonical tool path differs from selected alias chain')
        data = take(base / recipe.relative(current), deployment.MAX_FILE, row, 0o755)
        if name == 'usr/bin/busctl':
            arm(data)
        tools[name] = dict(simple_pin(row), resolved_path='/' + current)

    template_row = index['hwsim_template']
    require(set(template_row) == {'path', 'bytes', 'sha256'}, 'explicit template pin required')
    template = document(take(inputs / recipe.relative(template_row['path']), 262144, template_row))
    require(set(template) == TEMPLATE_KEYS and type(template['schema']) is int and template['schema'] == 1
        and template['guest_work_seconds'] == 660 and template['guest_close_seconds'] == 700, 'original hwsim template schema/bounds required')
    require(set(template['installed_files']) == TOOLS | PROFILE_FILES, 'exact22 hwsim source/config/tool slots required')
    for name in PROFILE_FILES:
        require(simple_pin(template['installed_files'][name]) == simple_pin(rows[name]), 'hwsim source/config pin differs from staged profile')
    protected = {'/' + n: simple_pin(rows[n]) for n in ['etc/pokelinux/connectivity.json', 'etc/pokelinux/two-boot-hostapd.conf']}
    require(template['protected_config_pins'] == protected, 'protected profiles differ from staged selection')
    for row in template['installed_files'].values():
        require(isinstance(row, dict), 'hwsim slot object required')
    generated = copy.deepcopy(template)
    generated['installed_files'].update(tools)
    body = (json.dumps(generated, indent=2, sort_keys=True) + '\n').encode()
    require(len(body) <= 262144, 'generated hwsim configuration exceeds original bound')
    plan[GENERATED] = (body, 0o600)
    require(len(plan) == 7 and sum(len(d) for d, _ in plan.values()) <= deployment.MAX_TOTAL, 'bounded seven-row completion layer required')
    for path, expected in observed:
        require(deployment.identity(path.lstat()) == expected, 'input identity changed before output creation')

    with tempfile.TemporaryDirectory(prefix='pokelinux-graphical-completion-') as temporary:
        source = Path(temporary) / 'inputs'
        manifest = recipe.inputs(source, plan)
        deployment.prepare(source, manifest, output / 'completion')
        output.mkdir(mode=0o755)
        deployment.install(source, manifest, output / 'completion')
        (output / 'completion.json').write_bytes(manifest.read_bytes())
    completion_rows = {n: recipe.pin(data, mode) for n, (data, mode) in plan.items()}
    completed = {'schema': 1, 'files': dict(rows, **completion_rows)}
    result = {'schema': 1, 'profile': 'generic-arm-graphical-hwsim', 'files': 326,
        'completion_files': 7, 'completion_bytes': sum(len(d) for d, _ in plan.values()),
        'original319_rows_unchanged': True, 'input_index_sha256': index_sha256,
        'authenticated_profile_metadata': index['profile'], 'hwsim_template': template_row,
        'actual_tool_file_pins': tools, 'declared_provider_origins': origins,
        'source_pins': {p.name: {'bytes': p.stat().st_size, 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()}
            for p in [Path(__file__), Path(recipe.__file__), Path(deployment.__file__)]},
        'template_preserved_except14_tool_rows': True, 'runtime_qualified': False,
        'compiled_source_provenance_revalidated': False, 'kernel_package_or_loader_checks_performed': False,
        'policy_initialization_or_service_activation_performed': False,
        'provenance_scope': 'Actual input bytes/modes/alias paths only; template metadata and build origins are authenticated declarations, not compiler or live-image attestation.'}
    (output / 'completed-profile.json').write_text(json.dumps(completed, indent=2, sort_keys=True) + '\n')
    (output / 'completion-proof.json').write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['profile', 'base-root', 'inputs', 'index', 'index-sha256', 'output']:
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    result = complete(*[deployment.absolute(v) for v in [args.profile, args.base_root, args.inputs, args.index]],
        args.index_sha256, deployment.absolute(args.output))
    print(json.dumps({'files': result['files'], 'completion_files': 7, 'runtime_qualified': False}))


if __name__ == '__main__':
    main()
