#!/usr/bin/env python3
"""Compare an unsigned delivery APK with the executable payload tested in Android."""
import argparse,hashlib,json,zipfile
from pathlib import Path

def payload(path):
    with zipfile.ZipFile(path) as apk:
        names=apk.namelist()
        if len(names)!=len(set(names)):raise ValueError('Duplicate APK entries')
        return {name:hashlib.sha256(apk.read(name)).hexdigest() for name in names
                if not name.startswith('META-INF/') and not name.endswith('/')}

def verify(unsigned,tested):
    with zipfile.ZipFile(unsigned) as apk:
        if any(name.startswith('META-INF/') for name in apk.namelist()):raise ValueError('Unsigned APK contains signing metadata')
    actual=payload(unsigned);expected=payload(tested)
    if actual!=expected:raise ValueError('Delivery payload differs from tested APK')
    if 'classes.dex' not in actual or 'AndroidManifest.xml' not in actual:raise ValueError('Incomplete APK')
    return {'status':'PASS','unsigned_sha256':hashlib.sha256(Path(unsigned).read_bytes()).hexdigest(),
            'tested_sha256':hashlib.sha256(Path(tested).read_bytes()).hexdigest(),'payload':actual}

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('unsigned',type=Path);parser.add_argument('tested',type=Path)
    args=parser.parse_args();print(json.dumps(verify(args.unsigned,args.tested),indent=2))
if __name__=='__main__':main()
