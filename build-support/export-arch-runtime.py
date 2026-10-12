"""Build a test-only relocatable Arch glibc/Python closure for the Android emulator."""
import json,os,re,shutil,subprocess,sys,tarfile
from pathlib import Path
out=Path('/runtime');(out/'usr/bin').mkdir(parents=True);(out/'usr/lib').mkdir()
python=Path(sys.executable).resolve();shutil.copy2(python,out/'usr/bin/python3')
stdlib=Path('/usr/lib')/f'python{sys.version_info.major}.{sys.version_info.minor}'
shutil.copytree(stdlib,out/'usr/lib'/stdlib.name,ignore=shutil.ignore_patterns('__pycache__','site-packages','test','tests'))
objects=[python,*stdlib.glob('lib-dynload/*.so')];dependencies=set()
for obj in objects:
    output=subprocess.check_output(['ldd',str(obj)],text=True)
    dependencies.update(re.findall(r'(/[^\s()]+)',output))
for name in sorted(dependencies):
    path=Path(name);shutil.copy2(path.resolve(),out/'usr/lib'/path.name)
shutil.copy2('/runtime-packages.txt',out/'packages.txt');shutil.copy2('/etc/os-release',out/'os-release')
with tarfile.open('/arch-runtime.tar.gz','w:gz') as archive:archive.add(out,arcname='arch-runtime')
print(json.dumps({'python':sys.version,'libraries':sorted(dependencies)}))
