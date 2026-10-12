package org.omarchy.bridge;
import java.util.concurrent.Callable;
import java.util.function.LongSupplier;
/** Read-only long polling. The caller supplies fresh main-looper sampling. */
final class CallWatch {
 interface Guard {void check()throws Exception;}
 static RingingRegistry.Snapshot read(CallChanges changes,Callable<RingingRegistry.Snapshot> sample,
   Guard guard,LongSupplier elapsed,String generation,long revision,int waitMs,long exchangeDeadline)throws Exception{
  if(waitMs<0||waitMs>25000)throw new IllegalArgumentException("Invalid wait");
  long started=elapsed.getAsLong();
  // Reserve two seconds for the final main-looper sample and half a second for
  // serialization/write. This is headroom, not a scheduler latency guarantee.
  long hold=Math.min(waitMs,Math.max(0,exchangeDeadline-started-2500));
  final long end=started+hold;
  for(;;){
   guard.check();
   // Capture before sampling: a publication during the sample cannot be lost
   // between comparing the wire cursor and registering the monitor wait.
   Object wake=changes.cursor();
   RingingRegistry.Snapshot current=sample.call();
   guard.check();
   long remaining=end-elapsed.getAsLong();
   if(!current.generation.equals(generation)||current.revision!=revision||remaining<=0)return current;
   changes.awaitAfter(wake,Math.min(remaining,25000));
   // Always sample again, including after timeout. Never send cached state as
   // evidence that Android's main looper remains responsive.
  }
 }
}
