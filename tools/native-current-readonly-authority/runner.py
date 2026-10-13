#!/usr/bin/env python3
"""Current split writer/readonly process fixture; preserves archived authority assertions."""
import argparse,importlib.util,json,os,select,signal,socket,subprocess,sys,time,traceback
from pathlib import Path
assert os.geteuid()==0 and os.environ.get('NATIVE_MESSAGES_ISOLATED')=='1'
spec=importlib.util.spec_from_file_location('archived_authority_fixture','/source/tools/native-session-authority/runner.py')
base=importlib.util.module_from_spec(spec);spec.loader.exec_module(base)
OUT=Path('/results/execution');OUT.mkdir(parents=True)

class Scope(base.Scope):
    def __init__(self,*args,**kwargs):
        super().__init__(*args,**kwargs)
        self.actor_signal_handle=None

    def track_children(self):
        path=Path(f'/proc/{self.service.pid}/task/{self.service.pid}/children')
        for entry in path.read_text().split():
            try:
                args=Path(f'/proc/{entry}/cmdline').read_bytes().split(b'\0')
                role='messages_read_worker' if b'--messages-read-worker' in args else 'messages_writer' if b'--messages-worker' in args else 'policy_storage_worker' if b'--storage-worker' in args else 'app'
                self.track_child(int(entry),role)
            except (FileNotFoundError,ProcessLookupError):pass

    def worker(self):
        matches=[]
        for entry in Path(f'/proc/{self.service.pid}/task/{self.service.pid}/children').read_text().split():
            path=Path('/proc')/entry
            try:
                args=path.joinpath('cmdline').read_bytes().split(b'\0')
                if b'--messages-read-worker' in args:
                    assert int(path.joinpath('stat').read_text().split()[3])==self.service.pid
                    matches.append(int(entry))
            except (FileNotFoundError,ProcessLookupError):pass
        assert len(matches)==1,('one exact current readonly child required',matches)
        pid=matches[0];self.track_child(pid,'messages_read_worker');handle=self.tracked_children[pid]['fd']
        assert base.fdinfo_pid(handle)==pid and not select.select([handle],[],[],0)[0]
        self.observe('current_exact_readonly_child_selected',{'pid':pid,'parent':self.service.pid,'selector':'--messages-read-worker','retained_kernel_pid':base.fdinfo_pid(handle)})
        return pid

    def _handle(self,pid):
        if self.source is not None and pid==self.source.pid:return self.alive
        if self.service is not None and pid==self.service.pid:
            if self.actor_signal_handle is None:
                self.actor_signal_handle=os.pidfd_open(self.service.pid,os.O_NONBLOCK)
                assert base.fdinfo_pid(self.actor_signal_handle)==self.service.pid
                self.observe('owned_actor_Child_signal_pidfd_retained',{'pid':self.service.pid,'kernel_pid':base.fdinfo_pid(self.actor_signal_handle)})
            return self.actor_signal_handle
        return self.tracked_children[pid]['fd']

    def halt(self,pid):
        handle=self._handle(pid)
        assert base.fdinfo_pid(handle)==pid and not select.select([handle],[],[],0)[0]
        signal.pidfd_send_signal(handle,signal.SIGSTOP)
        self.stopped.add(pid)
        state=base.wait(lambda:(s if (s:=Path(f'/proc/{pid}/stat').read_text().split()[2]) in ('T','t') else None))
        self.observe('exact_retained_pidfd_SIGSTOP_and_real_T_state',{'pid':pid,'state':state,'kernel_pid':base.fdinfo_pid(handle)})

    def resume(self,pid):
        try:signal.pidfd_send_signal(self._handle(pid),signal.SIGCONT)
        except ProcessLookupError:pass
        self.stopped.discard(pid)
        self.observe('exact_retained_pidfd_SIGCONT_requested',{'pid':pid})

    def cleanup(self):
        try:super().cleanup()
        finally:
            if self.actor_signal_handle is not None:
                dead=bool(select.select([self.actor_signal_handle],[],[],0)[0])
                absent=not Path(f'/proc/{self.service.pid}').exists()
                self.details['reaps'].append({'role':'actor_signal_handle_direct_Child_wait','pid':self.service.pid,'owned_pidfd_death_observed':dead,'proc_absent':absent,'returncode':self.service.returncode})
                os.close(self.actor_signal_handle);self.actor_signal_handle=None
                (self.output/(self.name+'.json')).write_text(json.dumps(self.details,indent=2)+'\n')
                assert dead and absent,'owned actor signal handle was not directly waited'


