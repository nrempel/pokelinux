#!/usr/bin/env python3
"""Build the native Omarchy plugin add-on without registering or enabling it."""
import argparse
import io
import os
import tarfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]


def build(destination, epoch=0):
    plugin=ROOT/'omarchy-plugin'
    sources=[plugin/'manifest.json',plugin/'Service.qml',plugin/'Panel.qml',*sorted((plugin/'qml').glob('*.qml'))]
    files={'usr/share/omarchy-phone/plugin/'+str(p.relative_to(plugin)):(p.read_bytes(),0o644) for p in sources}
    files['usr/lib/omarchy-phone/plugin_install.py']=((ROOT/'transport/plugin_install.py').read_bytes(),0o644)
    files['usr/bin/omarchy-phone-plugin']=(b'#!/bin/sh\nexec /usr/bin/python3 /usr/lib/omarchy-phone/plugin_install.py "$@"\n',0o755)
    size=sum(len(data) for data,mode in files.values())
    files['.PKGINFO']=(f'''pkgname = omarchy-phone-plugin
pkgbase = omarchy-phone-plugin
pkgver = 0.4.0-1
pkgdesc = Native Omarchy shell Phone plugin
builddate = {epoch}
packager = Local offline build
size = {size}
arch = any
depend = omarchy-phone>=0.5.0
depend = quickshell
'''.encode(),0o644)
    destination=Path(destination);destination.parent.mkdir(parents=True,exist_ok=True)
    temporary=destination.with_suffix(destination.suffix+'.tmp')
    with tarfile.open(temporary,'w',format=tarfile.USTAR_FORMAT) as archive:
        for name,(data,mode) in sorted(files.items()):
            member=tarfile.TarInfo(name);member.size=len(data);member.mode=mode;member.mtime=epoch
            member.uid=member.gid=0;member.uname=member.gname='root'
            archive.addfile(member,io.BytesIO(data))
    temporary.replace(destination)
    return destination


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,default=ROOT/'artifacts/client/omarchy-phone-plugin-0.4.0-1-any.pkg.tar')
    args=parser.parse_args()
    print(build(args.output,int(os.environ.get('SOURCE_DATE_EPOCH','0'))))
