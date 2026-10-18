"""Execute the examples-only Make install consumer in a disposable root."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest

NOW = lambda: time.clock_gettime_ns(time.CLOCK_BOOTTIME)
SHA = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()


class Install(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="settings-install-", dir="/run")
        self.root = Path(self.temp.name)
        self.source = Path(os.environ["NATIVE_SETTINGS_SOURCE"])
        self.binary = Path(os.environ["NATIVE_SETTINGS_LAUNCH_BIN"])

    def tearDown(self):
        self.temp.cleanup()

    def install(self, settings):
        until = NOW() + 5_000_000_000
        argv = ["make", "-C", str(self.source / "native-session-owner"), "install",
                "DESTDIR=" + str(self.root / "stage"),
                "SESSION_OWNER_BIN=" + os.environ["NATIVE_SESSION_OWNER_BIN"],
                "SOUNDS_LAUNCH_BIN=" + os.environ["NATIVE_SOUNDS_LAUNCH_BIN"],
                "MESSAGES_LAUNCH_BIN=" + os.environ["NATIVE_MESSAGES_LAUNCH_BIN"],
                "SETTINGS_LAUNCH_BIN=" + settings]
        child = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                 stderr=subprocess.PIPE, env={"PATH": os.environ["PATH"]})
        try:
            stdout, stderr = child.communicate(timeout=max(.001, (until-NOW())/1e9))
        except BaseException:
            child.kill()
            child.communicate(timeout=1)
            raise
        self.assertLess(NOW(), until)
        self.assertLess(len(stdout) + len(stderr), 65536)
        print(json.dumps({"actual_make_wait": child.returncode, "both_PIPE_EOF": True,
                          "original_until_ns": until, "completed_ns": NOW()}), flush=True)
        return child.returncode

    def test_settings_input_is_required(self):
        self.assertNotEqual(self.install(""), 0, "SETTINGS_LAUNCH_BIN must be explicit")

    def test_nonexecutable_input_is_refused(self):
        inert = self.root / "inert"
        inert.write_bytes(self.binary.read_bytes())
        inert.chmod(0o644)
        self.assertNotEqual(self.install(str(inert)), 0)

    def test_installed_exact_launcher_and_original_desktop(self):
        self.assertEqual(self.install(str(self.binary)), 0)
        stage = self.root / "stage"
        for source, target, mode in [
            (self.binary, stage / "usr/libexec/pokelinux/settings-launch", 0o755),
            (self.source / "native-settings-ui/org.omarchy.Settings.desktop",
             stage / "usr/share/applications/org.omarchy.Settings.desktop", 0o644),
        ]:
            self.assertTrue(target.is_file(), str(target))
            self.assertEqual(SHA(source), SHA(target))
            self.assertEqual(target.stat().st_mode & 0o777, mode)
            self.assertEqual((target.stat().st_uid, target.stat().st_gid), (0, 0))
        self.assertEqual(len([p for p in stage.rglob("*") if p.is_file()]), 10)


if __name__ == "__main__":
    unittest.main(verbosity=2)
