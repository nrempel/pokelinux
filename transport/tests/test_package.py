import gzip, hashlib, importlib.util, struct, tarfile, tempfile, unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('build_client',ROOT/'tools/build-client-package.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
class PackageTests(unittest.TestCase):
    def binaries(self, directory, machine=183):
        directory.mkdir()
        # Header fixtures exercise packaging checks, not executable validity.
        data=bytearray(64);data[:7]=b'\x7fELF\x02\x01\x01';struct.pack_into('<HHI',data,16,3,machine,1)
        for name in ('omarchy-phoned','omarchy-phone','omarchy-phone-app'):(directory/name).write_bytes(data)
        return directory
    def test_native_runtime_only_and_private_user_service(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);binaries=self.binaries(root/'bin');package=module.build(root/'phone.pkg.tar',binaries,'aarch64',epoch=1)
            with tarfile.open(package) as archive:
                self.assertEqual(set(archive.getnames()),{'.PKGINFO','.MTREE','usr/bin/omarchy-phone','usr/bin/omarchy-phoned','usr/bin/omarchy-phone-app','usr/lib/systemd/user/omarchy-phone.service','usr/share/applications/org.omarchy.Phone.desktop','usr/share/dbus-1/services/org.omarchy.Phone.service'})
                for name in ('omarchy-phone','omarchy-phoned','omarchy-phone-app'):
                    self.assertEqual(archive.extractfile('usr/bin/'+name).read(),(binaries/name).read_bytes())
                    self.assertEqual(archive.getmember('usr/bin/'+name).mode,0o755)
                mtree=gzip.decompress(archive.extractfile('.MTREE').read()).decode()
                self.assertTrue(mtree.startswith('#mtree\n'))
                for member in archive.getmembers():
                    if member.name == '.MTREE': continue
                    digest=hashlib.sha256(archive.extractfile(member).read()).hexdigest()
                    self.assertIn(f'./{member.name} type=file uid=0 gid=0 mode={member.mode:o} time=1.0 size={member.size} sha256digest={digest}\n', mtree)
                info=archive.extractfile('.PKGINFO').read().decode();self.assertIn('arch = aarch64\n',info);self.assertNotIn('python',info)
                for dependency in ('qt6-base>=6.8','qt6-declarative>=6.8','qt6-wayland>=6.8'):
                    self.assertIn('depend = '+dependency+'\n',info)
                desktop=archive.extractfile('usr/share/applications/org.omarchy.Phone.desktop').read().decode()
                self.assertIn('Exec=omarchy-phone-app\n',desktop)
                self.assertIn('Terminal=false\n',desktop)
                activation=archive.extractfile('usr/share/dbus-1/services/org.omarchy.Phone.service').read().decode()
                self.assertEqual(activation, '[D-BUS Service]\nName=org.omarchy.Phone\nExec=/usr/bin/omarchy-phone-app\n')
                unit=archive.extractfile('usr/lib/systemd/user/omarchy-phone.service').read().decode()
                self.assertIn('ExecStart=/usr/bin/omarchy-phoned --enrollment %E/omarchy-phone/enrollment',unit)
                self.assertIn('RuntimeDirectoryMode=0700',unit);self.assertIn('StateDirectoryMode=0700',unit)
                self.assertNotIn('--config',unit)
                self.assertTrue(all(m.uid==0 and m.gid==0 and m.mtime==1 for m in archive.getmembers()))
    def test_wrong_architecture_and_scripts_are_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);binaries=self.binaries(root/'bin',62)
            with self.assertRaises(ValueError):module.build(root/'bad.pkg.tar',binaries,'aarch64')
            (binaries/'omarchy-phoned').write_bytes(b'#!/bin/sh\n')
            with self.assertRaises(ValueError):module.build(root/'bad.pkg.tar',binaries,'x86_64')
            self.assertFalse((root/'bad.pkg.tar').exists())
    def test_app_is_required_and_must_match_architecture(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);binaries=self.binaries(root/'bin')
            app=binaries/'omarchy-phone-app';app.unlink()
            with self.assertRaises(FileNotFoundError):module.build(root/'bad.pkg.tar',binaries,'aarch64')
            data=bytearray(64);data[:7]=b'\x7fELF\x02\x01\x01';struct.pack_into('<HHI',data,16,3,62,1);app.write_bytes(data)
            with self.assertRaises(ValueError):module.build(root/'bad.pkg.tar',binaries,'aarch64')
            self.assertFalse((root/'bad.pkg.tar').exists())
    def test_reproducible_archive(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);binaries=self.binaries(root/'bin')
            a=module.build(root/'a.tar',binaries,'aarch64',epoch=1);b=module.build(root/'b.tar',binaries,'aarch64',epoch=1)
            self.assertEqual(hashlib.sha256(a.read_bytes()).digest(),hashlib.sha256(b.read_bytes()).digest())
if __name__=='__main__':unittest.main()
