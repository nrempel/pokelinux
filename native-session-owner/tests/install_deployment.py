#!/usr/bin/env python3
"""Filesystem effects of the actual build installer; no service or GUI execution."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

INSTALLER = Path(__file__).resolve().parents[1] / 'packaging/install_deployment.py'


class Deployment(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.base = Path(self.temporary.name)
        self.root = self.base / 'bundle'
        self.root.mkdir()
        self.dest = self.base / 'dest'
        self.dest.mkdir(mode=0o700)
        self.manifest = self.base / 'manifest.json'
        self.rows = {}
        self.file('etc/selected.conf', b'explicit site input\n', 0o644)
        self.file('usr/bin/selected', b'#!/bin/sh\nexit 17\n', 0o755)

    def tearDown(self):
        self.temporary.cleanup()

    def file(self, relative, data, mode):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        path.chmod(mode)
        self.rows[relative] = {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest(), 'mode': mode}

    def run_install(self, raw=None):
        self.manifest.write_text(raw if raw is not None else json.dumps({'schema': 1, 'files': self.rows}))
        return subprocess.run([sys.executable, str(INSTALLER), '--root', str(self.root),
                               '--manifest', str(self.manifest), '--destdir', str(self.dest)],
                              capture_output=True, timeout=5, check=False)

    def refused_without_writes(self, raw=None):
        self.assertNotEqual(self.run_install(raw).returncode, 0)
        self.assertEqual(list(self.dest.iterdir()), [])

    def test_exact_selected_bytes_modes_empty_file_and_existing_directory_mode(self):
        self.file('etc/empty', b'', 0o600)
        self.assertEqual(self.run_install().returncode, 0)
        self.assertEqual(self.dest.stat().st_mode & 0o777, 0o700)
        for relative, row in self.rows.items():
            self.assertEqual((self.dest / relative).read_bytes(), (self.root / relative).read_bytes())
            self.assertEqual((self.dest / relative).stat().st_mode & 0o777, row['mode'])
        self.assertFalse((self.dest / 'unselected').exists())

    def test_corrupted_last_source_refuses_before_copying_first(self):
        (self.root / 'usr/bin/selected').write_bytes(b'changed')
        self.refused_without_writes()

    def test_escape_cannot_modify_neighbor(self):
        sentinel = self.base / 'outside'
        sentinel.write_bytes(b'retain')
        self.rows['../outside'] = self.rows['etc/selected.conf']
        self.refused_without_writes()
        self.assertEqual(sentinel.read_bytes(), b'retain')

    def test_duplicate_json_keys_are_not_silently_selected(self):
        self.refused_without_writes('{"schema":1,"schema":1,"files":{}}')

    def test_symlink_source_is_not_followed(self):
        source = self.root / 'usr/bin/selected'
        source.unlink()
        source.symlink_to(self.root / 'etc/selected.conf')
        self.refused_without_writes()

    def test_destination_symlink_cannot_redirect_output(self):
        outside = self.base / 'outside'
        outside.mkdir()
        (self.dest / 'etc').symlink_to(outside, target_is_directory=True)
        before = os.readlink(self.dest / 'etc')
        self.assertNotEqual(self.run_install().returncode, 0)
        self.assertEqual(os.readlink(self.dest / 'etc'), before)
        self.assertEqual(list(outside.iterdir()), [])
        self.assertFalse((self.dest / 'usr').exists())

    def test_conflicting_file_directory_selection_refuses_before_writes(self):
        self.rows['etc'] = self.rows['etc/selected.conf']
        self.refused_without_writes()

    def test_privileged_mode_is_refused_before_writes(self):
        self.rows['usr/bin/selected']['mode'] = 0o4755
        self.refused_without_writes()


if __name__ == '__main__':
    unittest.main()
