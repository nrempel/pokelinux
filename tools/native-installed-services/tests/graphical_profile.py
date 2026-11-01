#!/usr/bin/env python3
"""Real historical artifacts and fixed source staging; no builds or runtime admission."""
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


def pin(path):
    data = path.read_bytes()
    return {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest(),
            'mode': stat.S_IMODE(path.stat().st_mode)}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ['reference-image', 'archive', 'output']:
        p.add_argument('--' + name, type=Path, required=True)
    p.add_argument('--sdk-sha256', required=True)
    a = p.parse_args()
    a.output.mkdir(mode=0o700)
    result = {'passed': False, 'scope': 'Actual host file staging only; no ELF, service, build or VM execution.',
              'runtime_candidate': False, 'compiled_cohort_rebuilt_or_attested': False}
    try:
        reference_path = a.reference_image / 'project/current/source/supplied-selected-profile.json'
        reference = json.loads(reference_path.read_text())['files']
        overlay = a.reference_image / 'project/current/overlay'
        assert len(reference) == 319
        artifact_root, base_root = a.output / 'artifacts', a.output / 'base'
        artifact_root.mkdir()
        (base_root / 'usr/bin').mkdir(parents=True)
        witness = 'historical-image17-manifest:' + hashlib.sha256(reference_path.read_bytes()).hexdigest()
        index = {'schema': 1, 'artifacts': {}, 'base_providers': {}, 'parameters': {}, 'abi': {}}
        for role, (target, _) in recipe.ROLES.items():
            data = (overlay / target).read_bytes()
            assert len(data) == reference[target]['bytes'] and hashlib.sha256(data).hexdigest() == reference[target]['sha256']
            path = artifact_root / role
            path.write_bytes(data)
            path.chmod(0o444)
            index['artifacts'][role] = {'path': role, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest(),
                'provenance': {'role': role, 'source_reference': witness,
                               'scope': 'Retained historical ELF identity, not an as-built public source attestation.'}}
        target = 'usr/bin/Hyprland'
        data = (overlay / target).read_bytes()
        assert len(data) == reference[target]['bytes'] and hashlib.sha256(data).hexdigest() == reference[target]['sha256']
        base_hyprland = base_root / target
        base_hyprland.write_bytes(data)
        base_hyprland.chmod(0o755)
        index['base_providers']['hyprland'] = {'path': target, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest(),
            'provenance': {'source_reference': witness, 'scope': 'Owned host base fixture; no guest/base-image qualification.'}}
        index['abi'] = {'hyprland_sha256': hashlib.sha256(data).hexdigest(), 'sdk_sha256': a.sdk_sha256}
        for role in ['session-source', 'focus-observer']:
            index['artifacts'][role]['abi'] = index['abi']
        radio = json.loads((overlay / 'etc/pokelinux/connectivity.json').read_text())
        index['parameters']['ssid'] = bytes(radio['ssid']).decode('ascii')
        index_path = a.output / 'artifact-index.json'
        index_path.write_text(json.dumps(index, indent=2) + '\n')
        before = {n: pin(artifact_root / n) for n in recipe.ROLES}
        before_base = pin(base_hyprland)
        output = a.output / 'staged'
        proof = recipe.stage(index_path, artifact_root, base_root, a.archive, output)
        actual = json.loads((output / 'selected-profile.json').read_text())['files']
        assert set(actual) == set(reference), {'missing': sorted(set(reference) - set(actual)),
                                               'unexpected': sorted(set(actual) - set(reference))}
        policy = 'usr/share/pokelinux/presentation/phone-windows.lua'
        maintained = REPO / 'native-session-owner/packaging/presentation/phone-windows.lua'
        assert actual[policy] == pin(maintained) and actual[policy] != reference[policy]
        delta = sorted(n for n in actual if actual[n] != reference[n])
        assert delta == [policy], delta
        assert b'Sounds|Permissions|Settings' in maintained.read_bytes()
        rows = 0
        for target, expected in actual.items():
            candidates = [output / 'graphical' / target, output / 'services' / target]
            if target == 'usr/bin/Hyprland':
                candidates = [base_root / target]
            found = [f for f in candidates if f.is_file()]
            assert len(found) == 1 and pin(found[0]) == expected, target
            rows += 1
        config = (output / 'graphical/etc/pokelinux/normal-hypr.lua').read_bytes()
        assert b'Sounds|Permissions|Settings' in config
        fragment = (REPO / 'tools/native-omarchy-menu/window-bindings.lua').read_bytes()
        assert config.count(fragment) == 1
        assert proof['layers']['graphical']['files'] == 241 and proof['layers']['services']['files'] == 77
        for name in ['graphical', 'services']:
            recipe.deployment.prepare(output / name, output / (name + '.json'), a.output / ('check-' + name))
        assert set(proof['artifact_roles']) == set(recipe.ROLES)
        assert proof['base_provider_origin'] == index['base_providers']['hyprland']['provenance']
        for role, info in proof['artifact_roles'].items():
            assert info['declared_origin']['role'] == role
            assert info['declared_origin']['source_reference'] == witness
            assert (REPO / info['public_build_definition']).is_file()
        assert before == {n: pin(artifact_root / n) for n in recipe.ROLES} and before_base == pin(base_hyprland)
        controls = {}
        for case in ['missing-role', 'swapped-role', 'plugin-abi', 'declared-abi-pair', 'base-provenance', 'base-identity']:
            changed = copy.deepcopy(index)
            if case == 'missing-role':
                changed['artifacts'].pop('settings-ui')
            elif case == 'swapped-role':
                changed['artifacts']['settings-ui'], changed['artifacts']['permissions-ui'] = changed['artifacts']['permissions-ui'], changed['artifacts']['settings-ui']
            elif case == 'plugin-abi':
                changed['artifacts']['focus-observer']['abi'] = {**index['abi'], 'sdk_sha256': '0' * 64}
            elif case == 'declared-abi-pair':
                changed['abi'] = {**index['abi'], 'sdk_sha256': '0' * 64}
                for role in ['session-source', 'focus-observer']:
                    changed['artifacts'][role]['abi'] = changed['abi']
            elif case == 'base-provenance':
                changed['base_providers']['hyprland'].pop('provenance')
            else:
                changed['base_providers']['hyprland']['sha256'] = '0' * 64
            selected = a.output / (case + '.json')
            selected.write_text(json.dumps(changed) + '\n')
            rejected = a.output / case
            try:
                recipe.stage(selected, artifact_root, base_root, a.archive, rejected)
            except ValueError as error:
                controls[case] = str(error)
                assert not rejected.exists()
            else:
                raise AssertionError('altered index accepted: ' + case)
        result.update(passed=True, selected_rows=rows, exact_historical_rows=rows - 1,
                      intended_maintained_source_delta={policy: {'historical': reference[policy], 'maintained': actual[policy]}},
                      Settings_rule_present_in_config_and_fragment=True, exact_single_close_fragment=True,
                      layers=proof['layers'], generic_limits_unchanged=True, base_provider_verified_not_copied=True,
                      artifact_roles_recorded=len(proof['artifact_roles']), refusal_controls=controls,
                      source_and_base_inputs_unchanged=True, declared_provenance_scope=proof['provenance_scope'])
    except BaseException as error:
        result['error'] = type(error).__name__ + ': ' + str(error)
    finally:
        (a.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result))
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
