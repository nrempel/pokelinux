package org.omarchy.enrollmentprobe;
import android.app.*;
import android.content.*;
import android.os.*;
import android.util.Base64;
import java.lang.reflect.*;
/** Same-signer offline test only. Exercises the real production enrollment adapter. */
public final class Probe extends Instrumentation {
 private Bundle options;
 @Override public void onCreate(Bundle args){super.onCreate(args);options=args;start();}
 @Override public void onStart(){
  Bundle result=new Bundle();
  try{
   Context target=getTargetContext();Class<?> enrollment=target.getClassLoader().loadClass("org.omarchy.bridge.Enrollment");
   String op=options.getString("op","request");
   if(op.startsWith("journal_")){
    ClassLoader loader=target.getClassLoader();
    Class<?> journal=loader.loadClass("org.omarchy.bridge.DialJournal"),attempt=loader.loadClass("org.omarchy.bridge.DialAttempt"),store=loader.loadClass("org.omarchy.bridge.DialAttempt$Store");
    Constructor<?> jc=journal.getDeclaredConstructor(Context.class);jc.setAccessible(true);
    Constructor<?> ac=attempt.getDeclaredConstructor(store);ac.setAccessible(true);Object value=ac.newInstance(jc.newInstance(target));
    Method pending=attempt.getDeclaredMethod("pending"),reserve=attempt.getDeclaredMethod("reserve",String.class),terminal=attempt.getDeclaredMethod("terminal",String.class);
    pending.setAccessible(true);reserve.setAccessible(true);terminal.setAccessible(true);
    if(op.equals("journal_begin")){
     if((Boolean)pending.invoke(value))throw new IllegalStateException("Test journal is not empty");
     if(!(Boolean)reserve.invoke(value,"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))throw new IllegalStateException("Reservation failed");
     result.putString("outcome","RESERVED");
    }else{
     if(op.equals("journal_unrelated"))terminal.invoke(value,"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
     if(op.equals("journal_end"))terminal.invoke(value,"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
     result.putString("outcome",(Boolean)pending.invoke(value)?"PENDING":op.equals("journal_end")?"RESOLVED":"EMPTY");
    }
   }else if(op.equals("request")){
    byte[] request=(byte[])enrollment.getMethod("request",Context.class).invoke(null,target);
    result.putString("request",Base64.encodeToString(request,Base64.NO_WRAP));
    result.putString("non_exportable",String.valueOf(enrollment.getMethod("keyIsNonExportable",Context.class).invoke(null,target)));
   }else if(op.equals("enable")){
    target.getSharedPreferences("gateway",0).edit().putBoolean("enabled",true).commit();
    startActivitySync(new Intent().setClassName("org.omarchy.bridge","org.omarchy.bridge.DialActivity").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
    target.startForegroundService(new Intent().setClassName("org.omarchy.bridge","org.omarchy.bridge.GatewayService"));
    result.putString("outcome","ENABLED");
   }else if(op.equals("disable")){
    target.getSharedPreferences("gateway",0).edit().putBoolean("enabled",false).putBoolean("controls",false).commit();
    target.stopService(new Intent().setClassName("org.omarchy.bridge","org.omarchy.bridge.GatewayService"));
    result.putString("outcome","DISABLED");
   }else if(op.equals("import")){
    byte[] bytes=Base64.decode(options.getString("bundle"),Base64.NO_WRAP);
    enrollment.getMethod("inspect",Context.class,byte[].class).invoke(null,target,bytes);
    enrollment.getMethod("activate",Context.class,byte[].class).invoke(null,target,bytes);
    result.putString("outcome","ENROLLED");
   }
   finish(0,result);
  }catch(Exception e){Throwable cause=e instanceof InvocationTargetException?e.getCause():e;result.putString("outcome",cause.getClass().getSimpleName());finish(1,result);}
 }
}