def terminal_retirement(scope,kind):
    scope.authorized()
    old=dict(scope.app);old_handle=scope.app_fd
    assert base.fdinfo_pid(old_handle)==old['pid'] and not select.select([old_handle],[],[],0)[0]
    started=base.boot()
    if kind=='lease':
        scope.halt(scope.source.pid)
        time.sleep(.12)  # Original unchanged expiry observation; Source lease remains100ms.
    elif kind=='eof':
        scope.command({'source_op':'close'})
        assert scope.source.poll() is None,'EOF must be observed independently of source child death'
    elif kind=='death':
        scope.source.kill();scope.source.wait(timeout=2)
        assert scope.source.returncode==-signal.SIGKILL
    else:raise AssertionError(kind)
    scope.responsiveness()
    snapshot=base.wait(lambda:(s if (s:=scope.admin({'op':'snapshot'}))['principals']==0 else None))
    base.wait(lambda:select.select([old_handle],[],[],0)[0] and not Path(f'/proc/{old["pid"]}').exists())
    assert scope.service.poll() is None,'terminal Source must leave current actor live for authenticated Stop'
    assert snapshot['ready'] and snapshot['locked'] and snapshot['permits']==0
    try:
        scope.driver({'driver':'send','request':{'op':'messages_read','request':{'query':base.Q,'ttl_ms':1000}}})
    except (ConnectionRefusedError,ConnectionResetError,BrokenPipeError,FileNotFoundError) as error:
        closed={'type':type(error).__name__,'errno':error.errno}
    else:raise AssertionError('retired app driver still accepted a request')
    # Real UID1000 fixed launch-only ingress must refuse the terminal epoch.
    code='import json,socket,sys;s=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET);s.settimeout(2);s.connect(sys.argv[1]);s.send(json.dumps({"op":"launch_messages"}).encode());print(s.recv(4096).decode(),flush=True)'
    client=subprocess.Popen([sys.executable,'-c',code,str(scope.directory/'launch-messages.sock')],user=base.SOURCE_UID,group=base.SOURCE_UID,extra_groups=(),stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    try:stdout,stderr=client.communicate(timeout=3)
    except BaseException:
        client.kill();client.wait(timeout=2);raise
    scope.details['reaps'].append({'role':'terminal_desktop_launcher','pid':client.pid,'returncode':client.returncode,'direct_communicate_wait':True})
    assert client.returncode==0,stderr
    reply=json.loads(stdout);assert reply=={'result':'unavailable'},reply
    assert scope.admin({'op':'snapshot'})['principals']==0
    scope.observe('current_source_terminal_app_retirement',{'kind':kind,'started_at_ns':started,'observed_at_ns':base.boot(),'old_app':old,'retained_app_pidfd_dead':bool(select.select([old_handle],[],[],0)[0]),'old_app_proc_absent':not Path(f'/proc/{old["pid"]}').exists(),'snapshot':snapshot,'old_driver':closed,'explicit_uid1000_launch_reply':reply,'actor_live':scope.service.poll() is None,'source_child_alive':scope.source.poll() is None})
    scope.clean_stop()


def source_lease_expiry_retires_app(scope):terminal_retirement(scope,'lease')
def source_eof_retires_app(scope):terminal_retirement(scope,'eof')
def source_death_retires_app(scope):terminal_retirement(scope,'death')

CASES=[base.unknown_denied,base.root_controls_unknown_denied,base.source_only_read,
       base.focus_and_handle_denied,base.dead_retained_handle_denied,base.held_semantic_burst,
       base.heartbeat_preserves_read,base.heartbeat_cannot_extend_read,
       source_lease_expiry_retires_app,source_eof_retires_app,source_death_retires_app,
       base.eagain_rechecks_source,base.source_false_cannot_clear_intent,base.new_app_identity_and_grant,
       base.anon_inode_owned_focus_read]
parser=argparse.ArgumentParser();parser.add_argument('--case',action='append',choices=[c.__name__ for c in CASES]);options=parser.parse_args()
if options.case:CASES=[c for c in CASES if c.__name__ in options.case]
results=[]
for case in CASES:
    scope=Scope(Path('/bins'),OUT,case.__name__)
    entry={'case':case.__name__,'passed':False,'setup_complete':False,'body_origin':'new current terminal retirement' if case in (source_lease_expiry_retires_app,source_eof_retires_app,source_death_retires_app) else 'unchanged archived assertion/body'}
    try:
        scope.setup();entry['setup_complete']=True;case(scope);entry['passed']=True
        print('PASS '+case.__name__,flush=True)
    except BaseException as error:
        entry['error']=str(error);entry['traceback']=traceback.format_exc();print('FAIL '+case.__name__+': '+str(error),flush=True)
    finally:
        try:scope.cleanup()
        except BaseException as error:entry['passed']=False;entry['cleanup_error']=str(error)
    results.append(entry)
passed=all(c['passed'] for c in results)
(OUT/'result.json').write_text(json.dumps({'passed':passed,'fixture_version':'current-split-v2','cases':results,'kernel':{n:getattr(os.uname(),n) for n in ('sysname','release','version','machine')},'source':'synthetic UID1000 SCM_CREDENTIALS source; actual retained SO_PEERPIDFD focus','source_lease_ns':100000000,'core_source_guards_changed':False,'original_archived_results_changed':False,'held_mechanism':'SIGSTOP exact current readonly child and owned actor pidfd, not a held filesystem syscall','compositor_original_protocol_qualified':False,'phone_hardware_qualified':False,'zero_external_transition_publication_race_qualified':False},indent=2)+'\n')
raise SystemExit(0 if passed else 1)
