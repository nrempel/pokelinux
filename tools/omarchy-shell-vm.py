#!/usr/bin/env python3
import datetime,json,os,subprocess,sys
from pathlib import Path
from sim import qemu_command
root=Path('/workspace')
mode='phone-lifecycle' if '--phone-lifecycle' in sys.argv else ('phone-plugin' if '--phone-plugin' in sys.argv else 'omarchy-shell')
if '--phone-broken' in sys.argv:mode='phone-broken'
if '--phone-upgrade' in sys.argv:mode='phone-upgrade'
if '--phone-recovery' in sys.argv:mode='phone-recovery'
out=root/'sim/results'/(mode+'-'+datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ'))
out.mkdir();print('QML results:',out,flush=True)
command=qemu_command(root,out)
command[command.index('-append')+1]=command[command.index('-append')+1].replace('pixel.mode=test','pixel.mode='+mode)
command[command.index('-drive')+1]+=',snapshot=on'
(out/'command.json').write_text(json.dumps(command,indent=2)+'\n')
with (out/'serial.log').open('w') as log:
    subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=300,env=dict(os.environ,LIBGL_ALWAYS_SOFTWARE='1',SDL_VIDEODRIVER='x11'))
value=json.loads((out/'result.json').read_text());print(json.dumps(value,indent=2))
assert value['passed'] and value['architecture']=='aarch64' and not value['hardware_verified']
