package org.omarchy.provision;
import android.app.*;
import android.content.*;
import android.os.*;
import java.io.*;
/** Test APK only. Not included in the production bridge. */
public final class Provision extends Instrumentation {
 private Bundle options;
 @Override public void onCreate(Bundle args){super.onCreate(args);options=args;start();}
 @Override public void onStart(){
  Bundle result=new Bundle();
  try{
   Context target=getTargetContext();
   for(String name:new String[]{"identity.p12","trust.p12","tls-password","client-sha256"}){
    try(InputStream in=getContext().getAssets().open(name);OutputStream out=new FileOutputStream(new File(target.getNoBackupFilesDir(),name))){
     byte[] buffer=new byte[4096];int count;while((count=in.read(buffer))!=-1)out.write(buffer,0,count);
    }
   }
   if(options!=null&&options.containsKey("peer_pin")){
    String pin=options.getString("peer_pin");if(!pin.matches("[0-9a-f]{64}"))throw new IllegalArgumentException("pin");
    try(OutputStream out=new FileOutputStream(new File(target.getNoBackupFilesDir(),"client-sha256"))){out.write(pin.getBytes("US-ASCII"));}
   }
   target.getSharedPreferences("gateway",0).edit().putBoolean("enabled",true).putBoolean("boot",true).commit();
   Intent activity=new Intent().setClassName("org.omarchy.bridge","org.omarchy.bridge.DialActivity").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
   startActivitySync(activity);
   target.startForegroundService(new Intent().setClassName("org.omarchy.bridge","org.omarchy.bridge.GatewayService"));
   result.putString("outcome","PROVISIONED");finish(0,result);
  }catch(Exception e){result.putString("outcome",e.toString());finish(1,result);}
 }
}
