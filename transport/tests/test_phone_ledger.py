import concurrent.futures
import json
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from phone_ledger import Ledger, LedgerError


class LedgerTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / 'state'
        self.ledger = Ledger(self.path, limit=2)
        self.addCleanup(lambda: self.ledger.close())
        self.command = {'op': 'dial', 'number': '5550100', 'offer': 'a' * 32}

    def test_duplicate_has_only_one_dispatch_and_conflict_rejects(self):
        with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
            results = list(pool.map(lambda _: self.ledger.reserve(
                self.ledger.epoch, 'b' * 32, self.command), range(8)))
        self.assertEqual(sum(results), 1)
        with self.assertRaises(LedgerError):
            self.ledger.reserve(self.ledger.epoch, 'b' * 32,
                                dict(self.command, number='5550101'))

    def test_restart_retains_unknown_but_cannot_dispatch_old_request(self):
        epoch = self.ledger.epoch
        self.ledger.reserve(epoch, 'b' * 32, self.command)
        self.ledger.close()
        self.ledger = Ledger(self.path, limit=2)
        self.assertNotEqual(epoch, self.ledger.epoch)
        self.assertEqual(self.ledger.result(epoch, 'b' * 32)['state'], 'unknown')
        with self.assertRaises(LedgerError):
            self.ledger.reserve(epoch, 'b' * 32, self.command)

    def test_result_lookup_never_dispatches_and_is_detached(self):
        epoch = self.ledger.epoch
        self.assertIsNone(self.ledger.result(epoch, 'b' * 32))
        self.ledger.reserve(epoch, 'b' * 32, self.command)
        result = {'outcome': 'invocation_attempted', 'reason': 'returned'}
        self.ledger.finish(epoch, 'b' * 32, result)
        result['outcome'] = 'changed'
        saved = self.ledger.result(epoch, 'b' * 32)
        self.assertEqual(saved['result']['outcome'], 'invocation_attempted')
        self.assertFalse(self.ledger.reserve(epoch, 'b' * 32, self.command))

    def test_saturation_preserves_replay_guards(self):
        epoch = self.ledger.epoch
        for letter in ('a', 'b'):
            self.ledger.reserve(epoch, letter * 32, self.command)
        with self.assertRaises(LedgerError):
            self.ledger.reserve(epoch, 'c' * 32, self.command)
        self.assertFalse(self.ledger.reserve(epoch, 'a' * 32, self.command))

    def test_second_owner_rejected_and_raw_number_not_persisted(self):
        with self.assertRaises(LedgerError):
            Ledger(self.path)
        self.ledger.reserve(self.ledger.epoch, 'b' * 32, self.command)
        for path in self.path.iterdir():
            self.assertNotIn(b'5550100', path.read_bytes())

    def test_unknown_result_survives_restart(self):
        epoch = self.ledger.epoch
        self.ledger.reserve(epoch, 'b' * 32, self.command)
        self.ledger.finish(epoch, 'b' * 32, None)
        self.ledger.close()
        self.ledger = Ledger(self.path)
        self.assertEqual(self.ledger.result(epoch, 'b' * 32)['state'], 'unknown')

    def test_abrupt_process_exit_retains_committed_reservation(self):
        self.ledger.close()
        script = '''import json,os,sys
from phone_ledger import Ledger
l=Ledger(sys.argv[1])
l.reserve(l.epoch,'b'*32,{'op':'dial','number':'5550100'})
print(l.epoch,flush=True)
os._exit(0)
'''
        epoch = subprocess.check_output([sys.executable, '-c', script, str(self.path)],
                                        cwd=Path(__file__).resolve().parents[1], text=True).strip()
        self.ledger = Ledger(self.path)
        self.assertEqual(self.ledger.result(epoch, 'b' * 32)['state'], 'unknown')

    def test_failed_write_cannot_grant_dispatch(self):
        self.ledger.db.execute('PRAGMA query_only=ON')
        with self.assertRaises(sqlite3.OperationalError):
            self.ledger.reserve(self.ledger.epoch, 'b' * 32, self.command)
        self.assertIsNone(self.ledger.result(self.ledger.epoch, 'b' * 32))

    def test_terminal_result_cannot_be_overwritten(self):
        epoch = self.ledger.epoch
        self.ledger.reserve(epoch, 'b' * 32, self.command)
        self.ledger.finish(epoch, 'b' * 32, None)
        with self.assertRaises(LedgerError):
            self.ledger.finish(epoch, 'b' * 32, {'outcome': 'submitted', 'reason': 'ok'})
        self.assertEqual(self.ledger.result(epoch, 'b' * 32)['state'], 'unknown')


if __name__ == '__main__':
    unittest.main()
