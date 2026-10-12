package org.omarchy.bridge;
import android.content.Context;
import android.content.pm.PackageManager;
import android.telephony.*;
import java.util.concurrent.*;
/** A new framework observation per request. No cached idle value or dialer role. */
final class PhoneObservation {
 static int read(Context context){
  if(context.checkSelfPermission("android.permission.READ_PHONE_STATE")!=PackageManager.PERMISSION_GRANTED)return -1;
  final CountDownLatch received=new CountDownLatch(1);
  final int[] state={-1};
  class Observer extends TelephonyCallback implements TelephonyCallback.CallStateListener {
   public void onCallStateChanged(int value){state[0]=value;received.countDown();}
  }
  Observer observer=new Observer();
  TelephonyManager phone=context.getSystemService(TelephonyManager.class);
  if(phone==null)return -1;
  boolean registered=false;
  try{
   phone.registerTelephonyCallback(context.getMainExecutor(),observer);registered=true;
   if(!received.await(1,TimeUnit.SECONDS))return -1;
   if(context.checkSelfPermission("android.permission.READ_PHONE_STATE")!=PackageManager.PERMISSION_GRANTED)return -1;
   return state[0]>=0&&state[0]<=2?state[0]:-1;
  }catch(SecurityException|IllegalStateException e){return -1;}
  catch(InterruptedException e){Thread.currentThread().interrupt();return -1;}
  finally{if(registered)try{phone.unregisterTelephonyCallback(observer);}catch(RuntimeException ignored){}}
 }
}
