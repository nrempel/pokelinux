"""A failed guest probe must reach the host instead of becoming a timeout."""
import importlib.util
import io
from pathlib import Path
import unittest
from contextlib import redirect_stderr

SPEC = importlib.util.spec_from_file_location(
    'guest_response', Path(__file__).resolve().parents[1] / 'guest/guest_response.py')

class GuestResponseTests(unittest.TestCase):
    def test_assertion_is_reported_and_does_not_stop_next_request(self):
        module = importlib.util.module_from_spec(SPEC)
        SPEC.loader.exec_module(module)
        def fail():
            raise AssertionError('Expected Android call state was not observed')
        output = io.StringIO()
        with redirect_stderr(output):
            result = module.run_probe('request-one', fail)
        self.assertEqual(result, {'id': 'request-one', 'outcome': 'FAILED',
                                  'error': 'AssertionError'})
        self.assertIn('Expected Android call state was not observed', output.getvalue())
        self.assertEqual(module.run_probe('request-two', lambda: {'passed': True}),
                         {'id': 'request-two', 'outcome': 'OK', 'value': {'passed': True}})

    def test_transport_unavailability_stays_distinct(self):
        module = importlib.util.module_from_spec(SPEC)
        SPEC.loader.exec_module(module)
        def fail():
            raise OSError('socket unavailable')
        self.assertEqual(module.run_probe('request', fail),
                         {'id': 'request', 'outcome': 'UNAVAILABLE',
                          'error': 'OSError', 'calls': []})
