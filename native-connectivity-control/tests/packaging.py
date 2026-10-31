#!/usr/bin/env python3
"""Actual host file staging and unit contracts; never starts a service or worker."""
import configparser
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import stat
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
PACKAGING = ROOT / 'native-connectivity-control/packaging'
UNIT_PATH = 'usr/lib/systemd/system/pokelinux-connectivity.service'
UNIT = PACKAGING / 'pokelinux-connectivity.service'
MANIFEST = PACKAGING / 'installed-unit.manifest.json'


class Packaging(unittest.TestCase):
    def test_direct_main_and_preserved_runtime_admission_contract(self):
        self.assertTrue(UNIT.is_file(), 'installed connectivity unit is missing')
        unit = configparser.ConfigParser(interpolation=None)
        unit.optionxform = str
        unit.read_string(UNIT.read_text())
        service = unit['Service']
        self.assertEqual(shlex.split(service['ExecStart']), [
            '/usr/libexec/pokelinux/pokelinux-connectivity-main',
            '/run/pokelinux-connectivity/control.sock',
            '/etc/pokelinux/connectivity.json',
            '/run/pokelinux-connectivity/admitted',
            '/usr/libexec/pokelinux/pokelinux-connectivity-service',
        ])
        for key, value in {
            'Type': 'exec', 'User': 'root', 'Group': 'root',
            'Restart': 'no', 'KillMode': 'mixed', 'KillSignal': 'SIGTERM',
            'TimeoutStartSec': '5s', 'TimeoutStopSec': '5s', 'SendSIGKILL': 'yes',
            'RuntimeDirectory': 'pokelinux-connectivity',
            'RuntimeDirectoryMode': '0700', 'RuntimeDirectoryPreserve': 'yes',
            'UMask': '0077', 'NoNewPrivileges': 'yes',
        }.items():
            self.assertEqual(service.get(key), value, key)
        self.assertEqual({key for key in service if key.startswith('Exec')}, {'ExecStart'},
                         'no wrapper, marker deletion or second Stop client')
        self.assertNotIn('PrivateNetwork', service,
                         'the worker must use the selected executing namespace')
        self.assertEqual(unit['Install']['WantedBy'], 'multi-user.target')
        self.assertNotRegex(UNIT.read_text().lower(), r'hwsim|hostapd|nsenter|/bin/(sh|bash)')

    def test_manifest_authenticates_only_the_optional_unit_layer(self):
        self.assertTrue(MANIFEST.is_file(), 'installed connectivity deployment rows are missing')
        self.assertTrue(UNIT.is_file(), 'selected unit source is missing')
        document = json.loads(MANIFEST.read_text())
        self.assertEqual(document, {'schema': 1, 'files': {UNIT_PATH: {
            'bytes': UNIT.stat().st_size,
            'sha256': hashlib.sha256(UNIT.read_bytes()).hexdigest(),
            'mode': 0o644,
        }}})

    def test_actual_make_stages_unit_and_preserves_site_and_admission_files(self):
        self.assertTrue(MANIFEST.is_file(), 'installed connectivity deployment rows are missing')
        self.assertTrue(UNIT.is_file(), 'selected unit source is missing')
        with tempfile.TemporaryDirectory(prefix='connectivity-unit-install-') as temporary:
            base = Path(temporary)
            bundle, stage = base / 'bundle', base / 'stage'
            source = bundle / UNIT_PATH
            source.parent.mkdir(parents=True)
            shutil.copyfile(UNIT, source)
            source.chmod(0o644)
            stage.mkdir(mode=0o700)
            retained = {}
            for relative, data in {
                'etc/pokelinux/connectivity.json': b'explicit protected site selection\n',
                'run/pokelinux-connectivity/admitted': b'admission-v1\nunclean-retained\n1\n',
                'var/lib/pokelinux/session/policy.json': b'persisted NetworkControl grant fixture\n',
            }.items():
                path = stage / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
                path.chmod(0o600)
                retained[relative] = (data, path.stat())
            result = subprocess.run([
                'make', '-C', str(ROOT / 'native-session-owner'), 'install-native-services',
                'NATIVE_SERVICES_ROOT=' + str(bundle),
                'NATIVE_SERVICES_MANIFEST=' + str(MANIFEST),
                'DESTDIR=' + str(stage),
            ], capture_output=True, timeout=5, check=False)
            self.assertEqual(result.returncode, 0, result.stderr.decode(errors='replace'))
            self.assertLess(len(result.stdout) + len(result.stderr), 65536)
            target = stage / UNIT_PATH
            self.assertEqual(target.read_bytes(), UNIT.read_bytes())
            self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o644)
            self.assertEqual((target.stat().st_uid, target.stat().st_gid),
                             (os.geteuid(), os.getegid()))
            for relative, (data, before) in retained.items():
                path = stage / relative
                self.assertEqual(path.read_bytes(), data)
                after = path.stat()
                for field in ('st_dev', 'st_ino', 'st_mode', 'st_uid', 'st_gid',
                              'st_size', 'st_mtime_ns', 'st_ctime_ns', 'st_nlink'):
                    self.assertEqual(getattr(after, field), getattr(before, field),
                                     (relative, field))
            self.assertEqual({str(path.relative_to(stage)) for path in stage.rglob('*')
                              if path.is_file()}, {UNIT_PATH, *retained})


if __name__ == '__main__':
    unittest.main(verbosity=2)
