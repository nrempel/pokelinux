#!/usr/bin/env python3
"""Real normal appearance collector on a fresh UID1000 home.

Only the build-selected production init command varies between RED and GREEN.
No palette, Source frame, shell host, clock, or service reply is authored here.
"""
import argparse, json, os, socket, struct, sys, time
from pathlib import Path
from owned import Scope, now
p=argparse.ArgumentParser();p.add_argument('--appearance',required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--provisioner');p.add_argument('--assets',type=Path);a=p.parse_args()
out=a.output;out.mkdir();scope=Scope(out);work=now()+30_000_000_000;close=work+5_000_000_000;child=None
result={'schema':1,'passed':False,'authored_palette_or_shell_host':False,'VM_or_graphics_executed':False,'normal_init_selected':bool(a.provisioner)}
def rpc(path,request):
 until=min(work,now()+500_000_000)
 with socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET)as s:
  s.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1);s.settimeout(max(.001,(until-now())/1e9));s.connect(str(path));peer=struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert peer==(child['child'].pid,0,0)
  data=json.dumps(request,separators=(',',':')).encode();assert s.send(data)==len(data);assert now()<until
  raw,anc,flags,_=s.recvmsg(8192,socket.CMSG_SPACE(12));assert now()<until and not flags&(socket.MSG_TRUNC|socket.MSG_CTRUNC)
  assert len(anc)==1 and anc[0][:2]==(socket.SOL_SOCKET,socket.SCM_CREDENTIALS)and struct.unpack('3i',anc[0][2])==peer
  value=json.loads(raw);assert now()<until;return value
try:
 root=Path('/run/theme-boundary');root.mkdir(mode=0o755);home=root/'home';runtime=root/'runtime'
 for directory in [home,runtime]:directory.mkdir(mode=0o700);os.chown(directory,1000,1000)
 config=root/'appearance.json';config.write_text(json.dumps({'home':str(home),'runtime':str(runtime)}));config.chmod(0o644)
 result['initial_home_has_no_selected_theme']=not(home/'.local/state/omarchy/current/theme').exists()
 assert result['initial_home_has_no_selected_theme']
 if a.provisioner:
  assert a.assets is not None
  code,_=scope.run([a.provisioner,str(config),str(a.assets)],'normal-theme-init.log',min(work,now()+20_000_000_000),limit=1024**2);assert code==0,'normal production theme initialization failed'
 public=root/'appearance.sock';control=root/'appearance-control.sock'
 child=scope.start([a.appearance,'--serve',str(public),str(config)],'normal-appearance.log',1024**2,cutoff=work)
 until=min(work,now()+3_000_000_000)
 while not control.is_socket():scope.pump();assert scope.live(child)and now()<until;time.sleep(.002)
 while True:
  status=rpc(control,{'op':'status'});scope.pump()
  if status['completed']>=1:break
  assert scope.live(child)and now()<until;time.sleep(.002)
 result['completed_collection_status']=status
 snapshot=rpc(public,{'op':'snapshot'});result['actual_snapshot']=snapshot
 assert snapshot['schema']==3 and int(snapshot['until_ns'])>now()
 assert snapshot['available'] is True,'normal fresh home has no genuine default-theme publication after actual completed collection'
 result['genuine_default_palette']=snapshot['appearance']['palette']
 assert snapshot['appearance']['palette']['background']=={'r':6682,'g':6939,'b':9766,'a':65535}
 result['passed']=True
except BaseException as error:result.update(error=type(error).__name__+': '+str(error))
finally:
 if child is not None:
  try:
   result['actual_stop_reply']=rpc(Path('/run/theme-boundary/appearance-control.sock'),{'op':'stop'});assert result['actual_stop_reply']=={'result':'stopping'}
   result['appearance_actual_wait_exit']=scope.finish(child,min(close,now()+3_000_000_000));assert result['appearance_actual_wait_exit']==0
  except BaseException as error:result['stop_error']=repr(error)
 result['cleanup_errors']=scope.close(close);result['receipts']=scope.receipts
 result['closed']=not result.get('stop_error')and not result['cleanup_errors']and all(v['actual_direct_parent_wait']and v['stdout_EOF']and v['wait_before_original_cutoff']and v['EOF_before_original_cutoff']for v in scope.receipts)
 (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'passed':result['passed'],'closed':result['closed'],'error':result.get('error')}));raise SystemExit(0 if result['passed']and result['closed']else 1)
