#!/usr/bin/env python3
import hashlib,json,os,platform,select,shlex,shutil,signal,subprocess,time,traceback
from pathlib import Path
O=Path('/results'); P=Path('/results/source'); B=Path('/tmp/native-session-probe-build')
processes=[]; logs=[]
r={'passed':False,'hardware_verified':False,'architecture':platform.machine(),'groups':[]}
def run(args,env=None,timeout=60):
 c=subprocess.run(args,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,env=env,timeout=timeout,text=True)
 if c.returncode:
  (O/'command-error.log').write_text(repr(args)+'\n'+c.stdout)
  raise RuntimeError('command failed '+repr(args)+'\n'+c.stdout[-2000:])
 return c.stdout

def start(args,name,env=None):
 f=(O/name).open('w'); logs.append(f)
 c=subprocess.Popen(args,stdout=f,stderr=subprocess.STDOUT,env=env,start_new_session=True); processes.append(c); return c

def wait(fn,seconds=30):
 end=time.monotonic()+seconds
 while time.monotonic()<end:
  try:
   v=fn()
   if v:return v
  except (FileNotFoundError,OSError,ValueError,RuntimeError):pass
  time.sleep(.2)
 raise TimeoutError('build probe bounded wait')

def group(s): r['groups'].append(s); print('PASS '+s,flush=True)
def kill(c):
 if c.poll() is None:
  os.killpg(c.pid,signal.SIGTERM)
  try:c.wait(4)
  except subprocess.TimeoutExpired:os.killpg(c.pid,signal.SIGKILL);c.wait(4)
