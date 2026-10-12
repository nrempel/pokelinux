package org.omarchy.bridge;

import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicReference;

public final class CallChangesTest {
 static void require(boolean value){if(!value)throw new AssertionError();}
 static void awaiting(Thread thread)throws InterruptedException{
  long deadline=System.nanoTime()+TimeUnit.SECONDS.toNanos(1);
  while(thread.getState()!=Thread.State.TIMED_WAITING&&System.nanoTime()<deadline)Thread.sleep(1);
  require(thread.getState()==Thread.State.TIMED_WAITING);
 }
 public static void main(String[] args)throws Exception {
  CallChanges changes=new CallChanges();
  Object before=changes.cursor();
  changes.changed();
  require(changes.awaitAfter(before,100)); // callback before waiter registration
  Object current=changes.cursor();
  require(!changes.awaitAfter(current,10));
  CountDownLatch entered=new CountDownLatch(2),finished=new CountDownLatch(2);
  AtomicReference<Throwable> failure=new AtomicReference<>();
  final Object observed=current;
  Runnable observer=()->{try{entered.countDown();require(changes.awaitAfter(observed,1000));}catch(Throwable e){failure.set(e);}finally{finished.countDown();}};
  Thread first=new Thread(observer),second=new Thread(observer);first.start();second.start();
  require(entered.await(1,TimeUnit.SECONDS));awaiting(first);awaiting(second);changes.changed();
  require(finished.await(1,TimeUnit.SECONDS));first.join();second.join();
  if(failure.get()!=null)throw new AssertionError(failure.get());
  before=changes.cursor();for(int i=0;i<10000;i++)changes.changed();
  require(changes.awaitAfter(before,10)); // coalesces; no event queue to fill
  current=changes.cursor();require(!changes.awaitAfter(current,10));
  AtomicBoolean interrupted=new AtomicBoolean();CountDownLatch waiting=new CountDownLatch(1);
  final Object latest=current;
  Thread stop=new Thread(()->{try{waiting.countDown();changes.awaitAfter(latest,30000);}catch(InterruptedException expected){interrupted.set(true);}});
  stop.start();require(waiting.await(1,TimeUnit.SECONDS));stop.interrupt();stop.join(1000);
  require(!stop.isAlive()&&interrupted.get());
  for(long invalid:new long[]{0,-1,30001,Long.MAX_VALUE}){
   try{changes.awaitAfter(latest,invalid);throw new AssertionError("invalid timeout accepted");}
   catch(IllegalArgumentException expected){}
  }
  System.out.println("CallChanges: pre-wait changes, multiple observers, coalescing, timeout, interruption and bounds passed");
 }
}
