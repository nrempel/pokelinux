import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('telecom_state', ROOT / 'framework/tests/telecom_state.py')

class TelecomStateTests(unittest.TestCase):
    def test_current_active_call_is_distinct_from_history(self):
        module = importlib.util.module_from_spec(SPEC)
        SPEC.loader.exec_module(module)
        live = (ROOT / 'artifacts/radio-baseline/telecom-0.txt').read_text()
        missed = (ROOT / 'artifacts/radio-baseline/telecom-8.txt').read_text()
        self.assertEqual(module.live_call_states(live), ['ACTIVE'])
        self.assertEqual(module.live_call_states(missed), [])
        self.assertIn('SET_ACTIVE', missed)
        with self.assertRaises(ValueError):
            module.live_call_states('OK\r\n')
