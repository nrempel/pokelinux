package org.omarchy.bridge;
import java.util.*;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.*;
public final class CallWatchTest {
 static void require(boolean value){if(!value)throw new AssertionError();}
 static RingingRegistry.Snapshot snapshot(String generation,long revision){return new RingingRegistry.Snapshot(generation,"detached",revision,Collections.<Integer,String>emptyMap());}
 public static void main(String[] args)throws Exception{
  String generation="11111111111111111111111111111111";
  CallChanges changes=new CallChanges();AtomicInteger samples=new AtomicInteger();
  RingingRegistry.Snapshot result=CallWatch.read(changes,()->{
   if(samples.incrementAndGet()==1){changes.changed();return snapshot(generation,1);}
   return snapshot(generation,2);
  },()->{},()->System.nanoTime()/1000000,generation,1,1000,Long.MAX_VALUE);
  require(samples.get()==2&&result.revision==2);
  samples.set(0);
  result=CallWatch.read(changes,()->{samples.incrementAndGet();return snapshot(generation,1);},()->{},()->System.nanoTime()/1000000,generation,1,20,Long.MAX_VALUE);
  require(samples.get()>=2&&result.revision==1);
  samples.set(0);
  result=CallWatch.read(changes,()->{samples.incrementAndGet();return snapshot(generation,1);},()->{},()->System.nanoTime()/1000000,null,0,25000,Long.MAX_VALUE);
  require(samples.get()==1);
  samples.set(0);
  try{CallWatch.read(changes,()->{samples.incrementAndGet();return snapshot(generation,1);},()->{throw new java.io.IOException("expired");},()->0L,generation,1,0,Long.MAX_VALUE);throw new AssertionError();}catch(java.io.IOException expected){}
  require(samples.get()==0);
  AtomicInteger checks=new AtomicInteger();
  try{CallWatch.read(changes,()->snapshot(generation,1),()->{if(checks.incrementAndGet()==2)throw new java.io.IOException("expired during sample");},()->0L,generation,1,0,Long.MAX_VALUE);throw new AssertionError();}catch(java.io.IOException expected){}
  samples.set(0);
  ExecutorService worker=Executors.newSingleThreadExecutor();
  try{
   Future<RingingRegistry.Snapshot> late=worker.submit(()->CallWatch.read(changes,()->{samples.incrementAndGet();return snapshot(generation,1);},()->{},()->4000L,generation,1,25000,5000));
   result=late.get(1,TimeUnit.SECONDS);
   require(samples.get()==1&&result.revision==1);
  }finally{worker.shutdownNow();}
  System.out.println("CallWatch: pre-wait wake, timeout resampling, initial snapshot, final-sample headroom and expiry checks passed");
 }
}
