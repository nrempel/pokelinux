import copy
import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from phone_local import validate_response
from test_control_client import snapshot
from test_dial_client import status


def response():
    return {'v': 1, 'id': 'a' * 32, 'snapshot': {
        'epoch': 'b' * 32, 'sequence': 1,
        'controls': {'fresh': True, 'value': snapshot()},
        'dial_status': {'fresh': True, 'value': status()}}}


class LocalClientTests(unittest.TestCase):
    def test_valid_and_stale_values_are_distinct(self):
        value = response()
        self.assertTrue(validate_response(value, 'a' * 32)['controls']['fresh'])
        value['snapshot']['controls']['fresh'] = False
        self.assertFalse(validate_response(value, 'a' * 32)['controls']['fresh'])
    def test_wrong_response_identity_rejected(self):
        with self.assertRaises(ValueError):
            validate_response(response(), 'c' * 32)
    def test_invalid_epoch_sequence_and_schema_rejected(self):
        for key, invalid in (('epoch', ''), ('sequence', True), ('sequence', -1), ('extra', 1)):
            value = response(); value['snapshot'][key] = invalid
            with self.subTest(key=key, invalid=invalid), self.assertRaises(ValueError):
                validate_response(value, 'a' * 32)
    def test_null_can_only_be_unavailable(self):
        value = response(); value['snapshot']['controls']['value'] = None
        with self.assertRaises(ValueError):
            validate_response(value, 'a' * 32)
        value['snapshot']['controls']['fresh'] = False
        self.assertIsNone(validate_response(value, 'a' * 32)['controls']['value'])
    def test_freshness_must_be_boolean(self):
        value = response(); value['snapshot']['controls']['fresh'] = 1
        with self.assertRaises(ValueError):
            validate_response(value, 'a' * 32)

if __name__ == '__main__':unittest.main()

class LocalOperationTests(unittest.TestCase):
    def test_terminal_pending_unknown_and_absent_are_distinct(self):
        from phone_local import validate_operation
        for operation in (None, {'state':'pending','result':None}, {'state':'unknown','result':None},
                          {'state':'finished','result':{'outcome':'submitted','reason':'accepted'}}):
            value = dict(v=1,id='a'*32,epoch='b'*32,operation=operation)
            self.assertEqual(validate_operation(value, 'a'*32, 'b'*32), operation)
    def test_wrong_identity_and_inconsistent_outcomes_rejected(self):
        from phone_local import validate_operation
        good = dict(v=1,id='a'*32,epoch='b'*32,operation={'state':'unknown','result':None})
        invalid = [dict(good,id='c'*32),dict(good,epoch='c'*32),dict(good,v=True),
                   dict(good,operation={'state':'finished','result':None}),
                   dict(good,operation={'state':'pending','result':{'outcome':'submitted','reason':'accepted'}}),
                   dict(good,operation={'state':'finished','result':{'outcome':'x'*65,'reason':'accepted'}}),
                   dict(good,operation={'state':'unknown','result':None,'extra':True})]
        for value in invalid:
            with self.subTest(value=value), self.assertRaises(ValueError):
                validate_operation(value,'a'*32,'b'*32)
