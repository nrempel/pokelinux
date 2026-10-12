import json
import socket
import struct
import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from omarchy_bridge import receive, validate_snapshot, ProtocolError

class ProtocolTests(unittest.TestCase):
    def test_coarse_status_without_dialer_role(self):
        value={'v':1,'id':'a','generation':'a'*32,'observed_ms':1,'dialer_role':False,'availability':'unavailable','calls':[], 'source':'android.telecom.InCallService', 'phone_state':{'availability':'observed','state':1,'source':'android.telephony.TelephonyCallback','scope':'default_subscription'}}
        self.assertEqual(validate_snapshot(value,'a')['phone_state']['state'],1)
    def test_detailed_observation_requires_role_and_calls(self):
        value={'v':1,'id':'a','generation':'a'*32,'observed_ms':1,'dialer_role':False,'availability':'observed','calls':[], 'source':'android.telecom.InCallService', 'phone_state':{'availability':'unavailable','state':None,'source':'android.telephony.TelephonyCallback','scope':'default_subscription'}}
        with self.assertRaises(ProtocolError): validate_snapshot(value,'a')
        value['dialer_role']=True
        with self.assertRaises(ProtocolError): validate_snapshot(value,'a')
    def test_oversize_rejected_before_body(self):
        a,b=socket.socketpair()
        with a,b:
            a.sendall(struct.pack('!I', 65537))
            with self.assertRaises(ProtocolError): receive(b)
    def test_truncated_frame(self):
        a,b=socket.socketpair()
        with a,b:
            a.sendall(struct.pack('!I', 12)+b'{}'); a.shutdown(socket.SHUT_WR)
            with self.assertRaises(ProtocolError): receive(b)
    def test_duplicate_keys(self):
        a,b=socket.socketpair()
        with a,b:
            payload=b'{"v":1,"v":2}'
            a.sendall(struct.pack('!I',len(payload))+payload)
            with self.assertRaises(ProtocolError): receive(b)
    def test_response_requires_matching_request(self):
        with self.assertRaises(ProtocolError): validate_snapshot({'v':1,'id':'wrong'},'expected')
    def test_unavailable_cannot_include_calls(self):
        value={'v':1,'id':'a','generation':'a'*32,'observed_ms':1,'dialer_role':False,'availability':'unavailable','calls':[{'id':1,'state':2}],'source':'android.telecom.InCallService','phone_state':{'availability':'unavailable','state':None,'source':'android.telephony.TelephonyCallback','scope':'default_subscription'}}
        with self.assertRaises(ProtocolError): validate_snapshot(value,'a')


class ConfigurationTests(unittest.TestCase):
    def test_dns_transport_host_rejected(self):
        from omarchy_bridge import validate_config
        with self.assertRaises(ValueError):
            validate_config({'host':'unbounded-dns.invalid','port':37179,'server_name':'omarchy-android','server_sha256':'a'*64,'ca':'ca','certificate':'cert','private_key':'key'})
    def test_boolean_port_rejected(self):
        from omarchy_bridge import validate_config
        with self.assertRaises(ValueError):
            validate_config({'host':'127.0.0.1','port':True,'server_name':'omarchy-android','server_sha256':'a'*64,'ca':'ca','certificate':'cert','private_key':'key'})

if __name__=='__main__': unittest.main()
