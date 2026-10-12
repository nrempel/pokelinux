import os, shlex, subprocess, unittest
class ProbeCLI(unittest.TestCase):
 @unittest.skipUnless(os.environ.get('PIXEL_PROBE'), 'Set PIXEL_PROBE to built ARM64 probe')
 def test_explicit_device_is_required(self):
  p=os.environ['PIXEL_PROBE']
  run=shlex.split(os.environ.get('PIXEL_PROBE_EMULATOR',''))
  self.assertEqual(subprocess.run(run+[p],capture_output=True).returncode,2)
  self.assertEqual(subprocess.run(run+[p,'--help'],capture_output=True).returncode,0)
  self.assertEqual(subprocess.run(run+[p,'--unknown'],capture_output=True).returncode,2)
  self.assertEqual(subprocess.run(run+[p,'--drm','/nonexistent-pixel-audit-node'],capture_output=True).returncode,1)
