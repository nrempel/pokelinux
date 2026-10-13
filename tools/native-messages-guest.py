#!/usr/bin/env python3
"""Owned ARM64 kernel + original Omarchy + distinct-UID Messages renderer fixture."""
import hashlib,json,os,platform,re,shutil,signal,socket,subprocess,tarfile,time,traceback
from pathlib import Path
O=Path('/results');P=Path('/workspace');processes=[];logs=[]
r={'passed':False,'hardware_verified':False,'architecture':platform.machine(),'modem_qualified':False,'fixture':'authored synthetic GSM DELIVER; previously qualified decoder; no modem ingestion','groups':[]}
def run(args,env=None,timeout=60):
 import tempfile
 with tempfile.TemporaryFile(mode='w+')as f:
  c=subprocess.run(args,stdout=f,stderr=subprocess.STDOUT,env=env,timeout=timeout);f.seek(0);v=f.read()
  if c.returncode:(O/'command-error.log').write_text(str(args)+'\n'+v);raise RuntimeError('owned command failed '+str(c.returncode))
  return v

def start(args,name,env=None):
 f=(O/name).open('w');logs.append(f);c=subprocess.Popen(args,stdout=f,stderr=subprocess.STDOUT,env=env,start_new_session=True);processes.append(c);return c

def wait(f,seconds=30):
 end=time.monotonic()+seconds
 while time.monotonic()<end:
  try:
   v=f()
   if v:return v
  except (FileNotFoundError,ConnectionRefusedError,OSError):pass
  time.sleep(.2)
 raise TimeoutError('bounded Messages guest condition')

def packet(path,v):
 with socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET)as s:s.settimeout(2);s.connect(str(path));s.send(json.dumps(v).encode());return json.loads(s.recv(4096))

