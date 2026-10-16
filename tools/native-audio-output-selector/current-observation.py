#!/usr/bin/env python3
"""Fixed real same-cookie graph loss/change -> current IPC observation/no-replay oracle."""
from pathlib import Path
import argparse, hashlib, json, os, select, subprocess, time

D=['docker','--host=unix:///var/run/docker.sock']
ENV={k:v for k,v in os.environ.items() if k not in {'DOCKER_HOST','DOCKER_CONTEXT','DOCKER_TLS','DOCKER_TLS_VERIFY','DOCKER_CERT_PATH'}}
CLIENT='''import socket,sys
s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
s.connect('/tmp/pokelinux-native-audio/control/audio.sock')
f=s.makefile('rwb',buffering=0)
for line in sys.stdin.buffer:
 f.write(line)
 reply=f.readline()
 if not reply:raise RuntimeError('authority EOF')
 sys.stdout.buffer.write(reply);sys.stdout.buffer.flush()
'''
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def dump(p,x):p.write_text(json.dumps(x,indent=2)+'\n')
def require(ok,why):
    if not ok:raise RuntimeError(why)

def line(process, seconds=5):
    until=time.monotonic()+seconds;data=bytearray()
    while not data.endswith(b'\n'):
        remaining=until-time.monotonic()
        require(remaining>0 and select.select([process.stdout],[],[],max(remaining,0))[0], 'owned line deadline')
        part=os.read(process.stdout.fileno(),1)
        require(part,'owned line EOF');data.extend(part)
        require(len(data)<=65_536,'bounded IPC line')
    return json.loads(data)

