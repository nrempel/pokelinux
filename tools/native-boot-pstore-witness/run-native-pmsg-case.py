#!/usr/bin/env python3
"""One owned native-PID1 PMSG case; fixed post-PowerOff QMP profile."""
import argparse,errno,fcntl,hashlib,importlib.util,json,os,re,socket,stat,struct,subprocess,time,traceback
from pathlib import Path
ROOT=Path(__file__).resolve().parent
DOCKER=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock']
def sha(p):
    with Path(p).open('rb')as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def save(p,value):
    with p.open('x')as stream:json.dump(value,stream,indent=2);stream.write('\n')
def absent(row,identity):return row.returncode==1 and any(text+identity in row.stderr for text in ['No such object: ','No such container: '])
def ticks(pid):return Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()[19]
def rom_preflight(text,selection):
    assert isinstance(text,str)and len(text)<=32768
    rows=[];fw_regions=[]
    for line in text.splitlines():
        if not line.strip():continue
        m=re.fullmatch(r'addr=(?:0x)?([0-9a-fA-F]+) size=(?:0x)?([0-9a-fA-F]+) mem=(\S+) name="([^"]+)"',line.strip())
        if not m:
            fw=re.fullmatch(r'(/rom@etc/(?:acpi/tables|table-loader|acpi/rsdp)) size=0x([0-9a-fA-F]+) name="([^"]+)"',line.strip())
            assert fw,'unrecognized original info roms row: '+line
            fw_regions.append({'region':fw[1],'bytes':int(fw[2],16),'name':fw[3]})
            continue
        start,size=int(m[1],16),int(m[2],16);assert size>0
        rows.append({'start':start,'bytes':size,'end_exclusive':start+size,'memory':m[3],'name':m[4]})
    assert fw_regions==[{'region':'/rom@etc/acpi/tables','bytes':0x200000,'name':'etc/acpi/tables'},{'region':'/rom@etc/table-loader','bytes':0x10000,'name':'etc/table-loader'},{'region':'/rom@etc/acpi/rsdp','bytes':0x1000,'name':'etc/acpi/rsdp'}]
    by_name={r['name']:r for r in rows};assert len(rows)==len(by_name)
    kernel=by_name['/inputs/Image'];initrd=by_name['/inputs/generic.lz4'];dtb=by_name['dtb']
    # Original QEMU10 arm/boot.c: Image text_offset0 shifts by2MiB
    # to avoid the reset trampoline; initrd at128MiB; DTB aligned2MiB.
    assert kernel['start']==0x40200000 and kernel['bytes']==selection['kernel_raw_bytes']
    effective_end=kernel['start']+selection['kernel_effective_memory_bytes']
    assert initrd['start']==0x48000000 and initrd['bytes']==selection['generic_LZ4_bytes']
    expected_dtb=(initrd['end_exclusive']+0x1fffff)&~0x1fffff
    assert dtb['start']==expected_dtb
    reserve=selection['ramoops_reserved_start']
    assert effective_end<reserve
    ramrows=[r for r in rows if 0x40000000<=r['start']<0x50000000]
    assert len(ramrows)>=4 and all(r['end_exclusive']<reserve for r in ramrows)
    assert all(r['end_exclusive']<=0x40000000 or r['start']>=0x50000000 or r in ramrows for r in rows)
    return {'passed':True,'raw_original_info_roms':text,'all_addressed_ROM_rows':rows,'addressless_fw_cfg_regions':fw_regions,'fw_cfg_scope':'Original loader.c rom_set_mr creates separate host-backed resizeable MemoryRegions without mapping them into guest physical RAM; rom_reset writes their retained MR pointers separately. Only the exact three observed firmware-table regions accepted.','all_RAM_boot_blobs_end_below_reserved_start':True,'kernel_effective_memory_end_exclusive':effective_end,'reserved_start':reserve,'reserved_end_exclusive':0x50000000,'source_based_initrd_start':0x48000000,'source_based_DTB_start':expected_dtb,'actual_RAM_boot_blob_count':len(ramrows)}
