package org.omarchy.bridge;
import java.util.concurrent.*;

/** Two bounded, unqueued lanes: one held watch and one current-state confirmation. */
final class WatchWorkers {
 private final ThreadPoolExecutor held=lane(),confirmation=lane();
 private static ThreadPoolExecutor lane(){return new ThreadPoolExecutor(1,1,0,TimeUnit.SECONDS,new SynchronousQueue<Runnable>());}
 void execute(int waitMs,Runnable task){
  if(waitMs<0||waitMs>25000)throw new IllegalArgumentException("Invalid watch wait");
  (waitMs==0?confirmation:held).execute(task);
 }
 void shutdownNow(){held.shutdownNow();confirmation.shutdownNow();}
}
