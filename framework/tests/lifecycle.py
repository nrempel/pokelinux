#!/usr/bin/env python3
"""Production enrollment survives opt-in boot; explicit Stop survives reboot."""
import json,re,subprocess,time,sys,xml.etree.ElementTree as ET
from pathlib import Path
sys.path.insert(0,'transport')
from omarchy_bridge import Client
ADB=['android-framework/sdk/platform-tools/adb','-s','emulator-5554']
def adb(*args):return subprocess.check_output(ADB+list(args),text=True,stderr=subprocess.STDOUT,timeout=60)
def click(text):
 adb('shell','uiautomator','dump','/sdcard/window.xml');tree=ET.fromstring(adb('shell','cat','/sdcard/window.xml'))
 nodes=[n for n in tree.iter('node') if n.get('text')==text and n.get('clickable')=='true'];assert len(nodes)==1,(text,[n.get('text') for n in tree.iter('node')])
 x1,y1,x2,y2=map(int,re.findall(r'\d+',nodes[0].get('bounds')));adb('shell','input','tap',str((x1+x2)//2),str((y1+y2)//2));time.sleep(.3)
def reboot():
 adb('reboot');time.sleep(3);adb('wait-for-device');end=time.monotonic()+180
 while time.monotonic()<end:
  if adb('shell','getprop','sys.boot_completed').strip()=='1':break
  time.sleep(2)
 else:raise TimeoutError('Reboot')
 adb('shell','input','keyevent','82');adb('forward','tcp:37179','tcp:37179')
def connected():
 end=time.monotonic()+30
 while time.monotonic()<end:
  try:return client.controls()
  except (OSError,ValueError):time.sleep(.5)
 raise AssertionError('Gateway did not start')
client=Client(json.loads(Path('android-framework/enrollment-client.json').read_text()));checks={}
adb('forward','tcp:37179','tcp:37179')
adb('shell','am','start','-n','org.omarchy.bridge/.DialActivity');time.sleep(1);click('Bridge')
adb('shell','uiautomator','dump','/sdcard/window.xml')
tree=ET.fromstring(adb('shell','cat','/sdcard/window.xml'))
boot=next(n for n in tree.iter('node') if n.get('text')=='Start after reboot and unlock')
if boot.get('checked')!='true':click('Start after reboot and unlock')
click('Start bridge');before=connected()
reboot();after=connected();assert after['generation']!=before['generation'];checks['opt_in_boot']=True
adb('shell','am','start','-n','org.omarchy.bridge/.DialActivity');time.sleep(1);click('Bridge');click('Stop bridge')
for cycle in range(3):
 click('Start bridge');value=connected();assert value['generation']!=after['generation'];after=value;click('Stop bridge')
 try:client.controls();raise AssertionError('Stop still serves')
 except OSError:pass
checks['three_stop_start_cycles']=True
reboot()
try:client.controls();raise AssertionError('Stopped bridge restarted after reboot')
except OSError:checks['stop_persists_through_boot']=True
Path('android-framework/results/lifecycle.json').write_text(json.dumps({'passed':True,'checks':checks,'phone_accessed':False},indent=2)+'\n')
print('Production AndroidKeyStore boot and stop/start lifecycle: passed',flush=True)
