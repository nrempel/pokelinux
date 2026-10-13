#!/usr/bin/env python3
"""Actual original ARM compositor registry/bind/context lifetime boundary."""
import array,hashlib,json,os,select,shutil,signal,socket,struct,subprocess,time,traceback
from pathlib import Path
R=Path('/results');S=json.loads((R/'selection.json').read_text());O=Path('/tmp/display-evidence');O.mkdir();children=[];logs=[];owned_fds=[]
result={'passed':False,'red_detected':False,'setup_complete':False,'groups':[],'cleanup':[],'hardware_qualified':False,'messages_actor_qualified':False,'messages_body_qualified':False,'PAM_qualified':False,'source_100ms_actor_qualified':False,'actor_safe_poll_qualified':False,'production_supervisor_qualified':False}
FORBIDDEN={'ext_session_lock_manager_v1','wp_security_context_manager_v1','zwlr_screencopy_manager_v1','zwp_virtual_keyboard_manager_v1','zwlr_layer_shell_v1'}
ALLOWED={'wl_compositor','xdg_wm_base'}
context=None;output_receive=None

def now():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def wait(fn,seconds=15):
    end=now()+int(seconds*1e9)
    while now()<end:
        try:
            if value:=fn():return value
        except (FileNotFoundError,ConnectionRefusedError):pass
        time.sleep(.005)
    raise TimeoutError('bounded original compositor condition')
def start(args,label,uid=0,env=None,pass_fds=(),stdin=None):
    log=(O/label).open('w');logs.append(log)
    p=subprocess.Popen(args,stdout=log,stderr=subprocess.STDOUT,user=uid,group=uid,extra_groups=os.getgrouplist('alarm',1000) if uid==1000 else [],env=env,start_new_session=True,pass_fds=pass_fds,stdin=stdin);children.append((p,label));return p
def command(args,label,uid=0,env=None):
    p=start(args,label,uid,env);wait(lambda:p.poll() is not None,60);assert p.returncode==0,(label,(O/label).read_text()[-1600:]);return (O/label).read_text()
def lines(label):
    path=O/label
    return [json.loads(line) for line in path.read_text().splitlines() if line.startswith('{')]
def event(label,name,after=0,seconds=6):
    return wait(lambda:next((v for v in lines(label)[after:] if v.get('event')==name),None),seconds)
def ctl(request):
    with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as c:
        c.settimeout(3);c.connect(str(control));assert struct.unpack('3i',c.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))==(compositor.pid,1000,1000)
        c.sendall((request if request.startswith('j/') else '/'+request).encode());out=b''
        while part:=c.recv(65536):out+=part;assert len(out)<131072
        return out.decode()
def record(name,**observed):result['groups'].append({'id':name,'passed':True,**observed});print('PASS '+name,flush=True)
def probe(path,label,forged=None,uid=11010,allowed=False):
    args=['python3',str(R/'source/registry-probe.py'),str(path)]
    if forged:args += ['--forged-name',str(forged['name']),'--forged-interface',forged['interface']]
    if allowed:args += ['--bind-allowed']
    return json.loads(command(args,label,uid))
def protected(values):
    names={v['interface'] for v in values}
    assert ALLOWED<=names,sorted(ALLOWED-names)
    assert not FORBIDDEN&names,sorted(FORBIDDEN&names)
def helper_command(value):
    before=len(lines('context.log'));context.stdin.write((value+'\n').encode());context.stdin.flush()
    response=event('context.log','stopped' if value=='stop' else 'status',before)
    assert response['state']==('stopped' if value=='stop' else 'ready'),response
    return response
def denied_original_path(path,label):
    script='import json,socket; s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)\ntry:\n s.connect('+repr(str(path))+')\nexcept OSError as e:\n print(json.dumps({"connected":False,"errno":e.errno}))\nelse:\n print(json.dumps({"connected":True}))\n'
    value=json.loads(command(['python3','-c',script],label,11010));assert not value['connected'] and value['errno']==13,value;return value
def future_listener_refused(path):
    with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as c:
        c.settimeout(.05)
        try:c.connect(str(path))
        except OSError as e:return {'errno':e.errno,'connected':False}
    return None

