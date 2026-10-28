"""One selected fixture case: normal profile request, independent cap0 packets.
The caller exclusively owns radio/AP/namespaces; this does not grant authority.
"""
from pathlib import Path
import hashlib,json,os,stat,time,traceback
from owned import now
import address_consumer as oracle
import worker_caps


def pin(path):
 with Path(path).open('rb')as stream:return {'bytes':Path(path).stat().st_size,'sha256':hashlib.file_digest(stream,'sha256').hexdigest()}


def prepare(run, inputs, selection, work_until):
 source=inputs/selection['address_worker']['relative_path'];assert pin(source)==selection['address_worker']['ELF']
 destination=Path(selection['address_worker']['guest_path']);assert not os.path.lexists(destination)
 for directory in [Path('/'),Path('/usr'),Path('/usr/libexec'),destination.parent]:
  s=directory.lstat();assert stat.S_ISDIR(s.st_mode)and s.st_uid==s.st_gid==0 and not s.st_mode&0o022
 with source.open('rb')as src,destination.open('xb')as dst:
  while block:=src.read(65536):
   assert now()<work_until;dst.write(block)
  dst.flush();os.fsync(dst.fileno())
 destination.chmod(0o755);os.chown(destination,0,0);s=destination.lstat()
 assert stat.S_ISREG(s.st_mode)and s.st_uid==s.st_gid==0 and stat.S_IMODE(s.st_mode)==0o755 and s.st_nlink==1
 assert'security.capability'not in os.listxattr(destination)and pin(destination)==selection['address_worker']['ELF']
 with destination.open('rb')as stream:header=stream.read(20)
 assert header[:6]==b'\x7fELF\x02\x01'and int.from_bytes(header[18:20],'little')==183
 loader=run(['/usr/bin/ldd','-r',str(destination)],'native-normal-profile-worker-loader')
 assert loader.strip()and'not found'not in loader.lower()and'undefined symbol'not in loader.lower()
 return destination


