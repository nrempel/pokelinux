#!/usr/bin/env python3
"""Stock BIOS native graphics composition; original finite QMP/KVM and owned child custody."""
import hashlib,json,os,signal,socket,stat,struct,sys,time
from pathlib import Path
sys.dont_write_bytecode=True
from owned import Scope,now
from viewer_host import ViewerOwner
R=Path('/results');S=json.loads((R/'selection.json').read_text());T0=now();QEMU_UNTIL=T0+780_000_000_000;CLOSE_UNTIL=T0+800_000_000_000;scope=Scope(R)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
result={'schema':1,'passed':False,'scope':S['scope'],'started_at_ns':T0,'original_QEMU_until_ns':QEMU_UNTIL,'original_closure_until_ns':CLOSE_UNTIL,'phone_accessed':False,'physical_display':False,'display_or_Pixel_profile_qualified':False,'KVM_fallback_permitted':False}
def command(argv,name,seconds=10,limit=65536):
 code,data=scope.run(argv,name,min(QEMU_UNTIL,now()+seconds*1_000_000_000),limit=limit);assert code==0,(name,code,data[:512]);return data

qmp_socket=None;qmp_receipt=None;qmp_buffer=bytearray();qmp_total=0;qmp_eof=False;viewer=None

def qmp_decode():
 while b'\n'in qmp_buffer:
  line,_,tail=qmp_buffer.partition(b'\n');qmp_buffer[:]=tail;assert len(line)<=8192
  value=json.loads(line);qmp_receipt['messages'].append({'value':value,'observed_at_boottime_ns':now()});assert len(qmp_receipt['messages'])<=32

def qmp_read(until,blocking=False):
 global qmp_total,qmp_eof
 assert now()<until
 qmp_socket.settimeout(max(.001,(until-now())/1e9))if blocking else qmp_socket.setblocking(False)
 try:data=qmp_socket.recv(4096)
 except(BlockingIOError,InterruptedError):return
 if not data:
  qmp_eof=True;qmp_receipt['socket_EOF_at_boottime_ns']=now();assert not qmp_buffer;return
 qmp_total+=len(data);assert qmp_total<=65536;qmp_buffer.extend(data);assert len(qmp_buffer)<=8192;qmp_decode();assert now()<until

