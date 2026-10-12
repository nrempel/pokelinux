#!/usr/bin/env python3
"""Release artifacts contain no legacy file-identity loader or test instrumentation."""
import sys,zipfile
from pathlib import Path
path=Path(sys.argv[1] if len(sys.argv)>1 else 'android-framework/apks/bridge.apk')
with zipfile.ZipFile(path) as apk:
    assert not any(name.startswith('assets/') or name.endswith(('.key','.p12','.jks','.pem')) for name in apk.namelist())
    dex=apk.read('classes.dex')
    for marker in (b'identity.p12',b'trust.p12',b'tls-password',b'client-sha256',b'org/omarchy/provision',b'org/omarchy/enrollmentprobe'):
        assert marker not in dex, 'Test credential path remains: '+marker.decode()
print('Release APK excludes legacy identity loaders and test components')
