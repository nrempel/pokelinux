#!/usr/bin/env python3
"""Graphical ARM64 phone-client test guest with end-to-end TLS to the emulator."""
import json,os,subprocess,sys
from pathlib import Path
from sim import qemu_command
project=Path('/workspace');out=project/'android-framework/phone-vm';out.mkdir(exist_ok=True)
command=qemu_command(project,out)
mode='phone-plugin-android' if '--plugin' in sys.argv else 'phone'
command[command.index('-append')+1]=command[command.index('-append')+1].replace('pixel.mode=test','pixel.mode='+mode)
command[command.index('-drive')+1]+=',snapshot=on'
command[command.index('-nic')+1]='user,model=virtio-net-pci'
(out/'command.json').write_text(json.dumps(command,indent=2)+'\n')
with (out/'serial.log').open('w') as log:
    subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,timeout=1200,check=True,
                   env=dict(os.environ,LIBGL_ALWAYS_SOFTWARE='1',SDL_VIDEODRIVER='x11'))
