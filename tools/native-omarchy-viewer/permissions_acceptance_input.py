"""One finite acceptance handoff to an external RFB peer; never guest input injection."""
from owned import now
from human_guest import HumanGuest
from viewer_protocol import publish, digest


class AcceptanceInput(HumanGuest):
 def begin(self,duration_ns):
  assert type(duration_ns)is int and 0<duration_ns<=300_000_000_000
  publish(self.root/'mice.request.json',{'schema':1,'selection_sha256':self.sha})
  host=self.wait(self.root/'host-ready.json',min(self.work,now()+10_000_000_000))
  assert len([r for r in host['actual_query_mice']if r.get('absolute')is True and r.get('current')is True and'virtio tablet'in r.get('name','').lower()])==1
  started=now();until=min(self.work,started+duration_ns)
  publish(self.root/'guest-ready.json',{'schema':1,'selection_sha256':self.sha,'requested_dwell_ns':until-started,'guest_until_ns':until})
  connected=self.wait(self.root/'acceptance-connected.json',min(until,now()+10_000_000_000))
  assert connected['connected']is True
  self.session_until=until
  return {'host_ready':host,'external_connection':connected,'guest_until_ns':until,'independent_guest_and_host_clocks':True}

 def perform(self,point,until,action='click'):
  assert action in('click','alt-tab','close','page-down','down','return')and not self.failed and self.seq<24
  assert now()<until<=min(self.work,self.session_until)
  assert(point is None)==(action!='click')
  if point is not None:assert len(point)==2 and all(type(v)is int for v in point)and 0<point[0]<1080 and 0<point[1]<2424
  self.seq+=1;prefix=f'accept-{self.seq:02d}'
  request={'schema':1,'selection_sha256':self.sha,'seq':self.seq,'action':action,'point':point,'guest_until_ns':until}
  try:
   publish(self.root/(prefix+'.request.json'),request)
   ready=self.wait(self.root/(prefix+'.ready.json'),until)
   assert ready['seq']==self.seq and ready['request_sha256']==digest(request)
   sampled=now();remaining=until-sampled;assert remaining>100_000_000
   publish(self.root/(prefix+'.commit.json'),{'schema':1,'selection_sha256':self.sha,'seq':self.seq,'request_sha256':digest(request),'host_ready_anchor_ns':ready['host_ready_anchor_ns'],'guest_sample_ns':sampled,'guest_until_ns':until,'remaining_ns':remaining})
   done=self.wait(self.root/(prefix+'.done.json'),until)
   assert done['seq']==self.seq and done['request_sha256']==digest(request)and done['outcome']=='sent_once'and now()<until
   value={'request':request,'ready':ready,'done':done,'completion_guest_ns':now()};self.actions.append(value);return value
  except BaseException:
   self.failed=True
   publish(self.root/'acceptance-abort.json',{'schema':1,'selection_sha256':self.sha,'failed_seq':self.seq})
   raise

 def finish_acceptance(self):
  publish(self.root/'acceptance-finished.json',{'schema':1,'selection_sha256':self.sha,'actions':self.seq})
  retired=self.wait(self.root/'retired.json',min(self.work,now()+10_000_000_000))
  assert retired['ending']=='client-disconnected'and retired['operational_success']is True
  return retired
