#!/usr/bin/env python3
"""Public pinned toolchain only; ignored build artifacts, no vendor input."""
from pathlib import Path
import hashlib
import base64
import json
import stat
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'artifacts/camera-boundary'
URL = 'https://dl.google.com/android/repository/android-ndk-r30-linux.zip'
SHA1 = '5107f898313790e449e87eee2183d9a20602dee9'
SHA256 = '753611f410d002cfcd3f3dc2ef49aad532089d3180b436c060a90bf0fcb64df2'
OUT.mkdir(parents=True, exist_ok=True)
archive = OUT / URL.rsplit('/', 1)[1]
if not archive.is_file():
    partial = archive.with_suffix('.partial')
    with urllib.request.urlopen(URL, timeout=60) as response, partial.open('wb') as file:
        while chunk := response.read(1024 * 1024):
            file.write(chunk)
    partial.rename(archive)
data = archive.read_bytes()
assert len(data) == 738633529 and hashlib.sha1(data).hexdigest() == SHA1 and hashlib.sha256(data).hexdigest() == SHA256, 'NDK archive identity mismatch'
if not (OUT / 'android-ndk-r30/source.properties').is_file():
    with zipfile.ZipFile(archive) as package:
        package.extractall(OUT)
# Python zip extraction does not preserve symlinks or execute bits.
with zipfile.ZipFile(archive) as package:
    for entry in package.infolist():
        mode = entry.external_attr >> 16
        path = OUT / entry.filename
        if stat.S_ISLNK(mode):
            if path.is_symlink() or path.exists():
                path.unlink()
            path.symlink_to(package.read(entry).decode())
        elif mode & 0o111:
            path.chmod(mode & 0o777)
(OUT / 'toolchain.json').write_text(json.dumps({
    'url': URL, 'version': '30.0.16248370', 'size': len(data),
    'sha1': SHA1, 'sha256': hashlib.sha256(data).hexdigest(),
    'source': 'https://developer.android.com/ndk/downloads',
    'scope': 'original public toolchain; synthetic Android test infrastructure only'
}, indent=2) + '\n')
print('Pinned NDK r30 ready', flush=True)

headers = OUT / 'public-headers/android'
headers.mkdir(parents=True, exist_ok=True)
base = 'https://android.googlesource.com/platform/frameworks/native/+/refs/tags/android-17.0.0_r1/libs/binder/ndk/include_cpp/android/'
pins = {
    "binder_api_level_utils.h": "a6e8c5357e08e06ccbaff882d7f9c324868322a653ac4af2be7c9feed3101f73",
    "binder_auto_utils.h": "91a7587d9010e2bac7c6d347d78463fa15a193c025eae260c91b17e76ef9631d",
    "binder_enums.h": "a4f3b34f5478d5ad820e93eabfa0d5fe82e53a71727ad6f48b5932a043060189",
    "binder_interface_utils.h": "6a3437abc3edd1dbec4cc78d3e3360ee457b4bcca26776f13b0e33ad6c9ad666",
    "binder_internal_logging.h": "b3dc27d60e3c75629f231f94ab24779af9faeede2c4d605401179265d6d5eea8",
    "binder_parcel_utils.h": "159b2f59a84fcf988bd4dc3ced2b58b55de9f8e49681ab96722045ae61f8ec3c",
    "binder_parcelable_utils.h": "9c330de3ef09843d69d54116616efb52e7f442fb3bca28dc7d22f26c8f0fb869",
    "binder_to_string.h": "23eb9663863013f862ebe8cee5d1ee9ba48e89998b8795a36f0772c1f0d91048"
}
identities = {}
for name in pins:
    path = headers / name
    url = base + name + '?format=TEXT'
    if not path.is_file():
        with urllib.request.urlopen(url, timeout=45) as response:
            path.write_bytes(base64.b64decode(response.read()))
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    assert digest == pins[name], 'Public Binder header identity mismatch: ' + name
    identities[name] = {'url': url, 'sha256': digest}
(OUT / 'public-headers.json').write_text(json.dumps(identities, indent=2) + '\n')
print('Public Android17 Binder C++ helper headers ready', flush=True)
