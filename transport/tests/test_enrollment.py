import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from enroll import issue_bundle

class EnrollmentTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name)
        self.key=self.root/'phone.key'
        subprocess.run(['openssl','genpkey','-algorithm','EC','-pkeyopt','ec_paramgen_curve:P-256','-out',str(self.key)],check=True,capture_output=True)
        public=subprocess.check_output(['openssl','pkey','-in',str(self.key),'-pubout','-outform','DER'])
        import base64
        self.request={'v':1,'request_id':'a'*32,'public_key':base64.b64encode(public).decode()}
        self.request_file=self.root/'request.json'; self.request_file.write_text(json.dumps(self.request))

    def test_bundle_has_public_material_only_and_client_config(self):
        dest=self.root/'enrollment'
        result=issue_bundle(self.request_file,dest,days=2)
        with zipfile.ZipFile(dest/'phone-enrollment.zip') as bundle:
            self.assertEqual(set(bundle.namelist()),{'request-id','server.der','client.der','ca.der'})
            self.assertEqual(bundle.read('request-id'),b'a'*32)
            self.assertEqual(hashlib.sha256(bundle.read('client.der')).hexdigest(),result['client_sha256'])
            self.assertEqual(hashlib.sha256(bundle.read('server.der')).hexdigest(),result['server_sha256'])
        config=json.loads((dest/'client.json').read_text())
        self.assertEqual(config['server_sha256'],result['server_sha256'])
        self.assertEqual(Path(config['private_key']).stat().st_mode&0o777,0o600)
        self.assertEqual(dest.stat().st_mode&0o777,0o700)
        subprocess.run(['openssl','verify','-CAfile',str(dest/'ca.pem'),'-purpose','sslclient',str(dest/'client.pem')],check=True,capture_output=True)
        subprocess.run(['openssl','verify','-CAfile',str(dest/'ca.pem'),'-purpose','sslserver','-verify_hostname','omarchy-android',str(dest/'server.pem')],check=True,capture_output=True)

    def test_never_overwrites_existing_enrollment(self):
        dest=self.root/'existing'; dest.mkdir(); (dest/'keep').write_text('existing')
        with self.assertRaises(FileExistsError): issue_bundle(self.request_file,dest,days=2)
        self.assertEqual((dest/'keep').read_text(),'existing')

    def test_rejects_bad_schema_duplicate_keys_and_unbounded_lifetime(self):
        for data in [dict(self.request,v=True),dict(self.request,request_id='../path'),dict(self.request,public_key='invalid'),dict(self.request,extra=1)]:
            self.request_file.write_text(json.dumps(data))
            with self.assertRaises(ValueError): issue_bundle(self.request_file,self.root/'bad',days=2)
            self.assertFalse((self.root/'bad').exists())
        self.request_file.write_text('{"v":1,"v":1}')
        with self.assertRaises(ValueError): issue_bundle(self.request_file,self.root/'bad',days=2)
        self.request_file.write_text(json.dumps(self.request))
        for days in [0,366,True]:
            with self.assertRaises(ValueError): issue_bundle(self.request_file,self.root/'bad',days=days)

if __name__=='__main__': unittest.main()
