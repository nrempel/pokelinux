"""Exercise the QML subprocess boundary through a real Unix socket."""
import json
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import threading
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from phone_ipc import receive, send

CLIENT = Path(__file__).resolve().parents[1] / 'phone_local.py'

class LocalCliTests(unittest.TestCase):
    def exchange(self, operation, reply=True):
        with tempfile.TemporaryDirectory(prefix='pc-', dir='/tmp') as directory:
            path = str(Path(directory) / 's')
            requests = []
            errors = []
            with socket.socket(socket.AF_UNIX) as server:
                server.bind(path)
                server.listen(2)
                server.settimeout(1)
                def serve():
                    try:
                        with server.accept()[0] as peer:
                            requests.append(receive(peer))
                            if reply:
                                send(peer, dict(v=1,id='a'*32,epoch='b'*32,
                                    operation={'state':'finished','result':{'outcome':'submitted','reason':'accepted'}}
                                    if operation == 'control' else None))
                    except Exception as error: errors.append(error)
                worker = threading.Thread(target=serve)
                worker.start()
                command = [sys.executable,str(CLIENT),'--socket',path,operation,
                           '--epoch','b'*32,'--id','a'*32]
                if operation == 'control':
                    command += ['--session','d'*32,'--generation','c'*32,'--call-id','7','--revision','3',
                                '--state','2','--action','answer']
                process = subprocess.run(command,capture_output=True,text=True,timeout=5)
                worker.join(2)
                self.assertFalse(worker.is_alive())
                self.assertFalse(errors, (process.stderr, errors))
                self.assertEqual(len(requests),1)
                # A reconnect/retry would be queued here even if the worker finished.
                server.settimeout(.05)
                with self.assertRaises(TimeoutError): server.accept()
                return process, requests[0]
    def test_control_preserves_original_identity_and_revision(self):
        process, request = self.exchange('control')
        self.assertEqual(process.returncode,0,process.stderr)
        self.assertEqual(request,dict(v=1,id='a'*32,epoch='b'*32,session='d'*32,op='control',command=dict(
            generation='c'*32,call_id=7,expected_revision=3,expected_state=2,action='answer')))
        self.assertEqual(json.loads(process.stdout)['operation']['state'],'finished')
    def test_result_is_read_only_and_absence_is_explicit(self):
        process, request = self.exchange('result')
        self.assertEqual(process.returncode,0,process.stderr)
        self.assertEqual(request,dict(v=1,id='a'*32,epoch='b'*32,op='result'))
        self.assertIsNone(json.loads(process.stdout)['operation'])
    def test_lost_reply_does_not_retry_or_report_rejection(self):
        process, request = self.exchange('control',False)
        self.assertEqual(process.returncode,1)
        self.assertEqual(json.loads(process.stdout),dict(available=False,id='a'*32,epoch='b'*32))
    def test_invalid_command_rejected_before_connect(self):
        process = subprocess.run([sys.executable,str(CLIENT),'--socket','/does/not/exist','control',
            '--epoch','b'*32,'--id','a'*32,'--generation','c'*32,'--call-id','7',
            '--revision','3','--state','2','--action','shell'],capture_output=True,text=True)
        self.assertEqual(process.returncode,2)
        self.assertEqual(process.stdout,'')
