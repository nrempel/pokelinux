import hashlib,json,os,unittest
from pathlib import Path
class StagedRuntime(unittest.TestCase):
 @unittest.skipUnless(os.environ.get('PIXEL_RUNTIME'),'Set PIXEL_RUNTIME for real factory payload checks')
 def test_payload_integrity_and_dependency_edges(self):
  from elftools.elf.elffile import ELFFile
  root=Path(os.environ['PIXEL_RUNTIME']);manifest=json.loads((root/'runtime-manifest.json').read_text())
  files={entry['path']:entry for entry in manifest['files']}
  self.assertEqual(manifest['build'],'CP3A.261005.005')
  self.assertIn('/vendor/lib64/egl/libGLESv1_CM_powervr.so',files)
  for path,entry in files.items():
   p=root/path.lstrip('/');self.assertTrue(p.resolve().is_relative_to(root.resolve()))
   data=p.read_bytes();self.assertEqual(len(data),entry['size']);self.assertEqual(hashlib.sha256(data).hexdigest(),entry['sha256'])
  for path,edges in manifest['needed_edges'].items():
   with (root/path.lstrip('/')).open('rb') as f:
    elf=ELFFile(f);self.assertEqual(elf['e_machine'],'EM_AARCH64');dyn=elf.get_section_by_name('.dynamic')
    needed={t.needed for t in dyn.iter_tags() if t.entry.d_tag=='DT_NEEDED'} if dyn else set()
   self.assertEqual(needed,set(edges))
   for target in edges.values():self.assertIn(target,files)
  self.assertTrue((root/'system/bin/linker64').resolve().is_relative_to(root.resolve()))