class Client:
    def __init__(self,cid,uid,receipts):
        self.receipts=receipts;self.uid=uid
        self.process=subprocess.Popen(D+['exec','-i','--user',str(uid),cid,'python3','-u','-c',CLIENT],env=ENV,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    def request(self,value):
        self.process.stdin.write(json.dumps(value).encode()+b'\n');self.process.stdin.flush()
        reply=line(self.process)
        self.receipts.append({'uid':self.uid,'request':value,'reply':reply})
        return reply
    def close(self):
        if self.process.stdin:self.process.stdin.close();self.process.stdin=None
        try:out,err=self.process.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill();out,err=self.process.communicate(timeout=3)
            raise RuntimeError('owned client deadline')
        self.receipts.append({'uid':self.uid,'actual_client_wait':self.process.returncode,'stdout_EOF':True,'stderr_EOF':True,'remaining_stdout_hex':out.hex(),'stderr_hex':err.hex()})
        require(self.process.returncode==0 and not out and not err,'owned client wait/PIPE EOF')

def run(build,out,image):
    out.mkdir()
    review=json.loads((build/'build-review.json').read_text());require(review['passed'],'strict build required')
    selected={r['path']:r['sha256'] for r in review['source_files']}
    def source_map():return {str(p.relative_to(build/'source')):sha(p) for p in (build/'source').rglob('*') if p.is_file()}
    require(source_map()==selected,'complete selected source hashes')
    artifact=next(a for a in review['compiled_executables'] if a['ELF_machine']==62)
    require(sha(build/artifact['path'])==artifact['sha256'],'selected audio executable')
    require(image.startswith('sha256:') and len(image)==71,'immutable runtime image required')
    cmd=D+['create','--cidfile',str(out/'cid'),'--network=none','--read-only','--cap-drop=ALL','--security-opt=no-new-privileges','--user','1000:1000','--cpus','2','--memory','1g','--pids-limit','128','--tmpfs','/tmp:rw,exec,nosuid,nodev,size=128m,mode=1777','-i','-v',f'{build}/source/tools/media-route-fixture.py:/fixture.py:ro','-v',f'{build}/target:/target:ro','-v',f'{out}:/results:rw',image,'python3','-u','/fixture.py']
    dump(out/'command.json',cmd)
    subprocess.run(cmd,env=ENV,capture_output=True,check=True,timeout=15)
    cid=(out/'cid').read_text().strip();began=time.monotonic_ns()
    fixture=subprocess.Popen(D+['start','-a','-i',cid],env=ENV,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    clients=[];requests=[];graph_count=0;stages=[]
    result={'passed':False,'classification':'SETUP_OR_FIXTURE_FAILURE','failure':None,'image':image,'cid':cid,'started_ns':began,'scope':'Native x86 real read-only audio IPC against changed original PipeWire virtual graph; no Qt/installed/physical/vendor proof','requests':requests,'stages':stages,'controller_sha256':sha(Path(__file__)),'build_review_sha256':sha(build/'build-review.json')}
    def command(op):
        fixture.stdin.write(json.dumps({'op':op}).encode()+b'\n');fixture.stdin.flush();reply=line(fixture,12);result.setdefault('fixture_operations',[]).append({'op':op,'reply':reply});return reply
    def client(uid):
        c=Client(cid,uid,requests);clients.append(c);return c
    def accepted(reply):require(reply.get('ok') is True,'actual IPC intent refused: '+repr(reply));return reply
    def graph():
        nonlocal graph_count
        r=subprocess.run(D+['exec','--user','1000',cid,'env','PIPEWIRE_RUNTIME_DIR=/tmp/pokelinux-native-audio/private','PIPEWIRE_REMOTE=pokelinux-audio-test-manager','pw-dump'],env=ENV,capture_output=True,timeout=4)
        require(r.returncode==0,'actual pw-dump failed')
        objects=json.loads(r.stdout);dump(out/f'graph-{graph_count:03}.json',objects);graph_count+=1
        nodes={x['id']:x.get('info',{}).get('props',{}).get('node.name') for x in objects if x['type'].endswith(':Node')}
        links={(nodes.get(x['info']['output-node-id']),nodes.get(x['info']['input-node-id'])) for x in objects if x['type'].endswith(':Link') and x.get('info',{}).get('state')=='active'}
        return objects,links
    def expect_graph(expected, marker='actual graph mismatch'):
        cutoff=time.monotonic()+5
        while True:
            objects,links=graph()
            states={x.get('info',{}).get('props',{}).get('node.name'):x.get('info',{}).get('state') for x in objects if x['type'].endswith(':Node')}
            if links==expected and all(states.get(n)=='running' for edge in expected for n in edge):return objects
            if time.monotonic()>=cutoff:
                result['last_actual_links']=sorted(links)
                raise RuntimeError(marker)
            time.sleep(.05)
    def observed(c,revision):
        cutoff=time.monotonic()+5
        while True:
            status=accepted(c.request({'op':'status'}))
            if status['intent_revision']==revision and status['application']=='observed' and status['observed']['available']:return status
            require(time.monotonic()<cutoff,'matching application observation deadline')
            time.sleep(.05)
    try:
        ready=line(fixture,12);require(ready['ready'],'actual original stack startup');result['fixture_ready']=ready
        versions={}
        for program in ['pipewire','wireplumber']:
            r=subprocess.run(D+['exec',cid,program,'--version'],env=ENV,capture_output=True,check=True,timeout=4)
            versions[program]=r.stdout.decode()+r.stderr.decode()
        result['original_versions']=versions
        objects=expect_graph(set())
        require(any(x.get('info',{}).get('props',{}).get('application.process.binary')=='wireplumber' for x in objects if x['type'].endswith(':Client')),'actual original WirePlumber client required')
        app=client(1002)
        status=accepted(app.request({'op':'status'}));catalog=status['media_routes']
        require(catalog==[{'id':'media-a7','label':'Speaker (virtual)'},{'id':'media-z2','label':'Headphones (virtual)'}],'unchanged sorted logical catalog')
        first=accepted(app.request({'op':'acquire','role':'media','route':catalog[0]['id']}))
        require('media_routes' not in first,'catalog contract belongs only to status')
        require(first['desired']==[[first['lease'],'media',catalog[0]['id']]],'actual admitted logical intent')
        result['accepted_logical_intent']=first
        initial_graph=expect_graph({('source.media','route.speaker')})
        initial=observed(app,first['intent_revision'])
        result['initial_observed_application']=initial
        def cookie(objects):
            identities=[x['info']['cookie'] for x in objects if x['type'].endswith(':Core')]
            require(len(identities)==1,'actual core identity required');return identities[0]
        core=cookie(initial_graph);epoch=initial['observed']['epoch'];revision=initial['observed']['revision']
        result['unchanged_actual_core_cookie']=core
        def current(expected):
            nonlocal revision
            cutoff=time.monotonic()+5
            while True:
                started=time.monotonic_ns();reply=accepted(app.request({'op':'status'}))
                require(time.monotonic_ns()-started<350_000_000,'read-only status responsiveness')
                require(reply['intent_revision']==first['intent_revision'] and reply['desired']==first['desired'],'read-only refresh changed desired intent')
                require(reply['observed']['available'] and reply['observed']['epoch']==epoch,'same live backend epoch required')
                actual={(x['source'],x['target'],x['state']) for x in reply['observed']['links']}
                if reply['observed']['revision']>revision and actual=={(a,b,'active') for a,b in expected}:
                    revision=reply['observed']['revision'];break
                require(time.monotonic()<cutoff,'fresh changed-graph observation deadline');time.sleep(.025)
            objects=expect_graph(expected)
            require(cookie(objects)==core,'real PipeWire core must not restart')
            require(reply['media_routes']==catalog,'configured catalog changes on link observation')
            return reply
        nodes={x['id']:x.get('info',{}).get('props',{}).get('node.name') for x in initial_graph if x['type'].endswith(':Node')}
        link_ids=[x['id'] for x in initial_graph if x['type'].endswith(':Link') and (nodes.get(x['info']['output-node-id']),nodes.get(x['info']['input-node-id']))==('source.media','route.speaker')]
        require(len(link_ids)==1,'exact actual owned link required')
        removed=subprocess.run(D+['exec','--user','1000',cid,'env','PIPEWIRE_RUNTIME_DIR=/tmp/pokelinux-native-audio/private','PIPEWIRE_REMOTE=pokelinux-audio-test-manager','pw-cli','destroy',str(link_ids[0])],env=ENV,capture_output=True,timeout=4)
        result['actual_link_destroy']={'link_id':link_ids[0],'actual_tool_wait':removed.returncode,'stdout_EOF':True,'stderr_EOF':True,'stdout_hex':removed.stdout.hex(),'stderr_hex':removed.stderr.hex()}
        require(removed.returncode==0,'actual PipeWire link destruction failed')
        expect_graph(set())
        failures=[];loss=[]
        for _ in range(3):
            reply=current(set());loss.append(reply)
            if reply['application']=='observed':failures.append('HISTORICAL_APPLICATION_AFTER_REAL_LINK_REMOVAL')
        result['fresh_missing_link_observations']=loss
        stages.append('same_cookie_fresh_link_loss_preserves_intent_without_replay')
        command('start_wrong_link');expect_graph({('source.media','route.earpiece')})
        changed=[]
        for _ in range(3):
            reply=current({('source.media','route.earpiece')});changed.append(reply)
            if reply['application']=='observed':failures.append('HISTORICAL_APPLICATION_FOR_WRONG_TARGET_GRAPH')
        result['fresh_wrong_target_observations']=changed
        stages.append('same_cookie_wrong_target_is_not_current_application')
        stopped=command('stop_fixture_link');require(stopped['children']['fixture_link'] is not None,'fixture wrong link actually waited')
        expect_graph(set());command('start_matching_link');expect_graph({('source.media','route.speaker')})
        matching=current({('source.media','route.speaker')})
        require(matching['application']=='observed','fresh matching graph should establish current application without replay')
        result['fresh_matching_observation_without_new_intent']=matching
        stages.append('fresh_matching_graph_can_be_observed_without_new_intent_or_apply')
        result['behavioral_failures']=sorted(set(failures))
        require(not failures,'HISTORICAL_APPLICATION_RETAINED_AFTER_ACTUAL_GRAPH_CHANGE')
        result.update(passed=True,classification='QUALIFIED_REAL_DAEMON_CURRENT_OBSERVATION_GREEN')
    except BaseException as error:
        result['failure']=type(error).__name__+': '+str(error)
        if str(error)=='HISTORICAL_APPLICATION_RETAINED_AFTER_ACTUAL_GRAPH_CHANGE' and result.get('behavioral_failures'):
            result['classification']='BEHAVIORAL_HISTORICAL_APPLICATION_RETAINED'
    finally:
        cleanup_errors=[]
        for c in clients:
            try:c.close()
            except BaseException as error:cleanup_errors.append(str(error))
        try:
            require(command('stop')['stopping'],'normal fixture stop')
            fixture.stdin.close();fixture.stdin=None
            stdout,stderr=fixture.communicate(timeout=20)
        except BaseException as error:
            cleanup_errors.append(str(error))
            subprocess.run(D+['kill',cid],env=ENV,capture_output=True,timeout=8)
            if fixture.stdin:fixture.stdin.close();fixture.stdin=None
            stdout,stderr=fixture.communicate(timeout=8)
        result.update(actual_fixture_CLI_wait=fixture.returncode,fixture_stdout_EOF=True,fixture_stderr_EOF=True,remaining_fixture_stdout_hex=stdout.hex(),fixture_stderr_hex=stderr.hex())
        waited=subprocess.run(D+['wait',cid],env=ENV,capture_output=True,check=True,timeout=8)
        result['actual_container_wait']=int(waited.stdout)
        state=json.loads(subprocess.check_output(D+['inspect','--format','{{json .State}}',cid],env=ENV,timeout=8));result['container_state']=state
        subprocess.run(D+['rm',cid],env=ENV,capture_output=True,check=True,timeout=8)
        result['owned_container_absent']=subprocess.run(D+['inspect',cid],env=ENV,capture_output=True,timeout=8).returncode!=0
        result['source_inventory_unchanged']=source_map()==selected
        cleanup=json.loads((out/'fixture-cleanup.json').read_text());result['actual_fixture_cleanup']=cleanup
        result['cleanup_errors']=cleanup_errors
        require(not cleanup_errors and fixture.returncode==0 and result['actual_container_wait']==0 and result['owned_container_absent'] and not stdout and not stderr and result['source_inventory_unchanged'],'actual bounded container/process/PIPE closure')
        require(all(v is not None for v in cleanup['children_reaped'].values()),'all original daemons and Rust authority actually waited')
        result['graph_snapshots']=graph_count;result['completed_ns']=time.monotonic_ns()
        dump(out/'result.json',result)
    print(json.dumps({k:result[k] for k in ['passed','classification','failure','graph_snapshots','actual_fixture_CLI_wait','actual_container_wait','owned_container_absent']}),flush=True)
    return 0 if result['passed'] else 1

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--build',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--image',required=True);args=p.parse_args()
    raise SystemExit(run(args.build.resolve(),args.output.absolute(),args.image))
