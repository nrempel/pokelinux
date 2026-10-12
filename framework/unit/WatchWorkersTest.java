package org.omarchy.bridge;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicInteger;
public final class WatchWorkersTest {
 static void require(boolean value){if(!value)throw new AssertionError();}
 static void rejected(Runnable action){try{action.run();throw new AssertionError("accepted excess watch");}catch(RejectedExecutionException expected){}}
 public static void main(String[] args)throws Exception{
  WatchWorkers workers=new WatchWorkers();
  CountDownLatch longStarted=new CountDownLatch(1),shortStarted=new CountDownLatch(1),blocked=new CountDownLatch(1),interrupted=new CountDownLatch(2);
  AtomicInteger excess=new AtomicInteger();
  try{
   workers.execute(25000,()->{longStarted.countDown();try{blocked.await();}catch(InterruptedException expected){interrupted.countDown();}});
   require(longStarted.await(1,TimeUnit.SECONDS));
   rejected(()->workers.execute(1,()->excess.incrementAndGet()));
   // A parked positive-wait request cannot take the zero-wait confirmation slot.
   workers.execute(0,()->{shortStarted.countDown();try{blocked.await();}catch(InterruptedException expected){interrupted.countDown();}});
   require(shortStarted.await(1,TimeUnit.SECONDS));
   rejected(()->workers.execute(0,()->excess.incrementAndGet()));
   rejected(()->workers.execute(1000,()->excess.incrementAndGet()));
   workers.shutdownNow();require(interrupted.await(1,TimeUnit.SECONDS));
   rejected(()->workers.execute(0,()->excess.incrementAndGet()));
   rejected(()->workers.execute(25000,()->excess.incrementAndGet()));
   require(excess.get()==0);
  }finally{workers.shutdownNow();blocked.countDown();}
  System.out.println("WatchWorkers: reserved confirmation capacity, no queues, bounded saturation and shutdown passed");
 }
}
