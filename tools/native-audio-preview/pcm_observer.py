#!/usr/bin/env python3
"""One finite Stop/quiet/new explicit Play observation; no source-to-sink mutation."""
import array, base64, hashlib, json, math, os, pathlib, select, selectors, signal, subprocess, sys, time

class SetupFailure(RuntimeError): pass
class SampleFailure(RuntimeError): pass

def observe(output):
    output=pathlib.Path(output);output.mkdir(mode=0o700)
    runtime=pathlib.Path(os.environ['POKELINUX_PREVIEW_RUNTIME'])
    oracle=json.loads(pathlib.Path(__file__).with_name('ORACLES.json').read_text())
    if sys.byteorder!='little':raise SetupFailure('F32LE fixture requires little endian')
    env=dict(os.environ,PIPEWIRE_RUNTIME_DIR='/tmp/pokelinux-native-audio/private',PIPEWIRE_REMOTE='pokelinux-audio-test-manager',PIPEWIRE_DEBUG='0')
    children=[];receipts=[];streams={};logs=[];handles=[];raw_stages={};serial=0;capture_until=None
    result={'passed':False,'classification':'SETUP_FAILURE','stage':'actual_QML_half_Stop_quiet_muted_Play','oracle_sha256':hashlib.sha256(pathlib.Path(__file__).with_name('ORACLES.json').read_bytes()).hexdigest(),'stages':{}}
    clock=lambda:time.clock_gettime_ns(time.CLOCK_BOOTTIME)
    def publish(name,value):
        target=output/name;temporary=output/(name+'.tmp');temporary.write_text(json.dumps(value)+'\n');os.replace(temporary,target)
    def tool(label,args):
        begin=time.monotonic_ns();until=begin+2_000_000_000
        if capture_until is not None:
            until=min(until,capture_until)
            if begin>=until:raise SampleFailure('one original six-second observer cutoff')
        p=subprocess.Popen(args,env=env,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE);signals=[]
        try:a,b=p.communicate(timeout=max(0,(until-time.monotonic_ns())/1_000_000_000))
        except subprocess.TimeoutExpired:
            p.kill();signals.append(int(signal.SIGKILL));a,b=p.communicate(timeout=1)
        completed=time.monotonic_ns();within=completed<=until and not signals
        receipts.append({'label':label,'pid':p.pid,'argv':args,'actual_wait_exit':p.returncode,'signals_sent':signals,'stdout_EOF':True,'stderr_EOF':True,'original_until_ns':until,'waited_ns':completed,'stdout_EOF_at_ns':completed,'stderr_EOF_at_ns':completed,'completion_within_original_cutoff':within})
        (output/(label+'.stdout')).write_bytes(a);(output/(label+'.stderr')).write_bytes(b)
        if not within:raise SetupFailure('tool original cutoff '+label)
        if p.returncode or len(a)>1024*1024 or len(b)>65536:raise SetupFailure('tool failure '+label)
        return a
    def graph():
        nonlocal serial
        serial+=1;return json.loads(tool('graph-%02d'%serial,['/usr/bin/pw-dump']))
    def node(objects,name):
        rows=[x for x in objects if x['type'].endswith(':Node') and x.get('info',{}).get('props',{}).get('node.name')==name]
        if len(rows)!=1:raise SetupFailure('ambiguous/missing node '+name)
        return rows[0]
    def media(objects):
        return [x for x in objects if x['type'].endswith(':Node') and x.get('info',{}).get('props',{}).get('node.name')=='source.media']
    def port(objects,name,direction,monitor=False):
        n=node(objects,name);ports=[x for x in objects if x['type'].endswith(':Port') and str(x.get('info',{}).get('props',{}).get('node.id'))==str(n['id']) and x.get('info',{}).get('props',{}).get('port.direction')==direction]
        if len(ports)!=1:raise SetupFailure('ambiguous/missing mono port '+name)
        pname=ports[0]['info']['props']['port.name']
        if monitor and not pname.startswith('monitor_'):raise SetupFailure('sink output is not named monitor port')
        return name+':'+pname
    def identity(objects):
        rows=media(objects)
        if len(rows)!=1:raise SampleFailure('one actual owned preview source required')
        source=rows[0];sp=source['info']['props'];clients=[x for x in objects if x['type'].endswith(':Client') and x['id']==sp.get('client.id')]
        if len(clients)!=1:raise SampleFailure('missing server source client identity')
        cp=clients[0]['info']['props'];main=json.loads((runtime/'audio-main.json').read_text());pid=cp.get('pipewire.sec.pid')
        if type(pid)is not int or cp.get('pipewire.sec.uid')!=11040 or cp.get('pipewire.sec.gid')!=11040 or type(sp.get('object.serial'))is not int or type(cp.get('object.serial'))is not int:raise SampleFailure('missing server source attribution')
        try:fields=(pathlib.Path('/proc')/str(pid)/'stat').read_text().rsplit(') ',1)[1].split()
        except FileNotFoundError:raise SampleFailure('preview producer already absent')
        if int(fields[1])!=main['pid']:raise SampleFailure('preview is not retained direct Main child')
        if not any(v.get('mediaType')=='audio' and v.get('mediaSubtype')=='raw' and v.get('format')=='F32LE' and v.get('rate')==48000 and v.get('channels')==1 and v.get('position')==['MONO'] for v in source['info'].get('params',{}).get('EnumFormat',[])):raise SampleFailure('actual preview source format unsupported')
        core=next(x for x in objects if x['type'].endswith(':Core'))
        return {'pid':pid,'actual_parent_Main':main['pid'],'starttime':fields[19],'node_id':source['id'],'node_serial':sp['object.serial'],'client_id':clients[0]['id'],'Client_server_security':{k:cp[k]for k in ['pipewire.sec.pid','pipewire.sec.uid','pipewire.sec.gid','object.serial']},'current_core_cookie':core['info']['cookie'],'Core_version':core['info']['version'],'raw_EnumFormat':source['info']['params']['EnumFormat']}
    def active_source_link(objects):
        rows=media(objects)
        if not rows:return False
        if len(rows)!=1:raise SampleFailure('ambiguous preview source')
        target=node(objects,'route.speaker');links=[x for x in objects if x['type'].endswith(':Link') and x.get('info',{}).get('output-node-id')==rows[0]['id'] and x.get('info',{}).get('input-node-id')==target['id']]
        return len(links)==1 and links[0]['info'].get('state')=='active'
    seen={}
    def drain():
        for sink,(p,_) in streams.items():
            while True:
                if capture_until is not None and time.monotonic_ns()>=capture_until:raise SampleFailure('one original capture cutoff during drain')
                try:chunk=os.read(p.stdout.fileno(),65536)
                except BlockingIOError:break
                if not chunk:raise SetupFailure('recorder EOF before fresh PCM window')
                seen[sink]+=len(chunk)
                if seen[sink]>1_200_000:raise SetupFailure('bounded PCM byte budget')
    def marker(name):
        path=runtime/name
        while not path.exists():
            if time.monotonic_ns()>=capture_until:raise SampleFailure('one original capture cutoff waiting for '+name)
            drain();time.sleep(.002)
        return json.loads(path.read_text())
    def actual_send(value,op,choice):
        record=value['send'];raw=base64.b64decode(record['bytes_base64'],validate=True);wire=json.loads(raw);actor=json.loads((runtime/'actor-main.json').read_text())
        if not(record['actor_peer']is True and record['peer_valid']is True and(record['peer_pid'],record['peer_uid'],record['peer_gid'])==(actor['pid'],0,0)and record['fd']>=0 and record['fd']!=record['controller_fd_at_send']and record['complete_send']is True and int(record['result'])==int(record['length'])==record['copied_bytes']==len(raw)<=4096 and record['clocks_valid']is True and int(record['before_boottime_ns'])<=int(record['after_boottime_ns'])and record['play_choice']==choice and wire.get('op')==op):raise SetupFailure('complete actual unchanged-forward actor send telemetry required')
        dispatch=value['dispatch']
        if not(dispatch['play_choice']==choice and dispatch['clicked_signals']==1 and dispatch['press_delivered']is True and dispatch['release_delivered']is True and dispatch['all_quick_windows_hidden']is True):raise SetupFailure('one actual unshown QML button click required')
        return wire,record
    def gate(play,label,choice):
        wire,record=actual_send(play,'media_preview_play',choice);request=wire['request'];until=request['until_boottime_ns'];gate={'original_start_until_boottime_ns':until,'qualified':False,'actual_complete_send_marker':record['marker']}
        if not(set(wire)=={'op','request'}and set(request)=={'route','ttl_ms','until_boottime_ns','level_percent','muted'}and request['route']=='media-a7'and type(request['level_percent'])is int and request['level_percent']==50 and type(request['muted'])is bool and request['muted']==(choice==2)and type(until)is int and type(request['ttl_ms'])is int and 1<=request['ttl_ms']<=1000 and int(record['after_boottime_ns'])<until and 0<until-int(record['before_boottime_ns'])<=1_000_000_000):raise SampleFailure('Play deadline/remaining TTL must come from actual complete production bytes')
        while clock()<until and time.monotonic_ns()<capture_until:
            objects=graph();observed=clock();drain()
            if observed>=until:break
            if active_source_link(objects):
                value=identity(objects);fd=os.pidfd_open(value['pid'],os.O_NONBLOCK);handles.append(fd)
                if select.select([fd],[],[],0)[0]:raise SampleFailure('preview producer dead before capture')
                if identity(objects)!=value:raise SampleFailure('preview identity changed before capture')
                gate.update(qualified=True,observed_boottime_ns=observed,producer=value);result[label]=gate;return value,fd
            time.sleep(min(.01,max(0,(until-clock())/1e9)))
        result[label]=gate;raise SampleFailure('owned source/link absent before original Play start deadline')
    def absent(objects,old,fd):
        old_clients=[x for x in objects if x['type'].endswith(':Client') and x.get('info',{}).get('props',{}).get('pipewire.sec.pid')==old['pid']]
        old_links=[x for x in objects if x['type'].endswith(':Link') and x.get('info',{}).get('output-node-id')==old['node_id']]
        return not media(objects) and not old_clients and not old_links and bool(select.select([fd],[],[],0)[0]) and not pathlib.Path('/proc',str(old['pid'])).exists()
    def monitor_facts(objects):
        facts={}
        for sink,(_,recname) in streams.items():
            rec=node(objects,recname);sink_node=node(objects,'route.'+sink);links=[x for x in objects if x['type'].endswith(':Link') and x.get('info',{}).get('input-node-id')==rec['id']]
            if len(links)!=1 or links[0]['info'].get('output-node-id')!=sink_node['id'] or links[0]['info'].get('state')!='active':raise SetupFailure('actual unique active monitor link not qualified')
            formats=rec['info'].get('params',{}).get('Format',[])
            if not any(x.get('format')=='F32LE' and x.get('rate')==48000 and x.get('channels')==1 for x in formats):raise SetupFailure('actual negotiated recorder format not qualified')
            facts[sink]={'link':links[0],'recorder_formats':formats}
        return facts
    def capture(label,stage_name,old,fd,quiet=False):
        stage=next(x for x in oracle['stages']if x['name']==stage_name);goal=4*(oracle['discard_frames']+oracle['measure_frames']);data={s:bytearray()for s in streams};errors=[];drain()
        while any(len(x)<goal for x in data.values()):
            if time.monotonic_ns()>=capture_until:raise SetupFailure('incomplete actual PCM window under original six-second cutoff')
            for key,_ in selector.select(min(.05,max(0,(capture_until-time.monotonic_ns())/1e9))):
                chunk=os.read(key.fileobj.fileno(),65536)
                if not chunk:raise SetupFailure('recorder EOF before full paired windows')
                sink=key.data;seen[sink]+=len(chunk)
                if seen[sink]>1_200_000:raise SetupFailure('bounded PCM byte budget')
                data[sink].extend(chunk[:max(0,goal-len(data[sink]))])
            if any(p.poll()is not None for _,p,_ in children):raise SetupFailure('owned recorder/link exited during capture')
        final=graph();facts=monitor_facts(final);stats={}
        for sink,raw in data.items():
            samples=array.array('f');samples.frombytes(raw[4*oracle['discard_frames']:]);assert len(samples)==oracle['measure_frames']
            finite=all(math.isfinite(x)for x in samples)
            if not finite:raise SampleFailure('nonfinite actual PCM '+sink)
            n=len(samples);rms=math.sqrt(sum(x*x for x in samples)/n);peak=max(abs(x)for x in samples);amps={}
            for frequency in [440,880]:
                sine=sum(x*math.sin(2*math.pi*frequency*i/48000)for i,x in enumerate(samples));cosine=sum(x*math.cos(2*math.pi*frequency*i/48000)for i,x in enumerate(samples));amps[frequency]=2*math.hypot(sine,cosine)/n
            expected=stage[sink+'_hz'];stat={'frames':n,'finite_all':finite,'rms':rms,'peak':peak,'tone_amplitudes':amps,'expected_hz':expected};stats[sink]=stat
            if expected:
                fraction=amps[expected]**2/(2*rms*rms)if rms else 0;stat['selected_tone_energy_fraction']=fraction
                if not(oracle['active_rms_min']<=rms<=oracle['active_rms_max']and fraction>=oracle['active_tone_energy_fraction_min']and amps[1320-expected]<=oracle['wrong_tone_amplitude_max']):errors.append('selected actual tone oracle '+sink)
            elif rms>oracle['silence_rms_max']or peak>oracle['silence_peak_max']:errors.append('unintended old-route actual PCM '+sink)
        value={'complete_paired_windows':all(len(raw)==goal for raw in data.values()),'stats':stats,'actual_monitor_links':facts,'captured_boottime_ns':clock()};raw_stages[label]=data
        if quiet:
            value['old_source_client_links_absent_pidfd_dead_proc_absent']=absent(final,old,fd)
            if not value['old_source_client_links_absent_pidfd_dead_proc_absent']:errors.append('old producer/source reappeared through quiet window')
        else:
            if select.select([fd],[],[],0)[0]:errors.append('retained preview producer dead before final graph')
            else:
                after=identity(final);value['producer_after_capture']=after
                if after!=old:errors.append('server/retained preview identity changed across full capture')
                if not active_source_link(final):errors.append('owned source route not active in final graph')
            value['live_attributed_producer_and_active_selected_link']=not errors
        value['sample_errors']=errors;value['passed']=not errors;result['stages'][label]=value
        if errors:raise SampleFailure('; '.join(errors))
        return value
    try:
        initial=graph()
        if media(initial):raise SetupFailure('fixture media source must be absent before Play')
        result['initial_source_absent']=True
        for sink in ['speaker','earpiece']:
            recname='observer.preview_stop.'+sink;props=json.dumps({'node.name':recname,'node.autoconnect':False,'node.dont-reconnect':True,'node.always-process':False,'stream.capture.sink':True})
            args=['/usr/bin/pw-record','--raw','--format','f32','--rate','48000','--channels','1','--channel-map','MONO','--latency','256','--target','0','-P',props,'-'];log=open(output/(sink+'-recorder.stderr'),'wb');logs.append(log)
            child=subprocess.Popen(args,env=env,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=log);os.set_blocking(child.stdout.fileno(),False);children.append(('recorder-'+sink,child,args));streams[sink]=(child,recname);seen[sink]=0
        ready_until=time.monotonic()+2;configured=set()
        while True:
            objects=graph()
            for sink,(child,recname)in streams.items():
                found=[x for x in objects if x['type'].endswith(':Node')and x.get('info',{}).get('props',{}).get('node.name')==recname]
                if len(found)==1 and sink not in configured:
                    tool('configure-'+sink,['/usr/bin/pw-cli','set-param',str(found[0]['id']),'PortConfig','{ direction = Input mode = dsp monitor = false format = { mediaType = audio mediaSubtype = raw format = F32 rate = 48000 channels = 1 position = [ MONO ] } }']);configured.add(sink)
            if configured:objects=graph()
            try:
                ports={s:(port(objects,'route.'+s,'out',True),port(objects,name,'in'))for s,(child,name)in streams.items()}
                if time.monotonic()>=ready_until:raise SetupFailure('recorder readiness original cutoff')
                break
            except SetupFailure:
                if any(child.poll()is not None for child,_ in streams.values())or time.monotonic()>=ready_until:raise
                time.sleep(.025)
        result['actual_monitor_ports']=ports
        for sink,(outport,inport)in ports.items():
            args=['/usr/bin/pw-link','-m','-w',outport,inport];log=open(output/(sink+'-link.stderr'),'wb');logs.append(log);child=subprocess.Popen(args,env=env,stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=log);children.append(('monitor-link-'+sink,child,args))
        capture_until=time.monotonic_ns()+oracle['capture_deadline_seconds']*1_000_000_000;capture_until_boot=clock()+oracle['capture_deadline_seconds']*1_000_000_000
        result['original_capture_until_monotonic_ns']=capture_until;result['original_capture_until_boottime_ns']=capture_until_boot
        selector=selectors.DefaultSelector()
        for sink,(child,_)in streams.items():selector.register(child.stdout,selectors.EVENT_READ,sink)
        publish('ready.json',{'no_fixture_media_source':True,'actual_monitor_ports':ports,'original_capture_until_monotonic_ns':capture_until,'original_capture_until_boottime_ns':capture_until_boot})
        play=marker('qml-play-sent.json');first,firstfd=gate(play,'first_original_start_gate',1);before=capture('before_Stop_half_tone','before_Stop_half_tone',first,firstfd);publish('before-Stop.json',{'producer':first,'full_PCM':before})
        stop=marker('qml-stop-sent.json');result['actual_QML_Stop']=stop;wire,stop_send=actual_send(stop,'media_preview_stop',1)
        if wire!={'op':'media_preview_stop'}or int(stop_send['after_boottime_ns'])>=int(play['dispatch']['before_boottime_ns'])+4_000_000_000:raise SampleFailure('one actual complete Stop send must precede natural four-second asset end')
        state=stop['observed_retiring']['state'];result['actual_QML_Retiring_fence']=state['status']==state['qml_status']=='Preview retiring'
        # This bound was fixed from first Play before Stop, not from cleanup progress.
        retire_until=min(capture_until_boot,int(play['dispatch']['before_boottime_ns'])+3_000_000_000);result['original_retirement_until_boottime_ns']=retire_until
        while clock()<retire_until and time.monotonic_ns()<capture_until:
            objects=graph();observed=clock();drain()
            if observed<retire_until and absent(objects,first,firstfd):
                result['actual_retirement']={'observed_boottime_ns':observed,'old_source_client_links_absent':True,'retained_pidfd_terminal':True,'old_proc_absent':True};break
            time.sleep(.01)
        else:raise SampleFailure('explicit Stop did not retire producer/source before original pre-natural bound')
        quiet=capture('after_Stop_quiet','after_Stop_quiet',first,firstfd,quiet=True);publish('quiet.json',quiet)
        new_play=marker('qml-new-play-sent.json');second,secondfd=gate(new_play,'new_explicit_original_start_gate',2)
        distinct=second['pid']!=first['pid']and second['starttime']!=first['starttime']and second['node_serial']!=first['node_serial']and second['Client_server_security']['object.serial']!=first['Client_server_security']['object.serial']and second['actual_parent_Main']==first['actual_parent_Main']and second['current_core_cookie']==first['current_core_cookie']
        result['new_explicit_producer_distinct_same_Main_cookie']=distinct
        if not distinct:raise SampleFailure('new explicit Play must have a new owned producer identity')
        capture('new_explicit_muted_Play','new_explicit_muted_Play',second,secondfd)
        result['complete_paired_windows']=all(v['complete_paired_windows']for v in result['stages'].values())and len(result['stages'])==3
        result['old_retained_pidfd_still_terminal']=bool(select.select([firstfd],[],[],0)[0]);result['captured_at_monotonic_ns']=time.monotonic_ns();result['observed_bytes']=seen
        if not result['actual_QML_Retiring_fence']:raise SampleFailure('actual public AudioClient/QML Retiring status required separately from quiet')
        if result['captured_at_monotonic_ns']>=capture_until:raise SampleFailure('one original six-second capture cutoff')
        result['passed']=True;result['classification']='ACTUAL_PCM_ORACLES_PASS'
    except Exception as error:
        result['error']=type(error).__name__+': '+str(error);result['classification']='BEHAVIORAL_SAMPLE_FAILURE'if isinstance(error,SampleFailure)else'SETUP_FAILURE'
    finally:
        if 'selector'in locals():selector.close()
        for fd in handles:os.close(fd)
        result.setdefault('complete_paired_windows',False)
        for name,child,args in reversed(children):
            until=time.monotonic_ns()+1_000_000_000;signals=[]
            if child.poll()is None:
                sig=signal.SIGINT if name.startswith('recorder-')else signal.SIGTERM;child.send_signal(sig);signals.append(int(sig))
            recovery=False
            try:a,_=child.communicate(timeout=max(0,(until-time.monotonic_ns())/1_000_000_000))
            except subprocess.TimeoutExpired:
                recovery=True;child.kill();signals.append(int(signal.SIGKILL));a,_=child.communicate(timeout=1)
            completed=time.monotonic_ns();within=completed<=until and not recovery
            receipts.append({'label':name,'pid':child.pid,'argv':args,'actual_wait_exit':child.returncode,'signals_sent':signals,'stdout_EOF':True if child.stdout else None,'stdout_EOF_at_ns':completed if child.stdout else None,'stderr_sink':'regular file','trailing_stdout_bytes':len(a or b''),'original_until_ns':until,'waited_ns':completed,'completion_within_original_cutoff':within})
            if not within:result['passed']=False;result['classification']='SETUP_FAILURE';result['cleanup_error']='owned cleanup original cutoff '+name
        for f in logs:f.close()
        for label,data in raw_stages.items():
            for sink,raw in data.items():(output/(label+'-'+sink+'.raw')).write_bytes(raw)
        try:
            pid,status=os.waitpid(-1,os.WNOHANG);result['final_children']={'unexpected_pid':pid,'status':status};result['passed']=False
        except ChildProcessError:result['final_children']={'ECHILD':True}
        result['owned_children']=receipts;publish('result.json',result)
    return result
if __name__=='__main__':
    value=observe(sys.argv[1]);print(json.dumps({'passed':value['passed'],'classification':value['classification'],'stage':value['stage']}));sys.exit(0 if value['passed']else 1)