def record(name):r['groups'].append(name);print('PASS '+name,flush=True)
try:
 config=json.loads((O/'run-config.json').read_text());C=P/config['source'];bins=P/config['bins'];r['selection']=config
 for name,digest in config['files'].items():assert hashlib.sha256((P/Path(config['source']).parent/name).read_bytes()).hexdigest()==digest
 r['kernel']=run(['uname','-a']);r['packages']=run(['pacman','-Q','hyprland','quickshell','qt6-base','qt6-declarative','qt6-wayland','fontconfig','acl'])
 # Pin original public Omarchy source and the same explicit Qt namespace patch.
 manifest=json.loads((P/'native-theme/public-manifest.json').read_text());archive=P/'artifacts/native-theme/omarchy-public.tar.gz';assert hashlib.sha256(archive.read_bytes()).hexdigest()==manifest['archive_sha256'];r['original_source']=manifest
 d=Path('/tmp/messages-original');d.mkdir()
 with tarfile.open(archive)as t:t.extractall(d,filter='data')
 original=d/('omarchy-'+manifest['commit'])
 for name,digest in manifest['files'].items():assert hashlib.sha256((original/name).read_bytes()).hexdigest()==digest
 import sys;sys.path.insert(0,str(C/'tools'));from omarchy_theme_patch import apply
 r['explicit_original_namespace_patch']=apply(original,json.loads((P/'sim/omarchy-source/source-manifest.json').read_text()))
 (O/'service-kernel').mkdir();run(['python3',str(C/'tools/native-messages-service-fixture.py'),'--bins',str(bins),'--out',str(O/'service-kernel'),'--phase','green','--retained-order'],env=dict(os.environ,NATIVE_MESSAGES_ISOLATED='1'),timeout=60);r['service_kernel']=json.loads((O/'service-kernel/result.json').read_text());record('complete_installed_service_process_SQLite_suite_under_actual_ARM64_kernel')
 (O/'service-foreign-fd').mkdir();run(['python3',str(C/'tools/native-messages-service-fixture.py'),'--bins',str(bins),'--out',str(O/'service-foreign-fd'),'--phase','recovery-only','--retained-fd','--runtime-suffix','foreign-fd'],env=dict(os.environ,NATIVE_MESSAGES_ISOLATED='1'),timeout=30);r['service_foreign_fd']=json.loads((O/'service-foreign-fd/result.json').read_text());record('actual_ARM64_dead_principal_retained_foreign_FD_local_recovery')
 B=Path('/tmp/messages-qt-build');r['build']=run(['python3',str(C/'tools/build-native-messages-qt-guest.py'),str(B),str(C)],timeout=420)
 run(['modprobe','virtio_gpu']);run(['udevadm','settle']);run(['usermod','-a','-G','video,input','alarm']);run(['useradd','-u','11010','-M','-s','/usr/bin/nologin','messages-app'])
 run(['install','-d','-o','alarm','-g','alarm','-m','700','/run/user/1000']);start(['seatd','-g','video'],'seatd.log',dict(os.environ,SEATD_VTBOUND='0'))
 h=Path('/tmp/messages-hypr.lua');h.write_text('hl.monitor({ output = "", mode = "800x900@60", position = "auto", scale = 1 })\nhl.config({ animations = { enabled = false }, decoration = { blur = { enabled = false }, shadow = { enabled = false } }, misc = { disable_hyprland_logo = true, disable_splash_rendering = true } })\nhl.permission("/usr/bin/grim", "screencopy", "allow")\n')
 env={'HOME':'/home/alarm','XDG_RUNTIME_DIR':'/run/user/1000','LIBSEAT_BACKEND':'seatd','AQ_NO_MODIFIERS':'1','XDG_SESSION_TYPE':'wayland','QT_QPA_PLATFORM':'wayland','OMARCHY_PATH':str(original),'PATH':str(original/'bin')+':/usr/local/bin:/usr/bin','OMARCHY_THEME_SKIP_BACKGROUND':'1'}
 def user(args):return ['runuser','-u','alarm','--','env',*[f'{k}={v}'for k,v in env.items()],*args]
 Path('/home/alarm/.bash_profile').write_text(f'export OMARCHY_PATH="{original}"\nexport PATH="$OMARCHY_PATH/bin:/usr/local/bin:/usr/bin"\n')
 plugin=Path('/home/alarm/.config/omarchy/plugins/pokelinux.theme-oracle');plugin.parent.mkdir(parents=True,exist_ok=True);shutil.copytree(P/'native-theme/oracle',plugin)
 cfg=Path('/home/alarm/.config/omarchy/shell.json');shellcfg=json.loads((original/'config/omarchy/shell.json').read_text());shellcfg['plugins'].append({'id':'pokelinux.theme-oracle'});cfg.write_text(json.dumps(shellcfg));run(['chown','-R','alarm:alarm','/home/alarm/.config'])
 compositor=start(user(['Hyprland','--config',str(h)]),'hyprland.log');wayland=wait(lambda:next((p for p in Path('/run/user/1000').glob('wayland-*')if p.is_socket()),None),120);env['WAYLAND_DISPLAY']=wayland.name
 control=wait(lambda:next(Path('/run/user/1000/hypr').glob('*/.socket.sock'),None),120);env['HYPRLAND_INSTANCE_SIGNATURE']=control.parent.name
 run(['systemctl','start','user@1000.service']);wait(lambda:Path('/run/user/1000/bus').is_socket());env['DBUS_SESSION_BUS_ADDRESS']='unix:path=/run/user/1000/bus';run(user(['systemctl','--user','import-environment',*env.keys()]))
 shell=start(user(['quickshell','-n','-p',str(original/'shell')]),'omarchy.log')
 def ipc(target,method):return run(user(['quickshell','ipc','-n','-p',str(original/'shell'),'call',target,method]),timeout=15).strip()
 def oracle():
  try:return json.loads(ipc('native-theme-oracle','snapshot'))
  except (RuntimeError,json.JSONDecodeError):return False
 wait(oracle,120);run(user(['omarchy-theme-set','tokyo-night']),timeout=120)
 # Actual root installed service/UI and fixed desktop launch-only ingress.
 I=Path('/usr/libexec/pokelinux');I.mkdir(parents=True,exist_ok=True);R=Path('/run/pokelinux/messages');R.mkdir(parents=True);S=Path('/var/lib/pokelinux');S.mkdir();S.chmod(0o700);(S/'policy').mkdir();(S/'policy').chmod(0o700)
 for name in ['messages-service','messages-launch','messages-stop','lifecycle-launcher']:shutil.copyfile(bins/name,I/name);(I/name).chmod(0o755)
 shutil.copyfile(B/'pokelinux-messages-observer',I/'pokelinux-messages');(I/'pokelinux-messages').chmod(0o755)
 theme=Path('/usr/share/pokelinux/appearance');theme.mkdir(parents=True)
 for name in ['PhoneTheme.qml','ThemePalette.js']:shutil.copyfile(C/'native-app'/name,theme/name)
 shutil.copyfile(C/'native-messages-ui/org.omarchy.Messages.desktop',Path('/usr/share/applications/org.omarchy.Messages.desktop'))
 apphome=Path('/var/lib/pokelinux-messages-app');apphome.mkdir();apphome.chmod(0o700);os.chown(apphome,11010,11010)
 appruntime=Path('/run/user/11010');appruntime.mkdir();appruntime.chmod(0o700);os.chown(appruntime,11010,11010)
 observe=Path('/run/messages-observer');observe.mkdir();observe.chmod(0o700);os.chown(observe,11010,11010)
 run(['setfacl','-m','u:11010:--x','/run/user/1000']);run(['setfacl','-m','u:11010:rw-',str(wayland)])
 # ACL only exact appearance paths and necessary ancestor traversal; never home recursion.
 def appearance_acl():
  source=Path('/home/alarm');source.chmod(0o700);run(['setfacl','-m','u:11010:--x',str(source)])
  theme_link=source/'.local/state/omarchy/current/theme';resolved=theme_link.resolve();assert resolved.is_dir() and (resolved.is_relative_to(source/'.local/state/omarchy') or resolved.is_relative_to(source/'.config/omarchy') or resolved.is_relative_to(original))
  files=[theme_link/'colors.toml',theme_link/'shell.toml',source/'.config/omarchy/shell.toml',source/'.config/fontconfig/fonts.conf']
  for file in files:
   for parent in [file.parent,*file.parent.parents]:
    if parent==source:break
    if parent.exists():run(['setfacl','-m','u:11010:--x',str(parent)])
   if file.exists():
    assert file.is_file() and not file.is_symlink(),'only selected regular appearance files receive read ACLs'
    run(['setfacl','-m','u:11010:r--',str(file)])
  run(['setfacl','-m','u:11010:r-x',str(theme_link)])
  for watched in [source/'.local/state/omarchy/current',source/'.config/omarchy',source/'.config/fontconfig']:
   if watched.is_dir():run(['setfacl','-m','u:11010:r-x',str(watched)])
 appearance_acl()
 private=Path('/home/alarm/private-message-credential');private.write_text('fixture only');private.chmod(0o600);os.chown(private,1000,1000)
 probe=subprocess.run(['runuser','-u','messages-app','--','cat',str(private)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);assert probe.returncode!=0,'theme access granted private home data'
 record('narrow_Wayland_theme_ACLs_preserve_private_desktop_home')
 profile=Path('/etc/pokelinux');profile.mkdir(exist_ok=True);(profile/'messages-profile.json').write_text(json.dumps({'apps':[{'app':'org.omarchy.Messages','uid':11010,'executable':str(I/'pokelinux-messages'),'args':[str(apphome),str(appruntime),str(wayland),'/home/alarm',str(R/'app.sock'),str(theme),'--isolated-observe',str(observe)]}]}))
 run([str(bins/'messages-seed-fixture'),str(S/'receipts'),'--renderer-text'],env=dict(os.environ,NATIVE_MESSAGES_ISOLATED='1'))
 r['executables']={name:hashlib.sha256((I/name).read_bytes()).hexdigest()for name in ['messages-service','messages-launch','lifecycle-launcher','pokelinux-messages']};r['consumer_sources']={str(f.relative_to(C)):hashlib.sha256(f.read_bytes()).hexdigest()for folder in ['native-messages','native-messages-ui','native-lifecycle']for f in (C/folder).rglob('*')if f.is_file() and f.suffix in ['.rs','.cpp','.h','.qml','.js','.toml','.qrc']}
 unit=Path('/etc/systemd/system/pokelinux-messages.service');shutil.copyfile(C/'native-messages/packaging/pokelinux-messages.service',unit);run(['systemctl','daemon-reload']);run(['systemctl','start','pokelinux-messages.service']);r['systemd_main_pid']=int(run(['systemctl','show','-p','MainPID','--value','pokelinux-messages.service']).strip());assert r['systemd_main_pid']>0
 def admin(v):return packet(R/'admin.sock',v)
 wait(lambda:admin({'op':'snapshot'}).get('ready'));wait(lambda:admin({'op':'messages_status'}).get('available'))
 assert admin({'op':'locked','locked':False})=={'result':'ok'};assert admin({'op':'grant','app':'org.omarchy.Messages','permission':'messages_read','allowed':True})=={'result':'ok'}
 # Standard installed desktop entry lookup/launch; actual alarm UID at launch ingress.
 run(user(['gtk-launch','org.omarchy.Messages']),timeout=15)
 def clients():return json.loads(run(user(['hyprctl','-j','clients'])))
 window=wait(lambda:next((v for v in clients()if v.get('title')=='Messages' and v.get('mapped')),None),90);assert not window['xwayland'];pid=window['pid'];assert Path(f'/proc/{pid}/status').read_text().split('Uid:\t')[1].split()[0]=='11010';r['window']={k:window[k]for k in ['pid','mapped','xwayland','class','title']}
 assert json.loads(run(user(['hyprctl','-j','activewindow'])))['pid']==pid,'trusted fixture did not observe actual foreground Messages window'
 assert admin({'op':'background','app':'org.omarchy.Messages','background':False})=={'result':'ok'}
 record('installed_desktop_launch_same_registered_distinct_UID_actual_Wayland_window')
 seq=0
 def snapshot():
  global seq;seq+=1;temp=observe/'control.tmp';temp.write_text(json.dumps({'sequence':seq}));os.chown(temp,11010,11010);temp.replace(observe/'control.json');v=wait(lambda:json.loads((observe/f'window-{seq}.json').read_text())if(observe/f'window-{seq}.json').exists()else False);r['last_observation']=v;return v
 v=wait(lambda:(v if (v:=snapshot())['literalFixture'] else False),20)
 assert v['plain'],'actual renderer interpreted incoming SMS body as rich text'
 record('actual_service_SMS_payload_is_literal_plain_text_in_native_renderer')
 def appearance():
  v=snapshot();o=oracle();return (v,o)if o and v['available'] and v['background']==o['background'] and v['pixel']==o['background'] and v['fontFamily']==o['fontFamily'] else False
 v,o=wait(appearance,30);r['theme_one']={'native':v,'original':o};record('original_Omarchy_tokyo_theme_and_font_match_native_window')
 run(user(['omarchy-theme-set','catppuccin-latte']),timeout=120);appearance_acl();v,o=wait(appearance,30);r['theme_two']={'native':v,'original':o};assert r['theme_one']['native']['background']!=v['background'];assert v['literalFixture'] and v['uid']==11010 and v['pid']==pid;assert any(c['address']==window['address'] and c['pid']==pid for c in clients());record('original_live_theme_switch_preserves_registered_window_and_local_text')
 # Real Qt window close during a held read, not only a driver or service stop.
 def current_worker():
  for candidate in Path('/proc').iterdir():
   if candidate.name.isdecimal():
    try:
     cmd=(candidate/'cmdline').read_bytes().split(b'\0');parent=int((candidate/'status').read_text().split('PPid:\t')[1].split()[0])
     if b'--messages-worker'in cmd and parent==r['systemd_main_pid']:return int(candidate.name)
    except (OSError,ValueError):pass
  raise AssertionError('owned storage process missing')
 old_worker=current_worker();old_window_pid=pid;os.kill(old_worker,signal.SIGSTOP);wait(lambda:admin({'op':'messages_status'}).get('busy'))
 assert re.fullmatch(r'0x[0-9a-f]+',window['address'])
 run(user(['hyprctl','dispatch','hl.dsp.window.close({ window = '+json.dumps('address:'+window['address'])+' })']),timeout=10)
 wait(lambda:not Path(f'/proc/{old_window_pid}').exists(),5);wait(lambda:admin({'op':'snapshot'}).get('principals')==0,5)
 run(user(['gtk-launch','org.omarchy.Messages']),timeout=15)
 window=wait(lambda:next((v for v in clients()if v.get('title')=='Messages' and v.get('mapped') and v['pid']!=old_window_pid),None),60);pid=window['pid'];assert Path(f'/proc/{pid}/status').read_text().split('Uid:\t')[1].split()[0]=='11010'
 assert json.loads(run(user(['hyprctl','-j','activewindow'])))['pid']==pid
 assert admin({'op':'background','app':'org.omarchy.Messages','background':False})=={'result':'ok'};wait(lambda:admin({'op':'messages_status'}).get('available'),5)
 assert current_worker()!=old_worker and not Path(f'/proc/{old_worker}').exists(),'reopen did not wait for actual old worker reap'
 v=wait(lambda:(v if (v:=snapshot())['pid']==pid and v['literalFixture'] and v['plain'] else False),15)
 record('actual_Qt_held_read_window_close_reap_desktop_reopen_without_root_restart')
 assert admin({'op':'grant','app':'org.omarchy.Messages','permission':'messages_read','allowed':False})=={'result':'ok'};v=wait(lambda:(v if (v:=snapshot())['accessDenied'] and v['bodyCount']==0 else False),15);record('native_window_clears_previously_released_text_on_denied_refresh')
 run(['systemctl','stop','pokelinux-messages.service'],timeout=10);assert run(['systemctl','show','-p','Result','--value','pokelinux-messages.service']).strip()=='success';wait(lambda:not Path(f'/proc/{pid}').exists());record('installed_systemd_authenticated_stop_reaps_window_and_storage_children')
 # Qualify the production target too; it has no observer or fixture branch.
 shutil.copyfile(B/'pokelinux-messages',I/'pokelinux-messages');(I/'pokelinux-messages').chmod(0o755)
 data=json.loads((profile/'messages-profile.json').read_text());data['apps'][0]['args']=data['apps'][0]['args'][:6];(profile/'messages-profile.json').write_text(json.dumps(data))
 run(['systemctl','start','pokelinux-messages.service']);wait(lambda:admin({'op':'snapshot'}).get('ready'));wait(lambda:admin({'op':'messages_status'}).get('available'));run(user(['gtk-launch','org.omarchy.Messages']),timeout=15)
 production=wait(lambda:next((v for v in clients()if v.get('title')=='Messages' and v.get('mapped')),None),90);production_pid=production['pid'];assert production_pid!=pid and Path(f'/proc/{production_pid}/status').read_text().split('Uid:\t')[1].split()[0]=='11010'
 assert json.loads(run(user(['hyprctl','-j','activewindow'])))['pid']==production_pid
 assert admin({'op':'locked','locked':False})=={'result':'ok'};assert admin({'op':'background','app':'org.omarchy.Messages','background':False})=={'result':'ok'};assert admin({'op':'grant','app':'org.omarchy.Messages','permission':'messages_read','allowed':True})=={'result':'ok'};time.sleep(2)
 run(user(['grim',str(O/'production-desktop.png')]));r['production_binary']=json.loads((B/'binary-manifest.json').read_text());r['production_window']={k:production[k]for k in ['pid','mapped','xwayland','class','title']}
 run(['systemctl','stop','pokelinux-messages.service'],timeout=10);assert run(['systemctl','show','-p','Result','--value','pokelinux-messages.service']).strip()=='success';wait(lambda:not Path(f'/proc/{production_pid}').exists());record('installed_production_binary_without_observer_real_UID_Wayland_and_graceful_stop')
 for name in ['pokelinux-messages','pokelinux-messages-observer','binary-manifest.json']:shutil.copyfile(B/name,O/name)
 for f in observe.glob('*'):
  if f.suffix in ['.png','.json']:shutil.copyfile(f,O/f.name)
 r['passed']=True
except BaseException as e:r['error']=str(e);r['traceback']=traceback.format_exc();print(r['traceback'],flush=True)
finally:
 if Path('/run/messages-observer').exists():
  for f in Path('/run/messages-observer').glob('*'):
   if f.suffix in ['.png','.json']:shutil.copyfile(f,O/f.name)
 if Path('/etc/systemd/system/pokelinux-messages.service').exists():
  subprocess.run(['systemctl','stop','pokelinux-messages.service'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=10)
 for c in reversed(processes):
  if c.poll()is None:
   try:os.killpg(c.pid,15);c.wait(timeout=5)
   except BaseException:
    try:os.killpg(c.pid,9);c.wait(timeout=5)
    except BaseException:pass
 for f in logs:f.close()
 (O/'result.json').write_text(json.dumps(r,indent=2)+'\n');run(['sync']);subprocess.run(['systemctl','poweroff'])
