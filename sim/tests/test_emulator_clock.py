import importlib.util
from pathlib import Path
import unittest

SPEC=importlib.util.spec_from_file_location('emulator_clock',Path(__file__).resolve().parents[2]/'framework/tests/emulator_clock.py')
class EmulatorClockTests(unittest.TestCase):
    def test_waits_for_device_time_without_changing_it(self):
        module=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(module)
        values=iter([98,99,100]);now=[0.0]
        def sleep(seconds):now[0]+=seconds
        result=module.wait_for_time(lambda:next(values),100,clock=lambda:now[0],sleep=sleep)
        self.assertEqual(result,[98,99,100])
        self.assertGreater(now[0],0)
    def test_clock_that_does_not_catch_up_fails(self):
        module=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(module)
        now=[0.0]
        def sleep(seconds):now[0]+=seconds
        with self.assertRaises(TimeoutError):
            module.wait_for_time(lambda:0,100,timeout=1,clock=lambda:now[0],sleep=sleep)