def main():
    parser=argparse.ArgumentParser();parser.add_argument('--selection-sha256',required=True);parser.add_argument('--command-sha256',required=True);args=parser.parse_args()
    assert sha(ROOT/'selection.json')==args.selection_sha256 and sha(ROOT/'command.json')==args.command_sha256
    s=json.loads((ROOT/'selection.json').read_text());cmd=json.loads((ROOT/'command.json').read_text())
    assert s['runtime_started']is False and s['mode']=='native-pid1-post-poweroff-pmsg-tracer'and s['production_ELF_source_and_build_bound']
    assert cmd[:len(DOCKER)]==DOCKER and cmd[cmd.index('--cpus')+1]=='2'and cmd[cmd.index('--memory')+1]=='512m'
    assert '--device'not in cmd and '--privileged'not in cmd and '--read-only'in cmd and cmd[cmd.index('--cap-drop')+1]=='ALL'
    assert cmd[cmd.index('--security-opt')+1]=='no-new-privileges'and cmd[cmd.index('--network')+1]=='none'and cmd[cmd.index('--user')+1]=='1000:1000'
    assert cmd[cmd.index('--pids-limit')+1]=='64'and cmd[cmd.index('-smp')+1]=='2'and cmd[cmd.index('-m')+1]=='256'
    assert '-S'in cmd and '-no-shutdown'in cmd and '-no-reboot'not in cmd and cmd[cmd.index('-nic')+1]=='none'
    assert cmd[cmd.index('-append')+1]==s['test_cmdline_append']
    assert sha(__file__)==s['local_inputs']['run-frozen.py']
    assert not any((ROOT/name).exists()for name in ['result.json','execution.json','console.log','owned-container-id','runtime-output.lock'])
    def closure():
        assert sha(ROOT/'selection.json')==args.selection_sha256 and sha(ROOT/'command.json')==args.command_sha256
        for relative,digest in s['local_inputs'].items():assert sha(ROOT/relative)==digest,relative
        for path,digest in s['external_inputs'].items():assert sha(path)==digest,path
    spec=importlib.util.spec_from_file_location('fixed_witness_observer',ROOT/'source/observer.py');observer=importlib.util.module_from_spec(spec);spec.loader.exec_module(observer)
    spec=importlib.util.spec_from_file_location('fixed_native_qmp_profile',ROOT/'source/qmp-profile.py');profile_module=importlib.util.module_from_spec(spec);spec.loader.exec_module(profile_module)
    assert s['case_id'] in observer.NATIVE_CASES
    e={'schema':1,'passed':False,'runtime_closure_passed':False,'selection_sha256':args.selection_sha256,'command_sha256':args.command_sha256,'runner_sha256':sha(__file__),'actual_Docker_CLI_wait':False,'exact_CID_absence':False,'timed_out':False,'process_waits':[],'owned_inspections':[],'qmp_commands':[],'qmp_records':[]}
    def call(argv,timeout=5):
        start=time.monotonic_ns();p=subprocess.run(argv,capture_output=True,text=True,timeout=timeout)
        e['process_waits'].append({'argv':argv,'exit':p.returncode,'stdout':p.stdout[:8192],'stderr':p.stderr[:2048],'started_monotonic_ns':start,'completed_monotonic_ns':time.monotonic_ns(),'actual_direct_wait':True});return p
    name=cmd[cmd.index('--name')+1];assert name==ROOT.name;e['owned_container_name']=name
    child=None;cid=None;state=None;qmp=None;peer_pidfd=None;buffer=b'';sent=set();done=set();greeting=False;continued=False;error=None;daemon=None;profile=None;pipe_eof=False;qmp_terminal=False
    with(ROOT/'runtime-output.lock').open('xb')as outputlock,Path(s['shared_readonly_input_lock']).open('rb')as inputlock:
        fcntl.flock(outputlock,fcntl.LOCK_EX|fcntl.LOCK_NB);fcntl.flock(inputlock,fcntl.LOCK_SH|fcntl.LOCK_NB)
        closure();e['all_inputs_before_match']=True
        def owned_inspect():
            observed=call(DOCKER+['container','inspect','--format','{"id":{{json .Id}},"name":{{json .Name}},"image":{{json .Config.Image}},"state":{{json .State}}}',cid],2)
            e['owned_inspections'].append({'identity':cid,'exit':observed.returncode,'stdout':observed.stdout[:4096],'stderr':observed.stderr[:2048],'actual_direct_wait':True})
            if observed.returncode!=0:
                assert absent(observed,cid),'untyped owned inspect failure'
                return None
            row=json.loads(observed.stdout);assert row['id']==cid and row['name']=='/'+name and row['image']==s['QEMU_image']
            return row['state']
        def current_peer():
            live=owned_inspect();assert live and live['Running']and live['Pid']==state['Pid']
            got=struct.unpack('3i',qmp.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))
            assert got==(state['Pid'],1000,1000)and ticks(got[0])==e['qmp_peer']['start_ticks']
            e.setdefault('authenticated_QMP_mutation_checks',[]).append({'peer_PID':got[0],'uid':got[1],'gid':got[2],'same_retained_pidfd_and_start_ticks':True,'monotonic_ns':time.monotonic_ns()})
        def send(execute,identity,arguments=None):
            assert time.monotonic()<outer_until and identity not in sent and not qmp_terminal
            allowed={'caps':'qmp_capabilities','status':'query-status','roms':'human-monitor-command','cont-first':'cont','status-shutdown':'query-status','reset':'system_reset','status-reset':'query-status','cont-second':'cont','quit':'quit'}
            assert allowed[identity]==execute
            if identity=='roms':assert arguments=={'command-line':'info roms'}
            else:assert arguments is None
            if execute in ('cont','system_reset','quit'):current_peer()
            request={'execute':execute,'id':identity}
            if arguments is not None:request['arguments']=arguments
            qmp.sendall(json.dumps(request).encode()+b'\r\n');sent.add(identity)
            stamp=time.monotonic_ns();assert stamp<e['original_outer_until_monotonic_ns']
            e['qmp_commands'].append({'request':request,'sent_monotonic_ns':stamp})
        def receive(message):
            nonlocal continued
            stamp=time.monotonic_ns();assert stamp<e['original_outer_until_monotonic_ns']
            e['qmp_records'].append({'received_monotonic_ns':stamp,'message':message});assert len(e['qmp_records'])<=96
            profile.receive(message)
            continued=profile.initial_cont_completed
            e['qmp_initial_status']=profile.initial_status
            e['qmp_shutdown_status']=profile.shutdown_status
            e['qmp_reset_status']=profile.reset_status
            e['boot_blob_preflight']=profile.boot_blob_preflight
        def finish_qmp(kind):
            nonlocal qmp_terminal
            assert not qmp_terminal
            assert profile.quit_replied and 'quit' in profile.done, 'early QMP closure without successful owned quit reply'
            assert profile.guest_shutdowns==profile.boots, 'early QMP closure without required guest shutdowns'
            assert not buffer.strip(), 'QMP closed with incomplete JSON tail'
            stamp=time.monotonic_ns();assert stamp<e['original_outer_until_monotonic_ns']
            e['qmp_terminal']={'kind':kind,'observed_monotonic_ns':stamp,'successful_quit_reply':True,'actual_guest_SHUTDOWN_count':profile.guest_shutdowns,'required_guest_SHUTDOWN_count':profile.boots}
            qmp_terminal=True
        def receive_qmp():
            nonlocal buffer
            assert qmp is not None and not qmp_terminal
            try:data=qmp.recv(16384)
            except (BlockingIOError,socket.timeout):return False
            except ConnectionResetError as failure:
                if failure.errno!=errno.ECONNRESET:raise
                finish_qmp('ECONNRESET');return False
            if not data:
                finish_qmp('EOF');return False
            buffer+=data;assert len(buffer)<=65536
            while b'\n'in buffer:
                line,buffer=buffer.split(b'\n',1)
                if line.strip():receive(json.loads(line))
            return True
        def drain_console(log):
            nonlocal pipe_eof
            assert child is not None and child.stdout is not None
            while not pipe_eof:
                try:data=os.read(child.stdout.fileno(),16384)
                except BlockingIOError:break
                if not data:
                    pipe_eof=True;e['merged_stdout_stderr_EOF_observed_monotonic_ns']=time.monotonic_ns();break
                log.write(data);log.flush()
                assert log.tell()<=1024*1024
        profile=profile_module.NativePostPowerOff(s['case_id'],observer,send,lambda text:rom_preflight(text,s))
        try:
            prior=call(DOCKER+['container','inspect',name]);assert absent(prior,name)
            row=call(DOCKER+['info','--format','{{.ID}}'],10);assert row.returncode==0;daemon=row.stdout.strip();assert daemon;e['daemon_before']=daemon
            row=call(DOCKER+['image','inspect','--format','{{.Id}}',s['QEMU_image']],10);assert row.returncode==0 and row.stdout.strip()==s['QEMU_image']
            started=time.monotonic();setup_until=started+s['QMP_setup_deadline_seconds'];outer_until=started+s['outer_deadline_seconds'];e['started_monotonic_ns']=time.monotonic_ns();e['original_outer_until_monotonic_ns']=int(outer_until*1_000_000_000);next_inspect=0
            with(ROOT/'console.log').open('xb')as log:
                child=subprocess.Popen(cmd,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);os.set_blocking(child.stdout.fileno(),False);e['owned_Docker_CLI_PID']=child.pid
                save(ROOT/'live-handle.json',{'owned_Docker_CLI_PID':child.pid,'owned_container_name':name,'selection_sha256':args.selection_sha256,'command_sha256':args.command_sha256})
                while child.poll()is None:
                    drain_console(log)
                    now=time.monotonic()
                    if now>=outer_until:e['timed_out']=True;raise TimeoutError('original45s RAM-only wrapper deadline')
                    if not continued and now>=setup_until:raise TimeoutError('bounded5s owned paused-QMP setup prerequisite')
                    cp=ROOT/'owned-container-id'
                    if cp.exists():
                        value=cp.read_text().strip()
                        if re.fullmatch('[0-9a-f]{64}',value):
                            if cid is None:cid=value;e['exact_owned_CID']=cid
                            assert value==cid
                    if cid and state is None and now>=next_inspect:
                        observed=owned_inspect()
                        if observed and observed['Running']and observed['Pid']>1:state=observed;e['exact_owned_running_container_confirmed']=True
                        next_inspect=time.monotonic()+.05
                    qp=ROOT/'runtime-output/qmp.sock'
                    if state and qmp is None and qp.exists():
                        meta=qp.lstat();assert stat.S_ISSOCK(meta.st_mode)and meta.st_uid==1000
                        qmp=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);qmp.settimeout(.1);qmp.connect(str(qp))
                        p,u,g=struct.unpack('3i',qmp.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert(p,u,g)==(state['Pid'],1000,1000)
                        peer_pidfd=os.pidfd_open(p);e['qmp_peer']={'actual_SO_PEERCRED_PID':p,'uid':u,'gid':g,'start_ticks':ticks(p),'retained_pidfd_opened':True};qmp.setblocking(False)
                        save(ROOT/'owned-runtime-handle.json',{'exact_owned_CID':cid,'owned_Docker_CLI_PID':child.pid,'owned_QEMU_host_PID':p,'same_Docker_daemon':daemon})
                    if qmp and not qmp_terminal:
                        receive_qmp()
                    if profile.initial_cont_completed:
                        console=(ROOT/'console.log').read_bytes().decode('utf-8').replace('\r\n','\n')
                        profile.drive(console)
                    time.sleep(.005)
                e['Docker_CLI_exit']=child.wait();e['actual_Docker_CLI_wait']=True;e['Docker_CLI_waited_monotonic_ns']=time.monotonic_ns();assert e['Docker_CLI_waited_monotonic_ns']<e['original_outer_until_monotonic_ns']
                while not pipe_eof and time.monotonic()<outer_until:
                    drain_console(log)
                    if not pipe_eof:time.sleep(.005)
                assert pipe_eof and e['merged_stdout_stderr_EOF_observed_monotonic_ns']<e['original_outer_until_monotonic_ns']
                child.stdout.close()
                if qmp and not qmp_terminal:
                    qmp.settimeout(.05)
                    while not qmp_terminal:
                        if not receive_qmp():break
        except BaseException as exception:
            error=type(exception).__name__+': '+str(exception);e['traceback']=traceback.format_exc()
            if child:
                if cid and child.poll()is None:
                    row=call(DOCKER+['kill',cid]);e['owned_exact_CID_kill']={'exit':row.returncode,'stdout':row.stdout[:2048],'stderr':row.stderr[:2048]}
                try:e['Docker_CLI_exit']=child.wait(timeout=5)
                except subprocess.TimeoutExpired:child.kill();e['Docker_CLI_exit']=child.wait(timeout=5);e['owned_CLI_killed_only_no_container_cleanup_inference']=True
                e['actual_Docker_CLI_wait']=True
        finally:
            if child and child.stdout and not child.stdout.closed:child.stdout.close()
            if qmp:qmp.close()
            if peer_pidfd is not None:os.close(peer_pidfd);e['retained_peer_pidfd_closed']=True
            if error:e['error']=error
            e['completed_monotonic_ns']=time.monotonic_ns()
            try:
                assert cid is not None
                gone=call(DOCKER+['container','inspect',cid]);e['exact_CID_absence']=absent(gone,cid)
                e['exact_CID_absence_inspect']={'identity':cid,'exit':gone.returncode,'stderr':gone.stderr[:2048],'actual_direct_wait':True}
                row=call(DOCKER+['info','--format','{{.ID}}'],10);assert row.returncode==0;e['same_daemon_before_after']=row.stdout.strip()==daemon
                closure();e['all_original_and_selected_inputs_after_match']=True
            except BaseException as failure:e['closure_error']=type(failure).__name__+': '+str(failure)
            e['completed_monotonic_ns']=time.monotonic_ns()
            e['owned_disposal_and_input_closure_before_original_cutoff']=e['completed_monotonic_ns']<e.get('original_outer_until_monotonic_ns',0)
            e['runtime_closure_passed']=bool(e.get('actual_Docker_CLI_wait')and e.get('exact_CID_absence')and e.get('same_daemon_before_after')and e.get('all_original_and_selected_inputs_after_match')and e['owned_disposal_and_input_closure_before_original_cutoff'])
            console=(ROOT/'console.log').read_text(errors='replace')if(ROOT/'console.log').exists()else''
            try:
                observed=observer.inspect_native(console,e['qmp_records'],e['qmp_commands'],s['case_id'])
                for key in ['retained_pmsg','retained_console','expected_first_native_pmsg']:
                    payload=observed.pop(key)
                    with(ROOT/(key+'.bin')).open('xb')as stream:stream.write(payload)
                e['witness_observer_passed']=True
            except BaseException as failure:
                observed={};e['witness_observer_passed']=False;e['witness_observer_error']=type(failure).__name__+': '+str(failure)
            cmdlines=[line for line in console.splitlines()if 'Kernel command line:'in line]
            actual_cmdline=any(all(t in line for t in (s['compiled_cmdline']+' '+s['test_cmdline_append']).split())for line in cmdlines)
            passed=bool(e['witness_observer_passed']and actual_cmdline and e.get('Docker_CLI_exit')==0 and not error and not e['timed_out']and e['runtime_closure_passed']and (e.get('boot_blob_preflight') or {}).get('passed')and profile.quit_replied and pipe_eof and qmp_terminal)
            result={'schema':1,'passed':passed,'case_id':s['case_id'],'prerequisites_passed':passed,'classification':'native-pmsg-case-prerequisites-and-goal-observed' if passed else 'fixture-prerequisite-failure-not-behavioral-RED','scope':s['scope'],'actual_observations':observed,'native_pmsg_goal':observed.get('native_pmsg_goal'),'actual_compiled_and_test_cmdline_tokens_observed':actual_cmdline,'actual_kernel_commandline_lines':cmdlines,'console_sha256':sha(ROOT/'console.log')if(ROOT/'console.log').exists()else None,'actual_cli_exit':e.get('Docker_CLI_exit'),'actual_cli_wait':e['actual_Docker_CLI_wait'],'stdout_eof':pipe_eof,'stderr_eof':pipe_eof,'stdout_stderr_are_one_merged_owned_pipe':True,'runtime_closure_passed':e['runtime_closure_passed'],'all_observations_before_original_cutoffs':bool(passed and e.get('Docker_CLI_waited_monotonic_ns',2**64)<e.get('original_outer_until_monotonic_ns',0) and e.get('merged_stdout_stderr_EOF_observed_monotonic_ns',2**64)<e.get('original_outer_until_monotonic_ns',0)),'observer_passed':e['witness_observer_passed'],'observer_error':e.get('witness_observer_error'),'root_disk_NIC_hostdevices_phone_vendor_runtime':False,'Pixel_loader_AVB_or_hardware_retention_USB_watchdog_thermal_powerloss_qualified':False,'qualified_behavioral_RED':False,'qualified_GREEN':False,'aggregate_classification_required':True}
            e['passed']=passed
            save(ROOT/'qmp-records.json',e['qmp_records']);save(ROOT/'result.json',result);e['result_sha256']=sha(ROOT/'result.json');e['qmp_records_sha256']=sha(ROOT/'qmp-records.json');save(ROOT/'execution.json',e)
    print(json.dumps({'passed':result['passed'],'classification':result['classification'],'actual_Docker_CLI_wait_exit':e.get('Docker_CLI_exit'),'runtime_closure_passed':e['runtime_closure_passed'],'error':error,'observer_error':e.get('witness_observer_error')},indent=2),flush=True)
    return 0 if result['passed']else 1
if __name__=='__main__':raise SystemExit(main())
