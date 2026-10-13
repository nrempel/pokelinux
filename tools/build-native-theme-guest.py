#!/usr/bin/env python3
"""Build only the own native renderer probe against the actual guest Qt ABI."""
from pathlib import Path
import subprocess
import sys
source=Path('/workspace')
out=Path(sys.argv[1]);out.mkdir(exist_ok=True)
consumer=Path(sys.argv[2]) if len(sys.argv)>2 else source/'native-app'
flags=subprocess.check_output(['pkg-config','--cflags','--libs','Qt6Quick','Qt6QuickControls2'],text=True).split()
subprocess.run(['/usr/lib/qt6/moc',str(consumer/'PhoneThemeFiles.h'),'-o',str(out/'moc_Theme.cpp')],check=True,timeout=60)
subprocess.run(['g++','-std=c++17','-O0','-Wall','-Wextra','-Werror','-fPIC','-I'+str(consumer),str(source/'native-theme/renderer.cpp'),str(consumer/'PhoneThemeFiles.cpp'),str(out/'moc_Theme.cpp'),'-o',str(out/'native-theme-renderer'),*flags],check=True,timeout=360)
