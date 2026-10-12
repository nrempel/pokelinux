import sys, unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from runtime import resolve_closure, safe_path

class RuntimeTests(unittest.TestCase):
 def test_vendor_and_system_dependencies_stay_separate(self):
  nodes={'/vendor/lib64/gpu.so':['libbase.so'], '/vendor/lib64/libbase.so':['libc.so'], '/system/bin/servicemanager':['libbase.so'], '/system/lib64/libbase.so':['libc.so'], '/apex/com.android.runtime/lib64/bionic/libc.so':[]}
  files,edges=resolve_closure(nodes,['/vendor/lib64/gpu.so','/system/bin/servicemanager'])
  self.assertEqual(edges['/vendor/lib64/gpu.so']['libbase.so'],'/vendor/lib64/libbase.so')
  self.assertEqual(edges['/system/bin/servicemanager']['libbase.so'],'/system/lib64/libbase.so')
  self.assertEqual(len(files),5)
 def test_missing_dependency_fails_closed(self):
  with self.assertRaisesRegex(ValueError,'missing.so'):resolve_closure({'/vendor/lib64/a.so':['missing.so']},['/vendor/lib64/a.so'])
 def test_cycle_terminates(self):
  files,_=resolve_closure({'/vendor/lib64/a.so':['b.so'],'/vendor/lib64/b.so':['a.so']},['/vendor/lib64/a.so']);self.assertEqual(len(files),2)
 def test_missing_explicit_root_fails(self):
  with self.assertRaises(ValueError):resolve_closure({},['/vendor/lib64/a.so'])
 def test_path_escape_rejected(self):
  for p in ['/../../etc/passwd','relative','/vendor/../etc/passwd']:
   with self.assertRaises(ValueError):safe_path(p)
 def test_valid_absolute_path(self):self.assertEqual(safe_path('/vendor/lib64/a.so'),'vendor/lib64/a.so')
if __name__=='__main__':unittest.main()
