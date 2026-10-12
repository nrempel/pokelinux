package org.omarchy.bridge;
import java.util.concurrent.atomic.*;
public final class AdmissionTest {
 static void require(boolean value){if(!value)throw new AssertionError();}
 public static void main(String[] args){
  AtomicLong elapsed=new AtomicLong(1),wall=new AtomicLong(1);AtomicBoolean closed=new AtomicBoolean();
  Admission a=new Admission(elapsed::get,wall::get,closed::get,10,20);
  a.cancel();require(!a.commit());
  a=new Admission(elapsed::get,wall::get,closed::get,10,20);require(a.commit());a.cancel();require(!a.commit());
  elapsed.set(10);a=new Admission(elapsed::get,wall::get,closed::get,10,20);require(!a.commit());elapsed.set(1);
  wall.set(20);a=new Admission(elapsed::get,wall::get,closed::get,10,20);require(!a.commit());wall.set(1);
  closed.set(true);a=new Admission(elapsed::get,wall::get,closed::get,10,20);require(!a.commit());closed.set(false);
  AtomicInteger reads=new AtomicInteger();a=new Admission(elapsed::get,wall::get,()->reads.incrementAndGet()>1,10,20);require(!a.commit());
  reads.set(0);a=new Admission(()->reads.incrementAndGet()==1?1:10,wall::get,closed::get,10,20);require(!a.commit());
  reads.set(0);a=new Admission(elapsed::get,()->reads.incrementAndGet()==1?1:20,closed::get,10,20);require(!a.commit());
  System.out.println("Admission: cancellation, commitment, expiry, closure and post-CAS races passed");
 }
}
