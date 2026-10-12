package org.omarchy.bridge;
import android.content.*;
/** Call-specific notification action. Stale process/registry handles cannot target a later call. */
public final class CallActionReceiver extends BroadcastReceiver {
 @Override public void onReceive(Context context,Intent intent){
  try{CallService.localAction(intent.getStringExtra("generation"),intent.getIntExtra("call_id",-1),intent.getStringExtra("action"));}
  catch(RuntimeException ignored){}
 }
}
