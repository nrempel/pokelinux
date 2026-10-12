package org.omarchy.bridge;
import java.net.Socket;
import java.io.IOException;
import java.util.concurrent.*;
public final class BridgeTransportTest {
 static void require(boolean value){if(!value)throw new AssertionError();}
 public static void main(String[] args)throws Exception{
  Socket raw=new Socket();BridgeTransport transport=new BridgeTransport(raw);
  CountDownLatch closing=new CountDownLatch(1),release=new CountDownLatch(1);
  Socket tls=new Socket(){@Override public void close()throws IOException{
   require(raw.isClosed());closing.countDown();try{release.await();}catch(InterruptedException e){throw new IOException(e);}
  }};
  transport.attach(tls);
  transport.abort();require(raw.isClosed());require(closing.getCount()==1);
  ExecutorService worker=Executors.newSingleThreadExecutor();
  try{
   Future<?> finished=worker.submit(()->transport.finish());
   require(closing.await(1,TimeUnit.SECONDS));
   // Cancellation does not wait for TLS cleanup or take a lock held by it.
   transport.abort();require(transport.isClosed());require(!finished.isDone());
   release.countDown();finished.get(1,TimeUnit.SECONDS);
  }finally{release.countDown();worker.shutdownNow();}
  Socket earlyRaw=new Socket();BridgeTransport early=new BridgeTransport(earlyRaw);early.abort();
  Socket lateTls=new Socket();try{early.attach(lateTls);throw new AssertionError("late attach accepted");}catch(IOException expected){}
  early.finish();require(lateTls.isClosed());
  System.out.println("BridgeTransport: raw abort precedes TLS cleanup, cancellation stays independent, late attachment cleaned up");
 }
}
