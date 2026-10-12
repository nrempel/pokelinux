package org.omarchy.bridge;
public final class ControlPolicyTest {
 static final String GEN="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", ID="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
 static class Phone implements ControlPolicy.Phone {
  boolean enabled=true, available=true; int state=2, dispatches; long revision=1; boolean fail, cancelled;
  public boolean commitAllowed(){return !cancelled;}
  public boolean enabled(){return enabled;}
  public boolean available(){return available;}
  public long revision(int id){return revision;}
  public void consumeRevision(int id){revision++;}
  public int state(int id){return id==7?state:-1;}
  public void dispatch(int id,String action){dispatches++;if(fail)throw new IllegalStateException();}
 }
 static int assertions;
 static void equal(Object a,Object b){assertions++;if(!a.equals(b))throw new AssertionError(a+" != "+b);}
 static String run(ControlPolicy p,Phone phone,String id,String gen,int call,int state,String action,long now){return p.execute(phone,id,gen,call,1,state,action,100,now);}
 public static void main(String[] args){
  Phone cancelled=new Phone();cancelled.cancelled=true;
  equal(run(new ControlPolicy(GEN,2),cancelled,ID,GEN,7,2,"answer",1),"rejected:deadline");equal(cancelled.dispatches,0);
  Phone phone=new Phone();ControlPolicy p=new ControlPolicy(GEN,2);
  equal(run(p,phone,ID,GEN,7,2,"answer",1),"submitted:accepted");
  equal(run(p,phone,ID,GEN,7,2,"answer",1),"submitted:accepted");equal(phone.dispatches,1);
  equal(run(p,phone,ID,GEN,7,2,"reject",1),"rejected:replay_conflict");
  phone.available=false;
  equal(run(p,phone,ID,GEN,7,2,"answer",1),"submitted:accepted");
  equal(run(p,phone,ID,GEN,7,2,"reject",1),"rejected:replay_conflict");equal(phone.dispatches,1);
  phone.available=true;
  phone.enabled=false;equal(run(p,phone,ID,GEN,7,2,"answer",1),"rejected:disabled");phone.enabled=true;
  equal(run(p,phone,"c", "old",7,2,"answer",1),"rejected:stale_generation");
  equal(run(p,phone,"c",GEN,7,2,"answer",100),"rejected:deadline");
  phone.available=false;equal(run(p,phone,"c",GEN,7,2,"answer",1),"rejected:unavailable");phone.available=true;
  equal(run(p,phone,"c",GEN,8,2,"answer",1),"rejected:missing_call");
  phone.state=4;equal(run(p,phone,"c",GEN,7,2,"answer",1),"rejected:stale_state");
  equal(run(p,phone,"c",GEN,7,4,"answer",1),"rejected:stale_state");
  equal(p.execute(phone,"rev",GEN,7,1,4,"disconnect",100,1),"rejected:stale_revision");
  phone.revision=1;phone.fail=true;equal(run(p,phone,"c",GEN,7,4,"disconnect",1),"unknown:dispatch_error");
  equal(run(p,phone,"c",GEN,7,4,"disconnect",1),"unknown:dispatch_error");equal(phone.dispatches,2);
  phone.revision=1;equal(run(p,phone,"d",GEN,7,4,"disconnect",1),"rejected:capacity");
  System.out.println("ControlPolicy: "+assertions+" assertions passed");
 }
}
