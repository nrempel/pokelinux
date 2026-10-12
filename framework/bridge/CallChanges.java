package org.omarchy.bridge;

/** Internal wake signal only; a cursor is never a command authority or wire ID. */
final class CallChanges {
 private Object current=new Object();
 synchronized Object cursor(){return current;}
 synchronized void changed(){current=new Object();notifyAll();}
 synchronized boolean awaitAfter(Object cursor,long timeoutMillis)throws InterruptedException{
  if(cursor==null||timeoutMillis<1||timeoutMillis>30000)throw new IllegalArgumentException("invalid observation wait");
  final long deadline=System.nanoTime()+timeoutMillis*1000000L;
  while(cursor==current){
   long remaining=deadline-System.nanoTime();
   if(remaining<=0)return false;
   wait(remaining/1000000L,(int)(remaining%1000000L));
  }
  return true;
 }
}
