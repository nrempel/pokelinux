"""Bounded human-viewer guest handshake; no input synthesis or grants."""
import select
from owned import now
from viewer_guest import GuestInput
from viewer_protocol import read, publish


class HumanGuest(GuestInput):
 def wait(self,path,until,cleanup=False):
  while now()<until:
   (self.cleanup_tick if cleanup else self.tick)()
   value=read(path)
   if value is not None:
    assert value['schema']==1 and value['selection_sha256']==self.sha
    assert now()<until
    return value
   select.select([],[],[],min(.01,max(0,(until-now())/1e9)))
  raise TimeoutError('original human-preview guest cutoff')

 def dwell(self,duration_ns,ready_details):
  assert type(duration_ns)is int and 0<duration_ns<=300_000_000_000
  publish(self.root/'mice.request.json',{'schema':1,'selection_sha256':self.sha})
  host=self.wait(self.root/'host-ready.json',min(self.work,now()+10_000_000_000))
  current=[value for value in host['actual_query_mice']if value.get('absolute')is True and value.get('current')is True and'virtio tablet'in value.get('name','').lower()]
  assert len(current)==1
  started=now();until=min(self.work,started+duration_ns)
  assert until-started>=1_000_000_000
  request={'schema':1,'selection_sha256':self.sha,'requested_dwell_ns':until-started,'guest_until_ns':until}
  publish(self.root/'guest-ready.json',request)
  result={'host_ready':host,'request':request,'ready_details':ready_details,'guest_clock_is_not_host_clock':True,'automated_input_actions':0,'human_client_claimed':False}
  while now()<until:
   self.tick()
   retired=read(self.root/'retired.json')
   if retired is not None:
    assert retired['schema']==1 and retired['selection_sha256']==self.sha
    result['guest_observed_end']='host-retired';break
   select.select([],[],[],min(.025,max(0,(until-now())/1e9)))
  else:result['guest_observed_end']='guest-dwell-expired'
  result['guest_observed_end_ns']=now()
  return result

 def retire(self,until):
  assert until<=self.close
  if not self.retire_sent:
   publish(self.root/'retire.request.json',{'schema':1,'selection_sha256':self.sha,'reason':'guest-ending'})
   self.retire_sent=True
  value=self.wait(self.root/'retired.json',until,cleanup=True)
  assert value['no_future_host_writes']is True and value['remote_event_queues_drained_claim']is False
  return value