def run_case(*,root,inputs,selection,station,ap,ap_prefix,namespace_binding,outer_scope,work_until,close_until,worker,owned_scope):
 out=root/'address-case';out.mkdir(mode=0o700);gates=inputs/'address-case-gates';gates.mkdir(mode=0o700)
 scope=owned_scope;assert scope.output==out and not scope.children
 peer=None;up=None;down=None;until=None
 result={'schema':1,'passed':False,'peer_ready_qualified':False,'AP_profile_setup_qualified':False,
         'baseline_refusal_qualified':False,'profile_up_qualified':False,'profile_down_qualified':False,
         'profile_up_commands_admitted':0,'profile_down_commands_admitted':0,'UDP_roundtrip_clients_admitted':0,
         'radio_or_address_retry_admitted':False,'Settings_or_user_consent_executed':False}
 selected_ns=namespace_binding()['station_namespace'];selected={'namespace':{k:selected_ns[k]for k in['dev','ino']},'ifindex':station['ifindex'],'name':station['name']}
 ap_ns=namespace_binding()['AP_namespace'];peer_path=ap_prefix[2].split('=',1)[1]
 def pump():outer_scope.pump();outer_scope.check();scope.pump();scope.check();namespace_binding()
 def run(argv,label,end,limit=32768,identity=None):
  pump();assert now()<end
  code,body=scope.run(argv,label,end,limit=limit,identity=identity)
  pump();assert now()<end and code==0,(label,code,body[-600:]);return body,scope.receipts[-1]
 def gate(name,end):
  path=gates/name
  while not path.exists():
   pump();assert now()<end and peer is not None and scope.live(peer);time.sleep(.001)
  value=json.loads(path.read_text());pump();assert now()<end and scope.live(peer);return value
 def selected_inventory(configured,end,label):
  armed=case_armed  # Real original packet-case origin, never a fabricated sample.
  body,actual=run(['/usr/bin/ip','-j','address','show','dev',selected['name']],label+'-GETADDR.log',end,262144)
  oracle.receipt(actual,expected_argv=['/usr/bin/ip','-j','address','show','dev',selected['name']],until=end,armed=armed,code=0,byte_count=len(body),limit=262144)
  value=oracle.kernel_profile(body,selected=selected,configured=configured)
  raw,route_receipt=run(['/usr/bin/cat','/proc/net/route'],label+'-routes.log',end,65536)
  route=oracle.route_profile(raw.decode(),selected,configured);assert now()<end
  return {'address':value,'routes':route,'GETADDR_receipt':actual,'route_receipt':route_receipt}
 def l2(label):
  outputs={};receipts={}
  for key,argv in [('link',['/usr/bin/iw','dev',station['name'],'link']),('AP_station_dump',ap_prefix+['/usr/bin/iw','dev',ap['name'],'station','dump']),('station_roles',['/usr/bin/iw','dev']),('AP_roles',ap_prefix+['/usr/bin/iw','dev'])]:
   body,actual=run(argv,label+'-'+key+'.log',until,262144);outputs[key]=body;receipts[key]=actual
  value=oracle.l2_observation(selected_station=station,selected_AP=ap,outputs=outputs,receipts=receipts,ap_prefix=ap_prefix,armed=case_armed,until=until,namespace_binding=namespace_binding())
  assert now()<until;result[label]={'observation':value,'receipts':receipts,'raw_output_sha256':{key:hashlib.sha256(body).hexdigest()for key,body in outputs.items()}};return value
 def profile(operation):
  evidence=out/(operation+'-preexec.json');fd=os.open(evidence,os.O_RDWR|os.O_CREAT|os.O_EXCL|os.O_CLOEXEC,0o600)
  parent=os.getpid()
  def reduce():
   values=worker_caps.reduce_caps(1<<worker_caps.NET_ADMIN);os.set_inheritable(fd,False)
   raw=json.dumps({'pid':os.getpid(),'ppid':os.getppid(),'caps':values,'observed_at_ns':now(),'evidence_fd_closes_on_exec':not os.get_inheritable(fd)}).encode()
   assert len(raw)<=4096 and os.write(fd,raw)==len(raw)
  armed=now();end=armed+oracle.ACTION_NS;assert end<until
  argv=oracle.argv(str(worker),operation,selected,end);admission=now();result[operation.replace('-','_')+'_commands_admitted']+=1
  try:
   item=scope.start(argv,operation+'.log',oracle.MAX_WIRE,cutoff=end,wait_cutoff=end,identity={'preexec_fn':reduce,'pass_fds':(fd,)})
   code=scope.finish(item,end)
  finally:os.close(fd)
  sample=json.loads(evidence.read_text());assert sample['pid']==item['child'].pid and sample['ppid']==parent and admission<=sample['observed_at_ns']<end and sample['evidence_fd_closes_on_exec']
  caps=sample['caps'];assert all(int(caps[k],16)==1<<worker_caps.NET_ADMIN for k in['CapEff','CapPrm','CapBnd'])and int(caps['CapInh'],16)==int(caps['CapAmb'],16)==0 and caps['NoNewPrivs']=='1'
  body=(out/item['name']).read_bytes();actual=item['wait'];actual['spawn_started_at_ns']=item['spawn_started_at_ns']
  value=oracle.observe_worker(code,body,actual,executable=str(worker),operation=operation,selected=selected,armed_at_ns=armed,admission_ns=admission,elf_sha256=selection['address_worker']['ELF']['sha256'])
  assert now()<end
  if value['known_baseline_refusal']:actual['expected_returncode']=1
  result[operation]={'observation':value,'receipt':actual,'same_PID_preexec_capabilities':sample,'after_exec_capability_sample_claimed':False}
  return value
 def client(mode,payload,end,label):
  args=['/usr/bin/python3','-B',str(inputs/'source/udp_client.py'),mode,payload.hex(),str(end)]
  spawn=now();wait_until=until if mode=='loss'else end
  item=scope.start(args,label+'.log',32768,cutoff=wait_until,wait_cutoff=wait_until,identity={'preexec_fn':lambda:worker_caps.reduce_caps(0)})
  code=scope.finish(item,wait_until);body=(out/item['name']).read_bytes();actual=item['wait'];actual['spawn_started_at_ns']=item['spawn_started_at_ns']
  assert code==0 and now()<wait_until
  value=oracle.decode(body);assert value['passed']and value['mode']==mode and value['payload_hex']==payload.hex()and value['original_until_ns']==end and value['namespace']==selected['namespace']
  assert all(int(v,16)==0 for k,v in value['caps'].items()if k.startswith('Cap'))and value['caps']['NoNewPrivs']=='1'
  if mode=='roundtrip':oracle.observe_packet(value,payload_hex=payload.hex(),actual=actual,expected_argv=args,until=end,armed=spawn,profile_worker=up,selected=selected)
  else:assert value['received_datagrams']==0 and value['completed_at_ns']>=end and value['sent_at_ns']<end
  result[label]={'value':value,'receipt':actual};return value
 try:
  # Actual after-exec guest UID0/cap0 gate I/O before any AP profile or peer.
  probe_until=min(work_until,now()+1_000_000_000)
  probe_argv=['/usr/bin/python3','-B',str(inputs/'source/udp_client.py'),'path-probe',str(gates)]
  body,actual=run(probe_argv,'actual-cap0-guest-gate-permission-preflight.log',probe_until,identity={'preexec_fn':lambda:worker_caps.reduce_caps(0)})
  probe=json.loads(body);assert probe['passed']and probe['uid']==probe['gid']==0 and probe['actual_create_fsync_close_unlink']and probe['probe_absent']
  assert all(int(v,16)==0 for k,v in probe['caps'].items()if k.startswith('Cap'))and probe['caps']['NoNewPrivs']=='1'
  result['guest_cap0_gate_permission_preflight']={'record':probe,'receipt':actual}
  # AP .2/30 is authored fixture setup only, serialized and retired before packets.
  provision_until=min(work_until,now()+1_000_000_000)
  provision_argv=['/usr/bin/python3','-B',str(inputs/'source/wifi_peer.py'),'provision',peer_path,str(provision_until),ap['name'],str(ap['ifindex']),str(ap_ns['dev']),str(ap_ns['ino'])]
  body,actual=run(provision_argv,'AP-fixed-peer-provisioner.log',provision_until)
  provision=json.loads(body);assert provision['passed']and provision['peer_index']==ap['ifindex']and provision['addresses']==[{'family':2,'prefix':30,'ifindex':ap['ifindex'],'local':'192.0.2.2'}]
  result['AP_profile_setup_qualified']=True;result['AP_profile_provisioner']={'value':provision,'receipt':actual}
  case_armed=now();until=case_armed+oracle.CASE_NS;assert until<work_until
  result.update(original_case_armed_at_ns=case_armed,original_case_until_ns=until,target=selected)
  peer_argv=['/usr/bin/python3','-B',str(inputs/'source/wifi_peer.py'),'serve',peer_path,str(gates),str(until)]
  peer=scope.start(peer_argv,'cap0-AP-packet-peer.log',65536,cutoff=until,wait_cutoff=until)
  ready=gate('peer-ready.json',until);assert ready['pid']==peer['child'].pid and ready['uid']==0 and ready['bound']==['192.0.2.2',36042]
  assert all(int(v,16)==0 for k,v in ready['caps'].items()if k.startswith('Cap'))and ready['caps']['NoNewPrivs']=='1'and ready['only_packet_socket_no_admin_descriptors']
  st=Path('/proc/'+str(peer['child'].pid)+'/ns/net').stat();assert(st.st_dev,st.st_ino)==(ap_ns['dev'],ap_ns['ino'])
  assert actual['waited_at_boottime_ns']<peer['spawn_started_at_ns']and actual['stdout_EOF_at_ns']<peer['spawn_started_at_ns']
  result['peer_ready_qualified']=True;result['peer_ready']=ready;result['peer_namespace_matches_retained_AP']=True
  l2('L2-before-profile')
  up=profile('profile-up')
  if up['known_baseline_refusal']:
   result['address_after_refusal']=selected_inventory(False,up['original_until_ns'],'refusal')
   result['station_address_absent_after_refusal']=True
   quiet_armed=now();quiet_end=quiet_armed+oracle.QUIET_NS;assert quiet_end<until
   loss=client('loss',b'PKL1'+bytes([3])+os.getrandom(32),quiet_end,'baseline-fresh-loss-client')
   result['baseline_loss_observation']={'original_armed_at_ns':quiet_armed,'original_until_ns':quiet_end,'actual_client':loss}
   l2('L2-after-baseline-refusal')
   result['baseline_refusal_qualified']=True
  else:
   assert up['profile_completion_confirmed']and up['configured']is True,'uncertain/unconfirmed profile-up; no new action admitted'
   result['address_after_up']=selected_inventory(True,up['original_until_ns'],'configured')
   result['profile_up_qualified']=True
   nonces=[]
   for tag in[1,2]:
    payload=b'PKL1'+bytes([tag])+os.getrandom(32);assert payload not in nonces;nonces.append(payload)
    result['UDP_roundtrip_clients_admitted']+=1;end=min(until,now()+oracle.ACTION_NS);client('roundtrip',payload,end,'nonce-'+str(tag))
   packets=json.loads((gates/'peer-packets.json').read_text());assert len(packets['packets'])==2
   assert[entry['payload_hex']for entry in packets['packets']]==[value.hex()for value in nonces]
   result['peer_packet_receipts']=packets;result['two_independent_nonce_ACKs_qualified']=True
   down=profile('profile-down');assert down['profile_completion_confirmed']and down['configured']is False,'uncertain/unconfirmed profile-down; no quiet or Disconnect admitted'
   result['address_after_removal']=selected_inventory(False,down['original_until_ns'],'removed');result['station_address_absent_after_removal']=True;result['profile_down_qualified']=True
   l2('L2-after-removal-before-quiet')
   payload=b'PKL1'+bytes([3])+os.getrandom(32);quiet_armed=now();quiet_end=quiet_armed+oracle.QUIET_NS;assert quiet_end<until
   request={'original_until_ns':quiet_end,'probe_hex':payload.hex(),'requested_at_ns':quiet_armed};temporary=gates/'quiet-request.tmp';temporary.write_text(json.dumps(request));os.replace(temporary,gates/'quiet-request.json')
   armed=gate('peer-quiet-armed.json',quiet_end);assert armed['original_until_ns']==quiet_end
   loss=client('loss',payload,quiet_end,'fresh-loss-after-removal');quiet=gate('peer-quiet.json',until)
   assert quiet['received_datagrams']==0 and quiet['original_until_ns']==quiet_end and quiet['armed_at_ns']<=loss['sent_at_ns']and quiet['completed_at_ns']>=quiet_end
   assert quiet['quiet_end_actual_EAGAIN']and quiet['quiet_end_EAGAIN_at_ns']>=quiet_end and not(gates/'peer-unexpected-datagram.json').exists()
   result['peer_quiet_qualified']=True;result['peer_quiet']={'request':request,'actual_peer':quiet,'actual_client':loss,'armed_to_final_EAGAIN_ns':quiet['quiet_end_EAGAIN_at_ns']-quiet['armed_at_ns']}
   l2('L2-after-removal-and-quiet');result['L2_remained_associated_through_removal_and_quiet']=True
 finally:
  if peer is not None and scope.live(peer):
   (gates/'stop-peer').touch()
   try:
    assert scope.finish(peer,until)==0
    terminal=json.loads((out/peer['name']).read_text());result['peer_terminal']=terminal
    if result['baseline_refusal_qualified']:assert terminal['packets']==0;result['peer_packets_after_refusal']=0
    else:assert terminal['packets']==2
    assert terminal['peer_stopped']and not(gates/'peer-unexpected-datagram.json').exists()
   except BaseException as error:result['peer_stop_error']=repr(error)
  result['cleanup_errors']=scope.close(close_until);result['receipts']=scope.receipts
  result['all_handles_closed']=all(v['wait']is not None and v['eof']and v['log'].closed and v['child'].stdout.closed and v['pidfd']is None for v in scope.children)
  result['owned_case_custody_qualified']=bool(not result['cleanup_errors']and not result.get('peer_stop_error')and result['all_handles_closed']and len(scope.children)==len(scope.receipts)and all(v['actual_direct_parent_wait']and v['stdout_EOF']and v['wait_before_original_cutoff']and v['EOF_before_original_cutoff']and not v['output_overflow']and v['returncode']==v.get('expected_returncode',0)for v in scope.receipts)and now()<close_until)
  if result['all_handles_closed']:
   saved=out/'gates-evidence';saved.mkdir(mode=0o700);result['gate_evidence']={}
   for name in ['peer-ready.json','peer-packets.json','quiet-request.json','peer-quiet-armed.json','peer-quiet.json','peer-unexpected-datagram.json']:
    source=gates/name
    if source.exists():
     info=source.lstat();assert stat.S_ISREG(info.st_mode)and info.st_uid==0 and info.st_size<=65536
     body=source.read_bytes();(saved/name).write_bytes(body);result['gate_evidence'][name]={'bytes':len(body),'sha256':hashlib.sha256(body).hexdigest()}
  result['completed_at_ns']=now();(out/'case-result.json').write_text(json.dumps(result,indent=2)+'\n')
 assert result['owned_case_custody_qualified']and(result['baseline_refusal_qualified']or(result['profile_up_qualified']and result['profile_down_qualified']and result['peer_quiet_qualified']))
 return result
