#!/usr/bin/env python3
"""Verify real ARM64 QML module loading without claiming display/GPU proof."""
import json,os,platform,subprocess,traceback
from pathlib import Path
out=Path('/results')
result={'passed':False,'architecture':platform.machine(),'hardware_verified':False,'graphics':'Qt offscreen/software'}
try:
    result['packages']=subprocess.check_output(['pacman','-Q','quickshell','qt6-base','qt6-declarative','qt6-wayland'],text=True)
    runtime=Path('/tmp/phone-qml-runtime');runtime.mkdir(mode=0o700);os.chown(runtime,1000,1000)
    qml=Path('/tmp/phone-module-check.qml')
    modules=['QtQuick','QtQuick.Controls','QtQuick.Layouts','QtQuick.Shapes','QtQuick.Effects','Quickshell','Quickshell.Io','Quickshell.Hyprland','Quickshell.Wayland','Quickshell.Networking','Quickshell.Services.Notifications','Quickshell.Services.Pipewire','Quickshell.Services.Polkit']
    qml.write_text('\n'.join('import '+module for module in modules)+'\nShellRoot { Timer { interval: 100; running: true; onTriggered: { console.log("PHONE_QML_MODULES_READY"); Qt.quit(); } } }\n')
    command=['runuser','-u','alarm','--','env','XDG_RUNTIME_DIR='+str(runtime),'QT_QPA_PLATFORM=offscreen','QT_QUICK_BACKEND=software','dbus-run-session','--','quickshell','-p',str(qml)]
    run=subprocess.run(command,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=60)
    (out/'qml-output.txt').write_text(run.stdout)
    result.update(returncode=run.returncode,modules=modules)
    assert run.returncode==0 and 'PHONE_QML_MODULES_READY' in run.stdout,run.stdout[-5000:]
    result['passed']=True
except subprocess.TimeoutExpired as error:
    output=error.stdout or b''
    (out/'qml-output.txt').write_text(output.decode(errors='replace') if isinstance(output,bytes) else output)
    result['error']=traceback.format_exc()
except Exception:
    result['error']=traceback.format_exc()
finally:
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
