import importlib.util
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

class PluginInstallTests(unittest.TestCase):
    def load(self):
        spec = importlib.util.spec_from_file_location('plugin_install', ROOT/'transport/plugin_install.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_registration_is_idempotent_and_unregistration_preserves_source(self):
        module = self.load()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp); source = root/'source'; source.mkdir()
            (source/'manifest.json').write_text('{"id":"nbrempel.phone"}')
            target = root/'config/plugins/nbrempel.phone'
            module.register(source, target)
            module.register(source, target)
            self.assertTrue(target.is_symlink())
            self.assertEqual(target.resolve(), source.resolve())
            module.unregister(source, target)
            module.unregister(source, target)
            self.assertFalse(target.is_symlink())
            self.assertTrue((source/'manifest.json').exists())

    def test_existing_directory_or_foreign_link_is_never_replaced_or_removed(self):
        module = self.load()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp); source = root/'source'; source.mkdir()
            (source/'manifest.json').write_text('{"id":"nbrempel.phone"}')
            target = root/'target'; target.mkdir(); (target/'custom').write_text('keep')
            for action in (module.register, module.unregister):
                with self.assertRaises(FileExistsError): action(source, target)
            self.assertEqual((target/'custom').read_text(), 'keep')
            link = root/'link'; link.symlink_to(root/'missing')
            for action in (module.register, module.unregister):
                with self.assertRaises(FileExistsError): action(source, link)
            self.assertTrue(link.is_symlink())

    def test_missing_or_wrong_plugin_source_is_rejected(self):
        module = self.load()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp); source = root/'source'; source.mkdir()
            for body in ('{}', '{"id":"someone.else"}'):
                (source/'manifest.json').write_text(body)
                with self.assertRaises(ValueError): module.register(source,root/'target')
            self.assertFalse((root/'target').exists())

if __name__ == '__main__': unittest.main()
