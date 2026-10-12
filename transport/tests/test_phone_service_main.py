"""Production daemon entrypoint: owned paths, recovery and clean shutdown."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from phone_local import snapshot, operation_result
from phone_ledger import Ledger

@unittest.skipUnless(sys.platform.startswith('linux'), 'Linux peer credentials required')
class ServiceMainTests(unittest.TestCase):
    def test_daemon_owns_ledger_recovers_and_restarts_without_commands(self):
        with tempfile.TemporaryDirectory(prefix='ps-',dir='/tmp') as directory:
            root=Path(directory)
            state=root/'state'
            ledger=Ledger(state)
            old_epoch=ledger.epoch
            ledger.reserve(old_epoch,'a'*32,{'saved':'never replay'})
            ledger.close()
            def start_and_observe():
                process=subprocess.Popen([sys.executable,str(Path(__file__).resolve().parents[1]/'phone_service.py'),
                    '--config',str(root/'absent-pairing.json'),'--runtime',str(root/'runtime'),
                    '--state',str(state)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
                try:
                    deadline=time.monotonic()+4
                    while time.monotonic()<deadline:
                        if process.poll() is not None:
                            self.fail(process.communicate()[1])
                        try:
                            value=snapshot(root/'runtime/phone.sock')
                            break
                        except (OSError,ValueError):time.sleep(.02)
                    else:self.fail('Daemon did not publish its socket')
                    self.assertFalse(value['controls']['fresh'])
                    self.assertIsNone(value['controls']['value'])
                    self.assertEqual(operation_result(root/'runtime/phone.sock',old_epoch,'a'*32),
                                     {'state':'unknown','result':None})
                    recovered=json.loads(subprocess.check_output([sys.executable,
                        str(Path(__file__).resolve().parents[1]/'phone_local.py'),
                        '--socket',str(root/'runtime/phone.sock'),'recovery'],text=True,timeout=4))
                    self.assertEqual(recovered,{'available':True,'recovery':{'epoch':value['epoch'],
                        'more':False,'items':[{'epoch':old_epoch,'id':'a'*32,'state':'unknown','result':None}]}})
                    return value['epoch']
                finally:
                    process.terminate()
                    try:process.communicate(timeout=4)
                    except subprocess.TimeoutExpired:
                        process.kill();process.communicate();self.fail('Daemon did not stop')
                    self.assertEqual(process.returncode,0)
                    self.assertFalse((root/'runtime/phone.sock').exists())
            first=start_and_observe()
            second=start_and_observe()
            self.assertNotEqual(first,old_epoch)
            self.assertNotEqual(first,second)
            self.assertEqual(os.stat(state/'operations.sqlite').st_mode & 0o777,0o600)
