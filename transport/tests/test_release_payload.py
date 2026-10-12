import importlib.util,tempfile,unittest,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('release_payload',ROOT/'tools/verify-release-payload.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
class ReleasePayloadTests(unittest.TestCase):
    def test_signature_metadata_is_excluded_but_executable_changes_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);unsigned=root/'unsigned.apk';tested=root/'tested.apk'
            def write(path,dex,signed=False):
                with zipfile.ZipFile(path,'w') as apk:
                    apk.writestr('AndroidManifest.xml',b'manifest');apk.writestr('classes.dex',dex)
                    if signed:apk.writestr('META-INF/CERT.RSA',b'test signature')
            write(unsigned,b'code');write(tested,b'code',True)
            self.assertEqual(module.verify(unsigned,tested)['status'],'PASS')
            write(unsigned,b'changed')
            with self.assertRaisesRegex(ValueError,'differs'):module.verify(unsigned,tested)
            write(unsigned,b'code',True)
            with self.assertRaisesRegex(ValueError,'signing metadata'):module.verify(unsigned,tested)
    def test_incomplete_payload_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'empty.apk'
            with zipfile.ZipFile(path,'w'):pass
            with self.assertRaisesRegex(ValueError,'Incomplete'):module.verify(path,path)
