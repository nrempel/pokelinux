#!/usr/bin/env python3
"""Own production/observer builds against actual ARM guest Qt; shared theme reader unchanged."""
import hashlib,json,subprocess,sys
from pathlib import Path
p=Path(sys.argv[2]) if len(sys.argv)>2 else Path('/workspace');out=Path(sys.argv[1]);out.mkdir();ui=p/'native-messages-ui';theme=p/'native-app'
cflags=subprocess.check_output(['pkg-config','--cflags','Qt6Quick','Qt6QuickControls2'],text=True).split()
libs=subprocess.check_output(['pkg-config','--libs','Qt6Quick','Qt6QuickControls2'],text=True).split()
for name,header in [('Theme',theme/'PhoneThemeFiles.h'),('Messages',ui/'MessagesClient.h')]:subprocess.run(['/usr/lib/qt6/moc',str(header),'-o',str(out/('moc_'+name+'.cpp'))],check=True,timeout=60)
subprocess.run(['/usr/lib/qt6/rcc',str(ui/'resources.qrc'),'-o',str(out/'resources.cpp')],check=True,timeout=60)
base=['g++','-std=c++17','-O0','-Wall','-Wextra','-Werror','-fPIC','-I'+str(ui),'-I'+str(theme),*cflags]
objects=[]
for source in [ui/'MessagesClient.cpp',theme/'PhoneThemeFiles.cpp',out/'moc_Theme.cpp',out/'moc_Messages.cpp',out/'resources.cpp']:
 obj=out/(source.stem+'.o');subprocess.run([*base,'-c',str(source),'-o',str(obj)],check=True,timeout=180);objects.append(str(obj))
for name,extra in [('pokelinux-messages',[]),('pokelinux-messages-observer',['-DMESSAGES_ISOLATED_OBSERVER'])]:
 obj=out/(name+'.o');subprocess.run([*base,*extra,'-c',str(ui/'main.cpp'),'-o',str(obj)],check=True,timeout=180)
 subprocess.run(['g++',str(obj),*objects,'-o',str(out/name),*libs],check=True,timeout=60)
(out/'binary-manifest.json').write_text(json.dumps({name:hashlib.sha256((out/name).read_bytes()).hexdigest()for name in ['pokelinux-messages','pokelinux-messages-observer']},indent=2)+'\n')
