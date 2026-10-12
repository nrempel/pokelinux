import importlib.util, struct, unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('hardware_audit',Path(__file__).parents[1]/'tools/hardware_audit.py')
audit=importlib.util.module_from_spec(spec); spec.loader.exec_module(audit)

class AuditTests(unittest.TestCase):
 def dex(self):
  strings=[b'Landroid/telephony/ims/ImsService;',b'getSystemService']
  data=bytearray(140)
  data[:8]=b'dex\n035\0'
  struct.pack_into('<II',data,56,2,112)
  struct.pack_into('<II',data,64,1,120)
  struct.pack_into('<II',data,88,1,124)
  struct.pack_into('<I',data,120,0)
  struct.pack_into('<HHI',data,124,0,0,1)
  for i,s in enumerate(strings):
   struct.pack_into('<I',data,112+i*4,len(data)); data.extend(bytes([len(s)])+s+b'\0')
  struct.pack_into('<I',data,32,len(data))
  return bytes(data)
 def test_method_references(self):
  result=audit.dex_references(self.dex())
  self.assertEqual(result['methods'],['Landroid/telephony/ims/ImsService;->getSystemService'])
 def test_reject_truncated(self):
  with self.assertRaises(ValueError):audit.dex_references(self.dex()[:-1])
 def test_reject_out_of_bounds(self):
  data=bytearray(self.dex()); struct.pack_into('<I',data,60,999999)
  with self.assertRaises(ValueError):audit.dex_references(data)
 def test_all_candidates_not_false_resolution(self):
  rows=[{'path':'/vendor/lib64/libx.so','needed':[]},{'path':'/system/lib64/libx.so','needed':[]}]
  self.assertEqual(audit.candidates(rows,'libx.so'),['/system/lib64/libx.so','/vendor/lib64/libx.so'])
 def test_graph_preserves_ambiguous_and_missing_edges(self):
  rows=[{'path':'/vendor/bin/radio','needed':['libx.so','missing.so']},{'path':'/vendor/lib64/libx.so','needed':[]},{'path':'/system/lib64/libx.so','needed':[]}]
  result=audit.native_graph(rows,['/vendor/bin/radio'])
  self.assertFalse(result['runtime_resolved'])
  self.assertEqual(len(result['ambiguous']),1)
  self.assertEqual(result['unresolved'],[{'from':'/vendor/bin/radio','needed':'missing.so'}])
  self.assertEqual(len(result['files']),3)
 def test_missing_ims_evidence_fails_closed(self):
  self.assertIn('missing_shannon_ims',audit.check_telephony_evidence({'hardware_verified':False,'files':[]},[]))
 def test_hardware_claim_rejected(self):
  self.assertIn('hardware_claim_not_allowed',audit.check_telephony_evidence({'hardware_verified':True,'files':[]},[]))
 def test_parse_indented_service(self):
  result=audit.init_services('on fs\n    setprop x y\n    service cpboot /vendor/bin/cbd -d\n    user radio\n\non boot\n    start cpboot\n')
  self.assertEqual(result,[{'name':'cpboot','command':'/vendor/bin/cbd -d','options':['user radio']}])
if __name__=='__main__':unittest.main()
