import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('fetch_upstreams', Path(__file__).resolve().parents[1] / 'tools/fetch_upstreams.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class FetchTests(unittest.TestCase):
    def test_verified_download_is_saved(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'source.tar.gz'
            module.fetch('https://example.test/source', target, hashlib.sha256(b'archive').hexdigest(), read=lambda _: b'archive')
            self.assertEqual(target.read_bytes(), b'archive')

    def test_bad_download_leaves_no_target(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'source.tar.gz'
            with self.assertRaises(ValueError):
                module.fetch('https://example.test/source', target, '0' * 64, read=lambda _: b'bad')
            self.assertFalse(target.exists())

    def test_corrupt_cache_is_rejected_without_network(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'source.tar.gz'
            target.write_bytes(b'corrupt')
            with self.assertRaises(ValueError):
                module.fetch('https://example.test/source', target, '0' * 64, read=lambda _: self.fail('network called'))
            self.assertEqual(target.read_bytes(), b'corrupt')

    def test_git_blob_verification(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'LICENSE'
            data = b'license text'
            digest = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
            module.fetch('https://example.test/LICENSE', target, digest, git_blob=True, read=lambda _: data)
            self.assertEqual(target.read_bytes(), data)
