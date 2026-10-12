package org.omarchy.bridge;
import java.io.IOException;
import java.util.function.LongSupplier;
final class DialPolicyTest {
 static final String G="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",ID="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",ID2="cccccccccccccccccccccccccccccccc";
 static final class Fixture implements DialPolicy.Phone,DialAttempt.Store,LongSupplier {
  long time,revision=1;String unavailable=null,account="sim1",persisted="",classification=null;boolean failStore,failDispatch,commit=true;
  int calls;String marker;Runnable onStore=()->{};
  public long getAsLong(){return time;}
  public String load(){return persisted;}
  public void save(String value)throws IOException{if(failStore)throw new IOException();persisted=value;onStore.run();}
  public String unavailable(){return unavailable;}
  public Object account(){return account;}
  public long revision(){return revision;}
  public String classify(String number){return classification;}
  public boolean commitAllowed(){return commit;}
  public void dispatch(String number,Object selected,String token){calls++;marker=token;if(persisted.isEmpty())throw new AssertionError("Dispatch before durable reservation");if(failDispatch)throw new IllegalStateException();}
 }
 static void eq(Object a,Object b){if(!a.equals(b))throw new AssertionError(a+" != "+b);}
 static DialPolicy policy(Fixture f,int capacity)throws Exception{return new DialPolicy(G,capacity,2,new DialAttempt(f),f);}
 static String dial(DialPolicy p,Fixture f,String id,String offer,String number){return p.execute(f,id,G,offer,number);}
 public static void main(String[] args)throws Exception{
  Fixture f=new Fixture();DialPolicy p=policy(f,3);String offer=p.offer(f);
  eq(dial(p,f,ID,offer,"5550100"),"invocation_attempted:returned");eq(f.calls,1);
  // No call callback has arrived; invocation return must not rearm dialing.
  eq(p.pending(),true);eq(p.offer(f),"");
  f.time=100000;f.unavailable="disabled";
  eq(dial(p,f,ID,offer,"5550100"),"invocation_attempted:returned");eq(f.calls,1);
  eq(dial(p,f,ID,offer,"5550101"),"not_invoked:replay_conflict");
  f.unavailable=null;eq(dial(p,f,ID2,offer,"5550100"),"not_invoked:pending");
  p.terminal("dddddddddddddddddddddddddddddddd");eq(p.pending(),true);p.terminal(f.marker);eq(p.pending(),false);
  offer=p.offer(f);eq(dial(p,f,"dddddddddddddddddddddddddddddddd",offer,"5550100"),"invocation_attempted:returned");eq(f.calls,2);
  p.terminal(f.marker);offer=p.offer(f);eq(dial(p,f,"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",offer,"5550100"),"not_invoked:capacity");
  for(String fault:new String[]{"expired","revision","account","permission","classifier","commit","storage","after_storage","dispatch"}){
   Fixture x=new Fixture();DialPolicy q=policy(x,2);String token=q.offer(x);
   if(fault.equals("expired"))x.time=5000;
   if(fault.equals("revision"))x.revision++;
   if(fault.equals("account"))x.account="sim2";
   if(fault.equals("permission"))x.unavailable="permission";
   if(fault.equals("classifier"))x.classification="emergency";
   if(fault.equals("commit"))x.commit=false;
   if(fault.equals("storage"))x.failStore=true;
   if(fault.equals("after_storage"))x.onStore=()->{x.time=5000;};
   if(fault.equals("dispatch"))x.failDispatch=true;
   String result=dial(q,x,ID,token,"5550100");
   eq(x.calls,fault.equals("dispatch")?1:0);
   if(fault.equals("dispatch")){eq(result,"outcome_unknown:dispatch_error");eq(q.pending(),true);}
   else if(!result.startsWith("not_invoked:"))throw new AssertionError(fault+": "+result);
   eq(dial(q,x,ID,token,"5550100"),result); // Repeats never turn a failed operation into a call.
  }
  Fixture x=new Fixture();DialPolicy q=policy(x,2);String a=q.offer(x),b=q.offer(x);eq(q.offer(x),"");
  eq(dial(q,x,ID,a,"5550100"),"invocation_attempted:returned");eq(dial(q,x,ID2,b,"5550100"),"not_invoked:pending");eq(x.calls,1);
  System.out.println("DialPolicy: duplicate, conflict, two-call recovery, quotas and nine admission/dispatch failures passed");
 }
}
