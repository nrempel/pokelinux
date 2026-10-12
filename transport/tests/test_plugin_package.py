import importlib.util
import tarfile
import tempfile
import unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
class NativePackageTests(unittest.TestCase):
    def test_native_package_contains_only_runtime_and_matches_sources(self):
        spec=importlib.util.spec_from_file_location('native_build',ROOT/'tools/build-plugin-package.py')
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as temp:
            one=module.build(Path(temp)/'one.tar');two=module.build(Path(temp)/'two.tar')
            self.assertEqual(one.read_bytes(),two.read_bytes())
            with tarfile.open(one) as archive:
                names=archive.getnames()
                for f in ['manifest.json','Panel.qml','Service.qml',*[str(p.relative_to(ROOT/'omarchy-plugin')) for p in (ROOT/'omarchy-plugin/qml').glob('*.qml')]]:
                    self.assertEqual(archive.extractfile('usr/share/omarchy-phone/plugin/'+f).read(),(ROOT/'omarchy-plugin'/f).read_bytes())
                self.assertEqual(archive.extractfile('usr/lib/omarchy-phone/plugin_install.py').read(),(ROOT/'transport/plugin_install.py').read_bytes())
                self.assertEqual(archive.getmember('usr/bin/omarchy-phone-plugin').mode,0o755)
                self.assertFalse(any('test' in n or n.endswith(('.pem','.key','.apk')) for n in names))
                self.assertNotIn('.INSTALL',names)
                info=archive.extractfile('.PKGINFO').read().decode()
                for dep in ('omarchy-phone>=0.5.0','quickshell'):self.assertIn('depend = '+dep+'\n',info)
if __name__=='__main__':unittest.main()