def qt_boundary(listener,peer_source_fd):
    """Normal production Qt only; no actor, body or session authorization claim."""
    stage=Path('/workspace/artifacts/native-messages/desktop/green-10');binary=stage/'pokelinux-messages'
    assert sha(binary)=='4aa50f980eab1100637d582a2752090f7545e1eb30630888753dfb9a7d2512c8'
    install=Path('/usr/libexec/pokelinux-display-fixture');install.mkdir(parents=True);shutil.copyfile(binary,install/'pokelinux-messages');(install/'pokelinux-messages').chmod(0o755)
    appearance=Path('/usr/share/pokelinux-display-fixture/appearance');appearance.mkdir(parents=True)
    for name in ['PhoneTheme.qml','ThemePalette.js']:shutil.copyfile(stage/'source/native-app'/name,appearance/name)
    home=Path('/var/lib/context-qt-app');runtime=Path('/run/user/11010')
    for directory in [home,runtime]:directory.mkdir(parents=True,exist_ok=True);directory.chmod(0o700);os.chown(directory,11010,11010)
    qt=start([str(install/'pokelinux-messages'),str(home),str(runtime),str(listener),'/home/alarm','/run/pokelinux-display-fixture-unavailable.sock',str(appearance)],'normal-qt.log',11010)
    qfd=os.pidfd_open(qt.pid,os.O_NONBLOCK);owned_fds.append(qfd)
    window=wait(lambda:next((v for v in json.loads(ctl('j/clients')) if v.get('pid')==qt.pid and v.get('mapped')),None),30)
    assert window['title']=='Messages' and not window['xwayland'];assert Path(f'/proc/{qt.pid}/status').read_text().split('Uid:\t')[1].split()[0]=='11010'
    # Independent original compositor API obtains actual socket credentials and pidfd.
    selected=Path('/workspace/artifacts/native-session-compositor/green-owned-source-final')
    assert sha(selected/'bins/probe.so')=='ac673f13ee21d1ea42b7d509e04703bac7bc40661c08d064172c4177d9a5e60b'
    shutil.copyfile(selected/'bins/probe.so',O/'probe.so');assert ctl('plugin load '+str(O/'probe.so')).strip()=='ok'
    facts=wait(lambda:(v if (v:=json.loads(ctl('nativebuildprobe')))['pid']==qt.pid else None),5)
    assert facts['uid']==facts['gid']==11010 and facts['eligible'] and facts['peer_pidfd_valid'] and facts['peer_pidfd_live']
    assert not select.select([qfd],[],[],0)[0]
    # Check original SCM_RIGHTS peer handle separately, without a fake producer or actor.
    assert sha(selected/'bins/adapter.so')=='312227400c39a793c2e591d0893c692d0ac258b8f5f9ad925ae2b8727fd53621'
    shutil.copyfile(selected/'bins/adapter.so',O/'adapter.so');time.sleep(.5);assert ctl('plugin load '+str(O/'adapter.so')).strip()=='ok'
    def focused_packet():
        try:data,anc,flags,_=peer_source_fd.recvmsg(1024,socket.CMSG_SPACE(12)+socket.CMSG_SPACE(4),socket.MSG_DONTWAIT|socket.MSG_CMSG_CLOEXEC)
        except BlockingIOError:return None
        if not data:raise RuntimeError('unexpected original source EOF')
        f=json.loads(data);rights=[];creds=[]
        try:
            for level,kind,body in anc:
                if level==socket.SOL_SOCKET and kind==socket.SCM_RIGHTS:
                    handles=array.array('i');handles.frombytes(body[:len(body)-len(body)%handles.itemsize]);rights.extend(handles)
                elif level==socket.SOL_SOCKET and kind==socket.SCM_CREDENTIALS:creds.append(struct.unpack('3i',body))
                else:raise ValueError('unexpected original source ancillary')
            assert not flags&(socket.MSG_TRUNC|socket.MSG_CTRUNC) and creds==[(compositor.pid,1000,1000)]
            assert len(rights)==int(f['focus'] is not None)
            if f['focus'] and f['focus']['pid']==qt.pid:
                fd=rights[0];assert os.readlink(f'/proc/self/fd/{fd}')=='anon_inode:[pidfd]';pid=int(next(v.split()[1] for v in Path(f'/proc/self/fdinfo/{fd}').read_text().splitlines() if v.startswith('Pid:')))
                assert pid==qt.pid and not select.select([fd],[],[],0)[0] and not select.select([qfd],[],[],0)[0]
                assert f['focus']=={'pid':qt.pid,'uid':11010,'gid':11010};assert now()<f['sampled_at_ns']+100_000_000
                return {'frame':f,'actual_received_socket_peer_pid':pid,'owned_app_pidfd_live':True}
            return None
        finally:
            for fd in rights:os.close(fd)
    transferred=wait(focused_packet,3);record('normal_production_Qt_window_through_filtered_listener_matches_actual_source_socket_peer',window={k:window[k] for k in ['pid','title','class','mapped','xwayland']},original_probe=facts,transferred=transferred)
    ctl('plugin unload '+str(O/'adapter.so'))
    result['normal_Qt_window_qualified']=True;result['normal_Qt_pid']=qt.pid;result['normal_Qt_elf_sha256']=sha(binary)
    return qt

