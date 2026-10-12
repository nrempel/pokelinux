package org.omarchy.bridge;
import android.content.*;
/** Start only after unlocked boot and prior explicit activation. */
public final class BootReceiver extends BroadcastReceiver {
 @Override public void onReceive(Context context,Intent intent){
  String action=intent.getAction();
  if(!Intent.ACTION_BOOT_COMPLETED.equals(action)&&!Intent.ACTION_MY_PACKAGE_REPLACED.equals(action))return;
  if(!context.getSharedPreferences("gateway",0).getBoolean("enabled",false)||!context.getSharedPreferences("gateway",0).getBoolean("boot",false))return;
  if(!GatewayIdentity.exists(context))return;
  try{context.startForegroundService(new Intent(context,GatewayService.class));}
  catch(IllegalStateException|SecurityException e){android.util.Log.w("OmarchyBridge","Automatic start unavailable");}
 }
}
