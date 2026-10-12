import sys,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from omarchy_bridge import canonical_number
class DialNumberTests(unittest.TestCase):
    def test_canonical_numbers_preserve_digits(self):
        for value in ('5550100','+12505550100','123','1'*32):
            self.assertEqual(canonical_number(value),value)
    def test_protocol_rejects_ambiguous_or_executable_dial_strings(self):
        for value in ('0','12','+1','+12','','+','++123','tel:5550100','sip:a@example.test','*21*5550100#',
                      '5550100,123','5550100;123','5550100\n','555 0100','１２３','١٢٣',
                      '1'*33,'+1'*20,None,12,True):
            with self.subTest(value=value),self.assertRaises(ValueError):canonical_number(value)