try:
    assert os.getuid()==0 and os.uname().release=='7.1.6-1-aarch64-ARCH';assert [p.name for p in Path('/sys/class/net').iterdir()]==['lo']
    for name,digest in S['files'].items():assert sha(R/name)==digest
    assert sha('/usr/bin/Hyprland')==S['hyprland_sha256']
    result['kernel']=os.uname().release;result['namespaces']={n:os.readlink('/proc/self/ns/'+n) for n in ['pid','user','mnt']}
    command(['modprobe','virtio_gpu'],'modprobe.log');command(['udevadm','settle'],'udev.log');command(['usermod','-a','-G','video,input','alarm'],'groups.log');command(['useradd','-u','11010','-M','-s','/usr/bin/nologin','display-app'],'useradd.log')
    command(['install','-d','-o','alarm','-g','alarm','-m','700','/run/user/1000'],'runtime.log')
    start(['seatd','-g','video'],'seatd.log',env=dict(os.environ,SEATD_VTBOUND='0'))
    config=O/'hypr.lua';config.write_text('hl.monitor({ output = "", mode = "320x240@60", position = "auto", scale = 1 })\nhl.config({ animations = { enabled = false }, decoration = { blur = { enabled = false }, shadow = { enabled = false } }, misc = { disable_hyprland_logo = true, disable_splash_rendering = true, allow_session_lock_restore = true } })\n')
    env={'HOME':'/home/alarm','XDG_RUNTIME_DIR':'/run/user/1000','LIBSEAT_BACKEND':'seatd','AQ_NO_MODIFIERS':'1','XDG_SESSION_TYPE':'wayland','PATH':'/usr/local/bin:/usr/bin'}
    output_receive,output_send=socket.socketpair(socket.AF_UNIX,socket.SOCK_SEQPACKET);output_receive.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1);output_receive.setblocking(False)
    env.update(POKELINUX_SESSION_FD=str(output_send.fileno()),POKELINUX_SESSION_SOURCE=os.urandom(16).hex())
    compositor=start(['/usr/bin/Hyprland','--config',str(config)],'compositor.log',1000,env,pass_fds=(output_send.fileno(),));output_send.close()
    source_alive=os.pidfd_open(compositor.pid,os.O_NONBLOCK);owned_fds.append(source_alive)
    wayland=wait(lambda:next((p for p in Path('/run/user/1000').glob('wayland-*') if p.is_socket()),None),90)
    control=wait(lambda:next(Path('/run/user/1000/hypr').glob('*/.socket.sock'),None),90)
    result['source']={'pid':compositor.pid,'uid':1000,'gid':1000,'launcher_retained_source_pidfd':True}
    if S['phase']=='red':
        command(['setfacl','-m','u:11010:--x','/run/user/1000'],'direct-parent-acl.log');command(['setfacl','-m','u:11010:rw-',str(wayland)],'direct-socket-acl.log')
        value=probe(wayland,'direct-app-registry.log');names={g['interface'] for g in value['globals']};assert ALLOWED|{'ext_session_lock_manager_v1','wp_security_context_manager_v1'}<=names
        result['setup_complete']=True;result['direct_app_registry']=value;present=sorted(FORBIDDEN&names);result['groups'].append({'id':'installed_app_cannot_advertise_privileged_wayland_globals','passed':not present,'observed_privileged_globals':present});assert present;result['red_detected']=True
    else:
        unrestricted=probe(wayland,'trusted-original-registry.log',uid=1000);names={g['interface'] for g in unrestricted['globals']};assert FORBIDDEN|ALLOWED<=names
        original={g['interface']:g for g in unrestricted['globals']};result['trusted_original_registry']=unrestricted
        denied=denied_original_path(wayland,'app-original-path-denied.log');record('distinct_UID_app_has_no_direct_original_runtime_or_socket_access',observed=denied)
        parent=Path('/run/pokelinux-display');parent.mkdir();parent.chmod(0o750);os.chown(parent,0,11010);listener=parent/'messages-wayland.sock'
        transferred_watch=os.dup(source_alive);os.set_inheritable(transferred_watch,False)
        try:context=start([str(R/'bins/display-context-fixture'),str(wayland),str(listener),'org.omarchy.Messages',os.urandom(16).hex(),'11010',str(compositor.pid),'1000',str(transferred_watch)],'context.log',env=dict(os.environ,NATIVE_DISPLAY_ISOLATED='1'),pass_fds=(transferred_watch,),stdin=subprocess.PIPE)
        finally:os.close(transferred_watch)
        ready=event('context.log','ready',seconds=7);assert ready['state']=='ready' and ready['protocol_creator_disconnected'] is True;assert context.poll() is None
        st=parent.stat();assert st.st_uid==0 and st.st_gid==11010 and st.st_mode&0o777==0o750
        stat=listener.stat();assert stat.st_uid==0 and stat.st_gid==11010 and not stat.st_mode&0o007
        result['setup_complete']=True;record('native_Rust_context_commit_acknowledged_and_setup_transport_disconnected',ready=ready,listener_mode=oct(stat.st_mode&0o777),parent_mode=oct(st.st_mode&0o777))
        first=probe(listener,'filtered-first.log',allowed=True);protected(first['globals']);assert first['uid']==first['gid']==11010;record('allowed_globals_bind_and_privileged_globals_absent',allowed_bound=first['allowed_bound'],globals=first['globals'])
        for interface in ['ext_session_lock_manager_v1','wp_security_context_manager_v1','zwlr_screencopy_manager_v1']:
            forged=probe(listener,'forged-'+interface+'.log',original[interface]);protected(forged['globals']);assert forged['forged_bind_error'] is not None and not forged['forged_bind_completed'];record('forged_actual_original_global_ID_refused_at_bind_'+interface,error=forged['forged_bind_error'],actual_original_global=original[interface])
        future=probe(listener,'filtered-future-after-creator-disconnect.log',allowed=True);protected(future['globals']);record('future_filtered_client_connects_after_protocol_creator_disconnect',allowed_bound=future['allowed_bound'])
        # Normal production app is separate from registry/lifetime admission.
        qt=None
        try:qt=qt_boundary(listener,output_receive)
        except BaseException as error:
            result['normal_Qt_window_qualified']=False;result['normal_Qt_gate_error']=str(error);result['normal_Qt_gate_traceback']=traceback.format_exc();print('OPEN normal Qt containment gate: '+str(error),flush=True)
        retained=start(['python3',str(R/'source/registry-probe.py'),str(listener),'--interactive'],'retained-client.log',11010,stdin=subprocess.PIPE)
        accepted=event('retained-client.log','ready');protected(accepted['globals']);assert accepted['pid']==retained.pid;record('retained_actual_accepted_client_is_filtered_and_allowed',pid=retained.pid)
        before=len(lines('context.log'));status=helper_command('status');record('native_owner_status_ready_after_setup_connection_was_dropped',status=status)
        stopped=helper_command('stop');wait(lambda:context.poll() is not None,3);assert context.returncode==0;record('native_owner_stopped_and_direct_child_reaped',stopped=stopped,returncode=context.returncode)
        refusal=wait(lambda:future_listener_refused(listener),3);assert refusal['errno'] in [2,13,111];record('close_pipe_stop_prevents_future_listener_connections',observed=refusal)
        before=len(lines('retained-client.log'));retained.stdin.write(b'{"op":"registry"}\n');retained.stdin.flush();existing=event('retained-client.log','registry',before);protected(existing['globals']);assert retained.poll() is None;record('existing_accepted_client_remains_filtered_after_creator_and_listener_close',pid=retained.pid,globals=existing['globals'])
        known=original['ext_session_lock_manager_v1'];before=len(lines('retained-client.log'));retained.stdin.write((json.dumps({'op':'forge','name':known['name'],'interface':known['interface']})+'\n').encode());retained.stdin.flush();rejected=event('retained-client.log','forged',before);assert rejected['forged_bind_error'] is not None and not rejected['forged_bind_completed'];wait(lambda:retained.poll() is not None,3);assert retained.returncode==0;record('existing_accepted_client_privileged_bind_still_refused_after_close',error=rejected['forged_bind_error'])
        if qt is not None:
            assert qt.poll() is None and any(v['pid']==qt.pid for v in json.loads(ctl('j/clients')));record('existing_normal_Qt_window_survives_context_creator_and_listener_close',pid=qt.pid)
        result['passed']=True
