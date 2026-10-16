#!/usr/bin/env python3
"""Finite actual sink-monitor PCM observer; creates no source-to-sink routes."""
import array, hashlib, json, math, os, pathlib, selectors, signal, subprocess, sys, time

class SetupFailure(RuntimeError): pass
class SampleFailure(RuntimeError): pass

def observe(stage_name, output):
    output=pathlib.Path(output);output.mkdir(mode=0o700)
    oracle=json.loads(pathlib.Path('/proposal/ORACLES.json').read_text())
    stage=next(x for x in oracle['stages'] if x['name']==stage_name)
    if sys.byteorder!='little': raise SetupFailure('F32LE fixture requires little endian')
    env=dict(os.environ,PIPEWIRE_RUNTIME_DIR='/tmp/pokelinux-native-audio/private',PIPEWIRE_REMOTE='pokelinux-audio-test-manager',PIPEWIRE_DEBUG='0')
    children=[];receipts=[];streams={};data={};logs=[];result={'passed':False,'stage':stage_name,'classification':'SETUP_FAILURE','oracle_sha256':hashlib.sha256(pathlib.Path('/proposal/ORACLES.json').read_bytes()).hexdigest()}
    def tool(label,args):
        begin=time.monotonic_ns();until=begin+2_000_000_000
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
    serial=0
    def graph():
        nonlocal serial
        serial+=1;return json.loads(tool('graph-%02d'%serial,['/usr/bin/pw-dump']))
    def node(objects,name):
        rows=[x for x in objects if x['type'].endswith(':Node') and x.get('info',{}).get('props',{}).get('node.name')==name]
        if len(rows)!=1:raise SetupFailure('ambiguous/missing node '+name)
        return rows[0]
    def port(objects,name,direction,monitor=False):
        n=node(objects,name);ports=[x for x in objects if x['type'].endswith(':Port') and str(x.get('info',{}).get('props',{}).get('node.id'))==str(n['id']) and x.get('info',{}).get('props',{}).get('port.direction')==direction]
        if len(ports)!=1:raise SetupFailure('ambiguous/missing mono port '+name)
        pname=ports[0]['info']['props']['port.name']
        if monitor and not pname.startswith('monitor_'):raise SetupFailure('sink output is not named monitor port')
        return name+':'+pname
    try:
        initial=graph()
        source_props={}
        for name,freq in [('source.media',440)]:
            props=node(initial,name)['info'].get('params',{}).get('Props',[])
            selected=[p for p in props if p.get('frequency')==freq and p.get('waveType')==0 and p.get('volume')==0.25]
            if len(selected)!=1:raise SetupFailure('actual controlled source Props not qualified: '+name)
            source_props[name]=selected[0]
        tone=json.loads(pathlib.Path('/results/tone-source.json').read_text())
        call_props=node(initial,'source.call')['info']['props']
        clients=[x for x in initial if x['type'].endswith(':Client') and x['id']==call_props.get('client.id')]
        client_props=clients[0]['info']['props'] if len(clients)==1 else {}
        if tone['frequency']!=880 or tone['format']!='F32LE' or tone['rate']!=48000 or tone['channels']!=1 or hashlib.sha256(pathlib.Path(tone['path']).read_bytes()).hexdigest()!=tone['sha256'] or client_props.get('application.process.binary')!='pw-cat' or int(client_props.get('application.process.id',-1))!=tone['actual_child_pid']:
            raise SetupFailure('actual controlled pw-cat call source not qualified')
        result['actual_call_source']={'node_props':call_props,'client_props':client_props,'finite_input':tone}
        result['actual_source_props']=source_props
        for sink in ['speaker','earpiece']:
            recname='observer.'+stage_name+'.'+sink
            props=json.dumps({'node.name':recname,'node.autoconnect':False,'node.dont-reconnect':True,'node.always-process':True,'stream.capture.sink':True})
            args=['/usr/bin/pw-record','--raw','--format','f32','--rate','48000','--channels','1','--channel-map','MONO','--latency','256','--target','0','-P',props,'-']
            log=open(output/(sink+'-recorder.stderr'),'wb');logs.append(log)
            p=subprocess.Popen(args,env=env,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=log);os.set_blocking(p.stdout.fileno(),False)
            children.append(('recorder-'+sink,p,args));streams[sink]=(p,recname);data[sink]=bytearray()
        ready_until=time.monotonic()+2
        configured=set()
        while True:
            objects=graph()
            for sink,(p,recname) in streams.items():
                found=[x for x in objects if x['type'].endswith(':Node') and x.get('info',{}).get('props',{}).get('node.name')==recname]
                if len(found)==1 and sink not in configured:
                    tool('configure-'+sink,['/usr/bin/pw-cli','set-param',str(found[0]['id']),'PortConfig','{ direction = Input mode = dsp monitor = false format = { mediaType = audio mediaSubtype = raw format = F32 rate = 48000 channels = 1 position = [ MONO ] } }'])
                    configured.add(sink)
            if configured:
                objects=graph()
            try:
                ports={s:(port(objects,'route.'+s,'out',True),port(objects,name,'in')) for s,(p,name) in streams.items()}
                if time.monotonic()>=ready_until:raise SetupFailure('recorder readiness original cutoff')
                break
            except SetupFailure:
                if any(p.poll() is not None for p,_ in streams.values()) or time.monotonic()>=ready_until:raise
                time.sleep(.025)
        result['actual_monitor_ports']=ports
        for sink,(outport,inport) in ports.items():
            args=['/usr/bin/pw-link','-m','-w',outport,inport]
            log=open(output/(sink+'-link.stderr'),'wb');logs.append(log)
            p=subprocess.Popen(args,env=env,stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=log);children.append(('monitor-link-'+sink,p,args))
        goal=4*(oracle['discard_frames']+oracle['measure_frames']);cutoff=time.monotonic()+oracle['capture_deadline_seconds'];select=selectors.DefaultSelector()
        for sink,(p,_) in streams.items():select.register(p.stdout,selectors.EVENT_READ,sink)
        seen={s:0 for s in streams}
        while any(len(x)<goal for x in data.values()):
            if time.monotonic()>=cutoff:raise SetupFailure('incomplete actual PCM window')
            for key,_ in select.select(min(.05,max(0,cutoff-time.monotonic()))):
                chunk=os.read(key.fileobj.fileno(),65536)
                if not chunk:raise SetupFailure('recorder EOF before full paired windows')
                sink=key.data;seen[sink]+=len(chunk)
                if seen[sink]>1_200_000:raise SetupFailure('bounded PCM byte budget')
                remaining=goal-len(data[sink]);data[sink].extend(chunk[:max(0,remaining)])
            if any(p.poll() is not None for _,p,_ in children):raise SetupFailure('owned recorder/link exited during capture')
        select.close();result['captured_original_until_monotonic']=cutoff;result['captured_at_monotonic']=time.monotonic();result['observed_bytes']=seen
        if result['captured_at_monotonic']>cutoff:raise SetupFailure('capture original cutoff')
        final=graph()
        result['actual_monitor_links']={}
        for sink,(_,recname) in streams.items():
            rec=node(final,recname);sink_node=node(final,'route.'+sink)
            links=[x for x in final if x['type'].endswith(':Link') and x.get('info',{}).get('input-node-id')==rec['id']]
            if len(links)!=1 or links[0]['info'].get('output-node-id')!=sink_node['id'] or links[0]['info'].get('state')!='active':
                raise SetupFailure('actual unique active monitor link not qualified')
            formats=rec['info'].get('params',{}).get('Format',[])
            if not any(x.get('format')=='F32LE' and x.get('rate')==48000 and x.get('channels')==1 for x in formats):
                raise SetupFailure('actual negotiated recorder format not qualified')
            result['actual_monitor_links'][sink]={'link':links[0],'recorder_formats':formats}
        result['stats']={}
        for sink,raw in data.items():
            samples=array.array('f');samples.frombytes(raw[4*oracle['discard_frames']:]);assert len(samples)==oracle['measure_frames']
            finite=all(math.isfinite(x) for x in samples)
            if not finite:raise SampleFailure('nonfinite actual PCM '+sink)
            n=len(samples);rms=math.sqrt(sum(x*x for x in samples)/n);peak=max(abs(x) for x in samples);amps={}
            for frequency in [440,880]:
                sine=sum(x*math.sin(2*math.pi*frequency*i/48000) for i,x in enumerate(samples));cosine=sum(x*math.cos(2*math.pi*frequency*i/48000) for i,x in enumerate(samples));amps[frequency]=2*math.hypot(sine,cosine)/n
            expected=stage[sink+'_hz'];stat={'frames':n,'finite_all':finite,'rms':rms,'peak':peak,'tone_amplitudes':amps,'expected_hz':expected};result['stats'][sink]=stat
            if expected:
                fraction=amps[expected]**2/(2*rms*rms) if rms else 0;stat['selected_tone_energy_fraction']=fraction
                if not(oracle['active_rms_min']<=rms<=oracle['active_rms_max'] and fraction>=oracle['active_tone_energy_fraction_min'] and amps[1320-expected]<=oracle['wrong_tone_amplitude_max']):raise SampleFailure('selected actual tone oracle '+sink)
            elif rms>oracle['silence_rms_max'] or peak>oracle['silence_peak_max']:raise SampleFailure('unintended old-route actual PCM '+sink)
        result['passed']=True;result['classification']='ACTUAL_PCM_ORACLES_PASS'
    except Exception as error:
        result['error']=type(error).__name__+': '+str(error)
        result['classification']='BEHAVIORAL_SAMPLE_FAILURE' if isinstance(error,SampleFailure) else 'SETUP_FAILURE'
    finally:
        for name,p,args in reversed(children):
            until=time.monotonic_ns()+1_000_000_000;signals=[]
            if p.poll() is None:
                sig=signal.SIGINT if name.startswith('recorder-') else signal.SIGTERM;p.send_signal(sig);signals.append(int(sig))
            recovery=False
            try:a,_=p.communicate(timeout=max(0,(until-time.monotonic_ns())/1_000_000_000))
            except subprocess.TimeoutExpired:
                recovery=True;p.kill();signals.append(int(signal.SIGKILL));a,_=p.communicate(timeout=1)
            completed=time.monotonic_ns();within=completed<=until and not recovery
            receipts.append({'label':name,'pid':p.pid,'argv':args,'actual_wait_exit':p.returncode,'signals_sent':signals,'stdout_EOF':True if p.stdout else None,'stdout_EOF_at_ns':completed if p.stdout else None,'stderr_sink':'regular file','trailing_stdout_bytes':len(a or b''),'original_until_ns':until,'waited_ns':completed,'completion_within_original_cutoff':within})
            if not within:result['passed']=False;result['classification']='SETUP_FAILURE';result['cleanup_error']='owned cleanup original cutoff '+name
        for f in logs:f.close()
        for sink,raw in data.items():(output/(sink+'.raw')).write_bytes(raw)
        try:
            pid,status=os.waitpid(-1,os.WNOHANG);result['final_children']={'unexpected_pid':pid,'status':status};result['passed']=False
        except ChildProcessError:result['final_children']={'ECHILD':True}
        result['owned_children']=receipts
        (output/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    return result
if __name__=='__main__':
    value=observe(sys.argv[1],sys.argv[2]);print(json.dumps({'passed':value['passed'],'classification':value['classification'],'stage':value['stage']}));sys.exit(0 if value['passed'] else 1)