try:
 r['source_hashes']={f.name:hashlib.sha256(f.read_bytes()).hexdigest() for f in P.iterdir() if f.is_file()}
 r['kernel']=run(['uname','-a'])
 r['network_interfaces']=sorted(p.name for p in Path('/sys/class/net').iterdir());assert r['network_interfaces']==['lo']
 r['packages']=run(['pacman','-Q','hyprland','gcc','binutils','pkgconf','hyprutils','aquamarine','hyprgraphics','hyprlang','hyprcursor','wayland','quickshell'])
 r['compiler']=run(['g++','--version'])
 r['pkgconfig']=run(['pkg-config','--cflags','hyprland'])
 r['hyprland_version_header']=Path('/usr/include/hyprland/src/version.h').read_text()
 r['hyprland_elf_sha256']=hashlib.sha256(Path('/usr/bin/Hyprland').read_bytes()).hexdigest()
 r['hyprland_build_id']=run(['readelf','-n','/usr/bin/Hyprland'])
 r['hyprland_compiler_comment']=run(['readelf','-p','.comment','/usr/bin/Hyprland'])
 r['package_sha256']=hashlib.sha256(Path('/var/cache/pacman/pkg/hyprland-0.56.2-4-aarch64.pkg.tar.xz').read_bytes()).hexdigest()
 (O/'package.BUILDINFO').write_text(run(['tar','-xOf','/var/cache/pacman/pkg/hyprland-0.56.2-4-aarch64.pkg.tar.xz','.BUILDINFO']))
 (O/'hyprland-symbols.txt').write_text(run(['nm','-D','--defined-only','--demangle','/usr/bin/Hyprland']))
 headers={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(Path('/usr/include/hyprland').rglob('*')) if p.is_file()}
 (O/'headers.json').write_text(json.dumps(headers,indent=2)+'\n')
 r['headers_manifest_sha256']=hashlib.sha256((O/'headers.json').read_bytes()).hexdigest()
 B.mkdir(); shutil.copyfile(P/'probe.cpp',B/'probe.cpp'); shutil.copyfile(P/'lock-client.c',B/'lock-client.c')
 cflags=shlex.split(run(['pkg-config','--cflags','hyprland']))
 command=['g++','-std=c++23','-fPIC','-shared','-O2','-g',*cflags,str(B/'probe.cpp'),'-o',str(B/'probe.so')]
 r['compile_command']=command
 (O/'build.log').write_text(run(command,timeout=420))
 shutil.copyfile(B/'probe.so',O/'probe.so'); r['probe_elf_sha256']=hashlib.sha256((O/'probe.so').read_bytes()).hexdigest()
 r['probe_build_id']=run(['readelf','-n',str(B/'probe.so')])
 (O/'probe-undefined-symbols.txt').write_text(run(['nm','-D','--undefined-only','--demangle',str(B/'probe.so')]))
 group('matching_installed_headers_compile_native_aarch64_plugin')
 xml='/usr/share/wayland-protocols/staging/ext-session-lock/ext-session-lock-v1.xml'
 run(['wayland-scanner','client-header',xml,str(B/'session-lock-client.h')])
 run(['wayland-scanner','private-code',xml,str(B/'session-lock-protocol.c')])
 run(['gcc','-O2',str(B/'lock-client.c'),str(B/'session-lock-protocol.c'),'-lwayland-client','-o',str(B/'lock-client')])
 run(['modprobe','virtio_gpu']);run(['udevadm','settle']);run(['usermod','-a','-G','video,input','alarm'])
 run(['install','-d','-o','alarm','-g','alarm','-m','700','/run/user/1000'])
 start(['seatd','-g','video'],'seatd.log',dict(os.environ,SEATD_VTBOUND='0'))
 h=B/'hypr.lua';h.write_text('hl.monitor({ output = "", mode = "800x900@60", position = "auto", scale = 1 })\nhl.config({ animations = { enabled = false }, decoration = { blur = { enabled = false }, shadow = { enabled = false } }, misc = { disable_hyprland_logo = true, disable_splash_rendering = true, allow_session_lock_restore = true } })\n')
 env={'HOME':'/home/alarm','XDG_RUNTIME_DIR':'/run/user/1000','LIBSEAT_BACKEND':'seatd','AQ_NO_MODIFIERS':'1','XDG_SESSION_TYPE':'wayland','QT_QPA_PLATFORM':'wayland','PATH':'/usr/local/bin:/usr/bin'}
 def user(args):return ['runuser','-u','alarm','--','env',*[f'{k}={v}' for k,v in env.items()],*args]
 compositor=start(user(['Hyprland','--config',str(h)]),'hyprland.log')
 wayland=wait(lambda:next((p for p in Path('/run/user/1000').glob('wayland-*') if p.is_socket()),None),120)
 env['WAYLAND_DISPLAY']=wayland.name
 control=wait(lambda:next(Path('/run/user/1000/hypr').glob('*/.socket.sock'),None),120); env['HYPRLAND_INSTANCE_SIGNATURE']=control.parent.name
 r['installed_running_version']=run(user(['hyprctl','-j','version']))
 r['load_plugin']=run(user(['hyprctl','plugin','load',str(B/'probe.so')]))
 def snap():return json.loads(run(user(['hyprctl','nativebuildprobe']),timeout=5))
 r['initial_snapshot']=snap(); assert r['initial_snapshot']['locked'] is False
 group('exact_installed_hyprland_plugin_load_and_initial_protocol_snapshot')
 foot=start(user(['foot','--app-id=native-build-test','sleep','180']),'foot.log')
 focused=wait(lambda:(s if (s:=snap())['eligible'] else None),30)
 r['native_window_snapshot']=focused
 active=json.loads(run(user(['hyprctl','-j','activewindow'])))
 assert focused['uid']==1000 and focused['gid']==1000 and focused['pid']==active['pid'] and focused['pid']>0
 r['native_focus_pid_proc_status']=Path(f"/proc/{focused['pid']}/status").read_text()
 group('central_seat_native_window_credentials_match_actual_kernel_principal')
 peer_check_fd=os.pidfd_open(focused['pid'],0)
 try:
  check_stat=os.fstat(peer_check_fd);check_poll=select.poll();check_poll.register(peer_check_fd,select.POLLIN);check_events=check_poll.poll(0)
  r['independent_pidfd_identity']={'pid':focused['pid'],'dev':check_stat.st_dev,'ino':check_stat.st_ino,'live':not check_events,'events':check_events,'fdinfo':Path(f'/proc/self/fdinfo/{peer_check_fd}').read_text()}
  assert focused['peer_pidfd_valid'] and focused['peer_pidfd_live'] and focused['peer_pidfs_magic']==0x50494446
  assert check_stat.st_dev==focused['peer_pidfs_dev'] and check_stat.st_ino==focused['peer_pidfs_ino'] and not check_events
  assert os.get_inheritable(peer_check_fd) is False
 finally:os.close(peer_check_fd)
 group('actual_wayland_SO_PEERPIDFD_pidfs_identity_matches_independent_current_task_pidfd')
 r['null_focus_snapshot']=json.loads(run(user(['hyprctl','nativebuildclearfocus'])))
 assert r['null_focus_snapshot']['pid']==0 and r['null_focus_snapshot']['eligible'] is False
 group('central_seat_null_focus_signal_observed')
 run(user(['hyprctl','dispatch','hl.dsp.focus({ window = \"class:native-build-test\" })']))
 wait(lambda:snap()['eligible'])
 layerdir=B/'layer';layerdir.mkdir();(layerdir/'shell.qml').write_text('import QtQuick\nimport Quickshell\nimport Quickshell.Wayland\nScope { PanelWindow { anchors { top: true; left: true; right: true } implicitHeight: 80; color: "#223344"; WlrLayershell.layer: WlrLayer.Overlay; WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive; Text { text: "Native layer focus probe"; color: "white" } } }\n')
 layer=start(user(['quickshell','-n','-p',str(layerdir)]),'layer.log')
 layerfocus=wait(lambda:(s if (s:=snap())['type']==3 else None),30)
 active=json.loads(run(user(['hyprctl','-j','activewindow'])))
 r['layer_focus_snapshot']=layerfocus;r['layer_activewindow']=active
 assert layerfocus['eligible'] is False and layerfocus['pid']!=focused['pid'] and active['pid']==focused['pid']
 group('actual_layer_focus_differs_from_remembered_activewindow_pid')
 kill(layer); wait(lambda:snap()['eligible'])
 locker=start(user([str(B/'lock-client')]),'early-lock.log')
 early=wait(lambda:(s if (s:=snap())['locked'] else None),10)
 r['early_lock_snapshot']=early; assert early['complete'] is False
 kill(locker);time.sleep(.4)
 r['stranded_lock_snapshot']=snap();assert r['stranded_lock_snapshot']['locked'] is True and r['stranded_lock_snapshot']['complete'] is False
 group('real_ext_session_lock_early_admission_and_dead_locker_remain_locked')
 restored=start(user([str(B/'lock-client')]),'restore-lock.log')
 wait(lambda:(O/'restore-lock.log').read_text().find('LOCKED')>=0,10)
 r['fallback_completed_snapshot']=snap();assert r['fallback_completed_snapshot']['locked'] and r['fallback_completed_snapshot']['complete']
 group('restored_lock_and_five_second_completion_fallback_are_separate')
 lockpid=int((O/'restore-lock.log').read_text().split('pid=')[1].split()[0]);os.kill(lockpid,signal.SIGUSR1)
 wait(lambda:snap()['locked'] is False,10);r['protocol_unlock_snapshot']=snap()
 group('real_protocol_unlock_signal_observed_after_protocol_state_clear')
 forceLocker=start(user([str(B/'lock-client')]),'force-lock.log');wait(lambda:snap()['locked'])
 r['force_unlock_snapshot']=json.loads(run(user(['hyprctl','nativebuildforceunlock'])))
 assert r['force_unlock_snapshot']['locked'] is False
 kill(forceLocker)
 group('manager_force_unlock_signal_observed')
 log=Path('/tmp/native-session-build-probe.ndjson').read_text();(O/'events.ndjson').write_text(log)
 events=[json.loads(s) for s in log.splitlines()]
 assert any(s['reason']=='protocolNewLock' and s['locked'] is False for s in events)
 assert any(s['reason']=='managerLock' and s['locked'] is False for s in events)
 r['event_count']=len(events)
 group('first_lock_signal_precedes_protocol_m_locked_assignment_executed')
 r['unload_plugin']=run(user(['hyprctl','plugin','unload',str(B/'probe.so')]))
 r['passed']=True
except Exception:
 r['error']=traceback.format_exc();print(r['error'],flush=True)
finally:
 if Path('/tmp/native-session-build-probe.ndjson').exists():shutil.copyfile('/tmp/native-session-build-probe.ndjson',O/'events.ndjson')
 (O/'result.json').write_text(json.dumps(r,indent=2)+'\n')
 for c in reversed(processes):
  try:kill(c)
  except Exception:pass
 for f in logs:f.close()
 subprocess.run(['systemctl','poweroff'],timeout=20)