def qmp_kvm(child,path):
 global qmp_socket,qmp_receipt
 until=min(QEMU_UNTIL,now()+20_000_000_000)
 qmp_receipt={'request_started_at_boottime_ns':now(),'original_handshake_until_ns':until,'original_socket_until_ns':QEMU_UNTIL,'messages':[],'not_guest_self_report':True};result['actual_QMP_KVM']=qmp_receipt
 while not path.is_socket():
  scope.pump();scope.check();assert scope.live(child)and now()<until;time.sleep(.005)
 qmp_socket=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
 def remaining():assert now()<until;qmp_socket.settimeout(max(.001,(until-now())/1e9))
 remaining();qmp_socket.connect(str(path));peer=struct.unpack('3i',qmp_socket.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert peer==(child['child'].pid,os.getuid(),os.getgid());qmp_receipt['actual_peer_PID_UID_GID']=list(peer)
 while not qmp_receipt['messages']:qmp_read(until,True);assert not qmp_eof
 assert 'QMP'in qmp_receipt['messages'][0]['value']
 for ident,verb in[(1,'qmp_capabilities'),(2,'query-kvm')]:
  packet=(json.dumps({'execute':verb,'id':ident},separators=(',',':'))+'\r\n').encode();remaining();qmp_socket.sendall(packet);assert now()<until
  while not any(r['value'].get('id')==ident for r in qmp_receipt['messages']):qmp_read(until,True);assert not qmp_eof
  answers=[r['value']for r in qmp_receipt['messages']if r['value'].get('id')==ident];assert len(answers)==1 and 'return'in answers[0]
  if ident==2:assert answers[0]['return']=={'enabled':True,'present':True}
 qmp_receipt.update({'query_kvm_enabled':True,'query_kvm_present':True,'handshake_completed_at_boottime_ns':now()});qmp_socket.setblocking(False)

def viewer_query_mice():
 until=min(QEMU_UNTIL,now()+10_000_000_000)
 assert not any(r['value'].get('id')==3 for r in qmp_receipt['messages'])
 packet=(json.dumps({'execute':'query-mice','id':3},separators=(',',':'))+'\r\n').encode();qmp_socket.settimeout(max(.001,(until-now())/1e9));qmp_socket.sendall(packet);assert now()<until
 while not any(r['value'].get('id')==3 for r in qmp_receipt['messages']):
  scope.pump();scope.check();assert scope.live(qemu)and now()<until;qmp_read(until,True);assert not qmp_eof
 replies=[r['value']for r in qmp_receipt['messages']if r['value'].get('id')==3];assert len(replies)==1 and isinstance(replies[0].get('return'),list)and'error'not in replies[0];qmp_socket.setblocking(False);return replies[0]['return']

try:
 assert S['phase']=='native-x86-installed-Source-Sounds-composition'and S['native_guest_admission']['qualified']is True
 assert os.getuid()==os.getgid()==1000 and 993 in os.getgroups()
 kvm=Path('/dev/kvm').stat();assert stat.S_ISCHR(kvm.st_mode)and os.major(kvm.st_rdev)==10 and os.minor(kvm.st_rdev)==232
 result['actual_container_UID_GID_groups']={'UID':os.getuid(),'GID':os.getgid(),'groups':os.getgroups(),'KVM_device_major_minor':[os.major(kvm.st_rdev),os.minor(kvm.st_rdev)]}
 # Bounded extraction of existing public tool reports, before qemu-img/ISO/VM.
 tool=Path('/usr/share/pokelinux-x86-qemu-build');reports=R/'runtime-tool-reports';reports.mkdir();result['runtime_tool_reports']={}
 for name in ['packages-before.tsv','packages-after.tsv','selected-qemu-family-version.txt','qemu-system-x86-version.txt','genisoimage-version.txt']:
  fd=os.open(tool/name,os.O_RDONLY|os.O_CLOEXEC|os.O_NOFOLLOW)
  try:
   info=os.fstat(fd);assert stat.S_ISREG(info.st_mode)and info.st_uid==0 and info.st_size<=262144
   data=os.read(fd,262145);assert len(data)==info.st_size and len(data)<=262144
  finally:os.close(fd)
  (reports/name).write_bytes(data);result['runtime_tool_reports'][name]={'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'regular_file_not_PIPE_EOF':True}
 family=(reports/'selected-qemu-family-version.txt').read_text().strip();packages={}
 for line in (reports/'packages-after.tsv').read_text().splitlines():
  fields=line.split('\t');assert len(fields)==3;packages[fields[0]]=fields[1:]
 assert packages['qemu-system-common']==packages['qemu-system-x86']==[family,'amd64']
 result['actual_matching_QEMU_package_family']=family
 result['actual_qemu_version']=command(['/usr/bin/qemu-system-x86_64','--version'],'qemu-version.log').decode()
 assert result['actual_qemu_version'].encode()==(reports/'qemu-system-x86-version.txt').read_bytes()
 # Image itself is read-only. Only a fresh child overlay may receive disk writes.
 overlay=Path('/tmp/native-composition.qcow2');assert not overlay.exists()
 command(['/usr/bin/qemu-img','create','-f','qcow2','-F','qcow2','-b','/vm/provisioned-layer.qcow2',str(overlay)],'qemu-img.log')
 seed=Path('/tmp/cloud-seed');seed.mkdir();selection_sha=sha(R/'selection.json')
 payload=[{'path':'/usr/local/libexec/pokelinux-native-composition/'+name,'owner':'root:root','permissions':'0500','content':(R/'source'/name).read_text()}for name in ['owned.py','cloud-bootstrap.py']]
 # JSON is valid YAML; #cloud-config remains the mandatory first line.
 cloud={'users':[],'package_update':False,'package_upgrade':False,'ssh_pwauth':False,'disable_root':True,'write_files':payload,'bootcmd':[['/usr/bin/python3','-c',(R/'source/owned.py').read_text()+'\n'+(R/'source/early.py').read_text()]],'power_state':{'mode':'poweroff','delay':'now','timeout':10,'condition':['test','-f','/results/viewer-input/retired.json']},'runcmd':[['/usr/bin/python3','/usr/local/libexec/pokelinux-native-composition/cloud-bootstrap.py','--selection-sha256',selection_sha,'--instance-id',S['label']]]}
 metadata={'instance-id':S['label'],'local-hostname':'pokelinux-native-composition','dsmode':'local'}
 (seed/'user-data').write_text('#cloud-config\n'+json.dumps(cloud,indent=2)+'\n');(seed/'meta-data').write_text(json.dumps(metadata,indent=2)+'\n')
 for name in ['user-data','meta-data']:(R/('seed-'+name)).write_bytes((seed/name).read_bytes())
 iso=Path('/tmp/cidata.iso');assert not iso.exists()
 command(['/usr/bin/genisoimage','-quiet','-output',str(iso),'-volid','cidata','-joliet','-rock',str(seed/'user-data'),str(seed/'meta-data')],'genisoimage.log')
 result['seed']={'user_data_sha256':sha(seed/'user-data'),'meta_data_sha256':sha(seed/'meta-data'),'iso_sha256':sha(iso),'iso_bytes':iso.stat().st_size,'local_NoCloud_only':True,'no_password_login_or_package_operation_requested':True,'runtime_only_mask_of_offline_time_wait_unit':True}
 xvfb=scope.start(['Xvfb',':97','-screen','0','1080x2424x24','-dpi','100','-nolisten','tcp'],'Xvfb.log',1048576,cutoff=QEMU_UNTIL,wait_cutoff=CLOSE_UNTIL)
 ready_until=min(QEMU_UNTIL,now()+10_000_000_000)
 while not Path('/tmp/.X11-unix/X97').is_socket():
  scope.pump();scope.check();assert scope.live(xvfb)and now()<ready_until;time.sleep(.02)
 env=dict(os.environ,DISPLAY=':97',LIBGL_ALWAYS_SOFTWARE='1',SDL_VIDEODRIVER='x11',XDG_RUNTIME_DIR='/tmp',TMPDIR='/tmp',LP_NUM_THREADS='4')
 qmp=Path('/tmp/native-composition-qmp.sock');assert not qmp.exists()
 argv=['/usr/bin/qemu-system-x86_64','-machine','q35,vmport=off','-accel','kvm','-cpu','host','-smp','6','-m','6144','-drive','file='+str(overlay)+',if=virtio,format=qcow2','-drive','file='+str(iso)+',if=virtio,format=raw,readonly=on','-fsdev','local,id=results,path=/results,security_model=mapped-xattr','-device','virtio-9p-pci,fsdev=results,mount_tag=results','-fsdev','local,id=assets,path=/assets,security_model=none,readonly=on','-device','virtio-9p-pci,fsdev=assets,mount_tag=assets','-vga','none','-device','virtio-gpu-gl-pci,id=phone-display,xres=1080,yres=2424','-device','virtio-tablet-pci,id=viewer-tablet','-device','virtio-rng-pci','-nic','none','-display','sdl,gl=on','-full-screen','-serial','stdio','-monitor','none','-qmp','unix:'+str(qmp)+',server=on,wait=off','-no-reboot']
 qemu=scope.start(argv,'console.log',8388608,env=env,cutoff=QEMU_UNTIL);result['QEMU_argv']=argv
 qmp_kvm(qemu,qmp)
 viewer=ViewerOwner(scope,R,S,qemu,xvfb,QEMU_UNTIL,CLOSE_UNTIL,lambda:qmp_read(QEMU_UNTIL),viewer_query_mice)
 while qemu['child'].poll()is None:
  assert now()<QEMU_UNTIL;scope.pump();scope.check();qmp_read(QEMU_UNTIL);viewer.pump();assert not(R/'viewer-input/guest-fence-failure.json').exists(),'guest withheld teardown because viewer retirement was unknown';time.sleep(.005)
 assert viewer.retired and viewer.passed;result['viewer_input_host']=viewer.record
 code=scope.finish(qemu,QEMU_UNTIL);result['QEMU_actual_wait_exit']=code;assert code==0
 while not qmp_eof:qmp_read(QEMU_UNTIL);time.sleep(.001)
 shutdowns=[r for r in qmp_receipt['messages']if r['value'].get('event')=='SHUTDOWN'];assert len(shutdowns)==1 and shutdowns[0]['value']['data'].get('guest')is True and shutdowns[0]['value']['data'].get('reason')=='guest-shutdown'
 qmp_receipt.update({'retained_same_socket_until_actual_QEMU_exit':True,'actual_guest_SHUTDOWN':shutdowns[0],'socket_bytes':qmp_total,'socket_EOF_before_original_cutoff':qmp_receipt['socket_EOF_at_boottime_ns']<QEMU_UNTIL});assert qmp_receipt['socket_EOF_before_original_cutoff']
 bootstrap=json.loads((R/'bootstrap-result.json').read_text());guest=json.loads((R/'result.json').read_text())
 assert bootstrap['passed']and bootstrap['selection_sha256']==selection_sha and bootstrap['instance_id']==S['label']and bootstrap['actual_resuid']==bootstrap['actual_resgid']==[0,0,0]and bootstrap['actual_uname_m']=='x86_64'
 assert bootstrap['actual_guest_result_sha256']==sha(R/'result.json')and guest['selection_sha256']==selection_sha
 lines=(R/'console.log').read_bytes().splitlines();markers=[json.loads(line[len(b'POKELINUX_NATIVE_COMPOSITION '):])for line in lines if line.startswith(b'POKELINUX_NATIVE_COMPOSITION ')]
 expected={k:bootstrap.get(k)for k in ['schema','passed','selection_sha256','instance_id','actual_resuid','actual_resgid','actual_uname_m','actual_kernel_release','actual_guest_result_sha256','guest_functional_passed']};expected['bootstrap_result_sha256']=sha(R/'bootstrap-result.json');assert markers==[expected]
 for row in bootstrap['owned_process_receipts']:
  assert row['returncode']==0 and row['actual_direct_parent_wait']and row['stdout_EOF']and row['wait_before_original_cutoff']and row['EOF_before_original_cutoff']and not row['output_overflow']
 result.update(actual_bootstrap_result_sha256=sha(R/'bootstrap-result.json'),actual_guest_result_sha256=sha(R/'result.json'),serial_guest_identity_and_result_hash_match=True,stock_guest_root_x86_64_identity_and_QMP_observed_guest_shutdown_qualified=True,guest_functional_passed=guest['passed'],actual_boundary_step_ids=[v['id']for v in guest['boundary_steps']],native_virtual_display_pixels=[1080,2424],display_or_Pixel_profile_qualified=(guest.get('Pixel10_profile',{}).get('qualification')or{}).get('qualified',False))
 if scope.live(xvfb):signal.pidfd_send_signal(xvfb['pidfd'],signal.SIGTERM)
 assert scope.finish(xvfb,CLOSE_UNTIL)==0
 result['disposable_overlay_bytes']=overlay.stat().st_size;assert result['disposable_overlay_bytes']<=2147483648
except BaseException as error:result['failure']=type(error).__name__+': '+str(error)
finally:
 if qmp_socket is not None:qmp_socket.close()
 errors=scope.close(CLOSE_UNTIL);result.update({'cleanup_errors':errors,'owned_process_receipts':scope.receipts,'completed_at_ns':now()});result['passed']='failure'not in result and not errors and len(scope.receipts)==8 and all(r['returncode']==0 and r['actual_direct_parent_wait']and r['stdout_EOF']and r['wait_before_original_cutoff']and r['EOF_before_original_cutoff']for r in scope.receipts)and now()<CLOSE_UNTIL
 (R/'boot-result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'boot_closed':result['passed'],'QEMU_actual_wait_exit':result.get('QEMU_actual_wait_exit')}),flush=True)
if not result['passed']:raise SystemExit(1)
