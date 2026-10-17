#!/usr/bin/env python3
"""Frozen two-case actual PID1 installed-service contract; no production imports.
start-stop is used before installation (RED) and after staging. cold-autostart
never calls start: it requires PID1's enabled-at-boot invocation before IPC/Stop.
"""
import argparse,json,os,re,select,socket,struct,time
from pathlib import Path
from owned import Scope,now

UNITS=[('appearance','pokelinux-appearance.service','/usr/libexec/pokelinux/pokelinux-appearance',0),
       ('audio','pokelinux-audio.service','/usr/libexec/pokelinux/pokelinux-audio',11040)]
FIELDS='Id,LoadState,UnitFileState,ActiveState,SubState,MainPID,ControlGroup,ExecMainCode,ExecMainStatus,Result,InvocationID'

def control(scope,until):
    assert os.getresuid()==(0,0,0) and Path('/proc/1/comm').read_text().strip()=='systemd','genuine root/PID1 systemd required'
    code,body=scope.run(['/usr/bin/pacman','-Q'],'control-packages.log',min(until,now()+10_000_000_000),limit=65536)
    assert code==0,'actual original package inventory command'
    rows=dict(line.split(' ',1) for line in body.decode().splitlines())
    expected=dict(line.split(' ',1) for line in (Path(__file__).parent/'expected-installed353.txt').read_text().splitlines())
    assert len(rows)==len(expected)==353 and rows==expected,'exact original353 package cohort required'
    return {'actual_PID1_comm':'systemd','actual_root_UIDs':list(os.getresuid()),'original353_package_map_matches':True,'package_count':353}