except BaseException as e:result['error']=str(e);result['traceback']=traceback.format_exc();print(result['traceback'],flush=True)
finally:
    if context is not None and context.poll() is None:
        try:helper_command('stop')
        except BaseException:pass
    for p,label in reversed(children):
        signals=[]
        try:
            if p.poll() is None:
                os.killpg(p.pid,signal.SIGTERM);signals.append('SIGTERM')
                try:p.wait(timeout=4)
                except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);signals.append('SIGKILL');p.wait(timeout=3)
            p.wait(timeout=1);result['cleanup'].append({'pid':p.pid,'role':label,'signals':signals,'reaped':True,'returncode':p.returncode})
        except BaseException as e:result.setdefault('cleanup_errors',[]).append(str(e))
    if output_receive is not None:output_receive.close()
    for fd in owned_fds:
        try:os.close(fd)
        except OSError:pass
    for f in logs:f.close()
    result['all_children_reaped']=not result.get('cleanup_errors') and len(result['cleanup'])==len(children);result['passed']=result['passed'] and result['all_children_reaped']
    (O/'result.json').write_text(json.dumps(result,indent=2)+'\n');shutil.copytree(O,R,dirs_exist_ok=True);subprocess.run(['sync'],timeout=10);subprocess.run(['systemctl','poweroff'],timeout=15)
