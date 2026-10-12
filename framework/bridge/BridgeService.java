package org.omarchy.bridge;
import android.app.Service;
import android.app.role.RoleManager;
import android.content.Intent;
import android.os.*;
import android.telephony.TelephonyManager;
import android.telephony.TelephonyCallback;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import android.telecom.Call;
import org.json.*;
/** Small versioned Binder boundary; no platform signature or hidden APIs. */
public final class BridgeService extends Service {
 private final Binder endpoint=new Binder(){
  @Override protected boolean onTransact(int code,Parcel request,Parcel reply,int flags) throws RemoteException {
   if(code==INTERFACE_TRANSACTION){reply.writeString("org.omarchy.bridge.v1");return true;}
   enforceCallingPermission("org.omarchy.bridge.ACCESS","Bridge caller must be signed by the bridge owner");
   request.enforceInterface("org.omarchy.bridge.v1");
   if(request.readInt()!=1)throw new IllegalArgumentException("UNSUPPORTED_VERSION");
   request.enforceNoDataAvail();
   String value;
   long identity=Binder.clearCallingIdentity();
   try { switch(code){
    case 1:value=status();break;
    case 2:
     enforcePermission("android.permission.READ_PHONE_STATE",android.os.Process.myPid(),android.os.Process.myUid(),"Phone-state permission is required");
     value=Integer.toString(getSystemService(TelephonyManager.class).getCallState());break;
    case 3:value=observePhoneState();break;
    default:return false;
   }} finally {Binder.restoreCallingIdentity(identity);}
   reply.writeNoException();reply.writeString(value);return true;
  }
 };
 private String observePhoneState(){
  final CountDownLatch observed=new CountDownLatch(1);
  final int[] state={-1};
  class Observer extends TelephonyCallback implements TelephonyCallback.CallStateListener {
   @Override public void onCallStateChanged(int value){state[0]=value;observed.countDown();}
  }
  TelephonyManager phone=getSystemService(TelephonyManager.class);
  Observer observer=new Observer();
  // Framework enforces READ_PHONE_STATE here; no local substitute for this check.
  phone.registerTelephonyCallback(getMainExecutor(),observer);
  try {
   if(!observed.await(5,TimeUnit.SECONDS))throw new IllegalStateException("NO_FRAMEWORK_OBSERVATION");
   return Integer.toString(state[0]);
  }catch(InterruptedException e){Thread.currentThread().interrupt();throw new IllegalStateException("OBSERVER_INTERRUPTED");}
  finally{phone.unregisterTelephonyCallback(observer);}
 }
 private String status(){
  try {
   JSONObject r=new JSONObject();r.put("version",1);r.put("service_uid",android.os.Process.myUid());r.put("service_pid",android.os.Process.myPid());
   r.put("dialer_role",getSystemService(RoleManager.class).isRoleHeld(RoleManager.ROLE_DIALER));
   JSONArray calls=new JSONArray();
   for(java.util.Map.Entry<Call,Integer> e:CallService.calls.entrySet()){
    JSONObject c=new JSONObject();c.put("id",e.getValue());c.put("state",e.getKey().getDetails().getState());calls.put(c);
   }
   r.put("source","android.telecom.InCallService");r.put("call_snapshot_scope","current_process_callbacks_only");r.put("calls",calls);return r.toString();
  }catch(JSONException e){throw new IllegalStateException(e);}
 }
 @Override public IBinder onBind(Intent i){return endpoint;}
}