def main():
    p=argparse.ArgumentParser();p.add_argument('--mode',choices=['start-stop','cold-autostart'],default='start-stop');p.add_argument('--control-only',action='store_true');p.add_argument('--output',type=Path,required=True);p.add_argument('--until-ns',type=int,required=True);a=p.parse_args()
    logs=a.output.with_suffix('.logs');logs.mkdir(mode=0o700);scope=Scope(logs)
    result={'schema':1,'mode':a.mode,'control_only':a.control_only,'passed':False,'cases':[],'original_outer_until_ns':a.until_ns,'normal_GUI_Source_or_hardware_claimed':False};serial=0
    def command(argv,label,until):
        nonlocal serial
        serial+=1;return scope.run(argv,f'{serial:03d}-{label}.log',until,limit=65536)
    def props(unit,until):
        code,body=command(['/usr/bin/systemctl','show',unit,'--no-pager','--property='+FIELDS],'show-'+unit,until)
        values=dict(line.split('=',1) for line in body.decode().splitlines() if '=' in line)
        return {'exit':code,'values':values}
    def ipc(path,packet,kind,pid,uid,until):
        assert now()<until,'original service IPC cutoff'
        with socket.socket(socket.AF_UNIX,kind) as s:
            s.settimeout(min(1,(until-now())/1e9));s.connect(path)
            peer=struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert peer==(pid,uid,uid),'IPC must belong to actual unit Main'
            payload=json.dumps(packet,separators=(',',':')).encode()+(b'\n' if kind==socket.SOCK_STREAM else b'');s.sendall(payload)
            body=bytearray()
            while True:
                block=s.recv(65537-len(body));assert block,'IPC closed without complete reply';body.extend(block);assert len(body)<=65536,'bounded copied service reply'
                if kind==socket.SOCK_SEQPACKET or body.endswith(b'\n'):break
            assert now()<until,'expired service reply'
            return json.loads(body)
    try:
        result['control']=control(scope,a.until_ns)
        if a.control_only:result['passed']=True
        else:
            for role,unit,exe,uid in UNITS:
                until=min(a.until_ns,now()+30_000_000_000);row={'role':role,'unit':unit,'original_until_ns':until,'passed':False};fds=[];mainpid=None;loaded=False
                try:
                    initial=props(unit,until);row['initial_show']=initial;loaded=initial['values'].get('LoadState')=='loaded'
                    if a.mode=='start-stop':
                        code,body=command(['/usr/bin/systemctl','start',unit],'actual-start-'+unit,until);row['actual_start']={'exit':code,'bounded_output':body.decode()}
                    # Both cases execute before this program returns failure. This is
                    # the genuine original-image deployment RED, not a setup error.
                    assert loaded,'required installed unit LoadState=loaded; observed '+str(initial['values'].get('LoadState'))
                    if a.mode=='start-stop':assert row['actual_start']['exit']==0,'actual systemctl start must succeed'
                    else:assert initial['values'].get('UnitFileState')=='enabled','cold mode requires actual installed enablement'
                    current=props(unit,until)['values'];assert current['ActiveState']=='active' and current['SubState']=='running','independently active process required'
                    mainpid=int(current['MainPID']);assert mainpid>1 and re.fullmatch('[0-9a-f]{32}',current['InvocationID']);row['active_identity']=current
                    mainfd=os.pidfd_open(mainpid,os.O_NONBLOCK);fds.append((mainpid,mainfd));assert not select.select([mainfd],[],[],0)[0],'actual Main already dead'
                    status=Path(f'/proc/{mainpid}/status').read_text();ppid=int(re.search(r'^PPid:\s+(\d+)$',status,re.M)[1]);uids=list(map(int,re.search(r'^Uid:\s+(.+)$',status,re.M)[1].split()));assert ppid==1 and uids==[uid]*4,'PID1-owned actual service UID identity'
                    assert os.readlink(f'/proc/{mainpid}/exe')==exe,'actual installed executable required'
                    group=current['ControlGroup'];assert group.startswith('/system.slice/') and '..' not in Path(group).parts
                    assert Path(f'/proc/{mainpid}/cgroup').read_text().strip()=='0::'+group,'actual unit cgroup identity'
                    # Domain readiness is separate from Type=exec/active. No health
                    # or Source availability is inferred from process admission.
                    while True:
                        assert now()<until,'one original per-service readiness/Stop cutoff'
                        assert not select.select([mainfd],[],[],0)[0],'Main died before IPC readiness'
                        try:
                            if role=='appearance':
                                snap=ipc('/run/pokelinux/appearance.sock',{'op':'snapshot'},socket.SOCK_SEQPACKET,mainpid,uid,until)
                                state=ipc('/run/pokelinux/appearance-control.sock',{'op':'status'},socket.SOCK_SEQPACKET,mainpid,uid,until)
                                assert set(snap)=={'schema','owner','revision','until_ns','available','appearance'} and snap['schema']==3 and snap['available'] is False and snap['appearance'] is None
                                assert re.fullmatch('[0-9a-f]{32}',snap['owner']) and re.fullmatch('[1-9][0-9]*',snap['until_ns']) and int(snap['until_ns'])>now()
                                ready=state['completed']>=1 and state['state']=='idle' and state['worker_pid'] is None and state['available'] is False
                                observation={'snapshot':snap,'control':state}
                            else:
                                state=ipc('/run/pokelinux-audio/audio.sock',{'op':'status'},socket.SOCK_STREAM,mainpid,uid,until)
                                assert state['ok'] is True and state['phase']=='running' and state['media_routes']==[{'id':'vm-speaker','label':'Virtual speaker'}]
                                ready=state['observed']['revision']>=1 and state['observed']['available'] is False and state['observed']['links']==[] and state['application']!='observed'
                                observation={'status':state,'PipeWire_health_claimed':False}
                            if ready:row['IPC_readiness']=observation;break
                        except (ConnectionError,FileNotFoundError,socket.timeout):pass
                        time.sleep(.01)
                    cg=Path('/sys/fs/cgroup'+group)
                    if (cg/'cgroup.procs').exists():
                        for raw in (cg/'cgroup.procs').read_text().split():
                            pid=int(raw)
                            if pid!=mainpid:
                                try:fds.append((pid,os.pidfd_open(pid,os.O_NONBLOCK)))
                                except ProcessLookupError:pass
                    code,body=command(['/usr/bin/systemctl','stop',unit],'actual-stop-'+unit,until);assert code==0,'actual Stop command'
                    final=props(unit,until)['values'];row['final_show']=final
                    assert final['ActiveState']=='inactive' and final['SubState']=='dead' and final['MainPID']=='0' and final['Result']=='success' and final['ExecMainCode']=='1' and final['ExecMainStatus']=='0','PID1 actual clean Main outcome required'
                    assert not cg.exists() or (cg/'cgroup.events').read_text().split('populated ')[1].splitlines()[0]=='0','actual empty unit cgroup required'
                    for pid,fd in fds:assert select.select([fd],[],[],0)[0] and not Path(f'/proc/{pid}').exists(),'retained service/child death and proc absence'
                    row['PID1_Main_wait_exit']=0;row['guest_waitpid_for_PID1_Main_claimed']=False;row['actual_cgroup_empty']=True;row['passed']=True
                except AssertionError as e:
                    row['failure']='AssertionError: '+str(e);row['failure_kind']='assertion'
                except BaseException as e:
                    row['failure']=type(e).__name__+': '+str(e);row['failure_kind']='unexpected'
                finally:
                    if loaded and not row['passed']:
                        try:
                            code,_=command(['/usr/bin/systemctl','stop',unit],'failure-stop-'+unit,min(a.until_ns,until));row['failure_Stop_exit']=code
                        except BaseException as e:row['failure_cleanup_error']=repr(e)
                    for _,fd in fds:os.close(fd)
                    row['completed_ns']=now();result['cases'].append(row)
                    print(('PASS ' if row['passed'] else 'FAIL ')+unit+': '+row.get('failure','independent IPC and PID1 Stop'),flush=True)
            result['passed']=len(result['cases'])==2 and all(r['passed'] for r in result['cases'])
    except BaseException as e:result['setup_error']=type(e).__name__+': '+str(e)
    finally:
        result['cleanup_errors']=scope.close(a.until_ns);result['owned_command_receipts']=scope.receipts;result['completed_ns']=now();result['passed']=bool(result['passed'] and not result['cleanup_errors'] and now()<a.until_ns)
        a.output.write_text(json.dumps(result,indent=2)+'\n')
    return 0 if result['passed'] else 1
if __name__=='__main__':raise SystemExit(main())
