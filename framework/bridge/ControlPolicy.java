package org.omarchy.bridge;
import java.util.HashMap;
import java.util.Map;
/** Main-thread command ledger. Entries are never evicted within a process generation. */
final class ControlPolicy {
 interface Phone {
  boolean enabled();
  boolean commitAllowed();
  boolean available();
  int state(int id);
  long revision(int id);
  void consumeRevision(int id);
  void dispatch(int id,String action);
 }
 private final String generation;
 private final int capacity;
 private final Map<String,String[]> ledger=new HashMap<>();
 ControlPolicy(String generation,int capacity){this.generation=generation;this.capacity=capacity;}
 String execute(Phone phone,String id,String expectedGeneration,int call,long expectedRevision,int expectedState,String action,long deadline,long now){
  if(!phone.enabled())return "rejected:disabled";
  if(!generation.equals(expectedGeneration))return "rejected:stale_generation";
  if(now>=deadline)return "rejected:deadline";
  String command=expectedGeneration+":"+call+":"+expectedRevision+":"+expectedState+":"+action;
  String[] previous=ledger.get(id);
  if(previous!=null)return previous[0].equals(command)?previous[1]:"rejected:replay_conflict";
  if(!phone.available())return "rejected:unavailable";
  int state=phone.state(call);
  if(state<0)return "rejected:missing_call";
  boolean allowed=(action.equals("answer")||action.equals("reject"))?state==2:
   action.equals("disconnect")&&(state==1||state==3||state==4||state==9);
  if(state!=expectedState||!allowed)return "rejected:stale_state";
  if(phone.revision(call)!=expectedRevision)return "rejected:stale_revision";
  if(ledger.size()>=capacity)return "rejected:capacity";
  if(!phone.commitAllowed())return "rejected:deadline";
  String[] entry=new String[]{command,"unknown:dispatch_error"};
  ledger.put(id,entry); // Reserve before dispatch: an exception must never permit a replay.
  phone.consumeRevision(call);
  try{phone.dispatch(call,action);entry[1]="submitted:accepted";}catch(RuntimeException ignored){}
  return entry[1];
 }
}
