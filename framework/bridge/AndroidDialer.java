package org.omarchy.bridge;
import android.app.role.RoleManager;
import android.content.*;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.*;
import android.telecom.*;
import android.telephony.TelephonyManager;
import java.io.IOException;
import java.util.*;
import org.json.*;
/** Ordinary outgoing calls. Invoked only on the main looper after transport authentication. */
final class AndroidDialer {
 static final String ATTEMPT="org.omarchy.bridge.OUTGOING_ATTEMPT";
 private static DialAttempt attempt;
 private static DialPolicy policy;
 private static String enrollment="",generation="";
 private static long revision=1;
 private static List<Object> observed=Collections.emptyList();
 private static DialAttempt attempt(Context c)throws IOException{
  if(attempt==null)attempt=new DialAttempt(new DialJournal(c));return attempt;
 }
 private static void setup(Context c,String owner)throws IOException{
  DialAttempt pending=attempt(c);
  if(policy==null||!enrollment.equals(owner)){
   enrollment=owner;generation=UUID.randomUUID().toString().replace("-","");
   policy=new DialPolicy(generation,1024,64,pending,new java.util.function.LongSupplier(){public long getAsLong(){return SystemClock.elapsedRealtime();}});
   observed=Collections.emptyList();revision++;
  }
 }
 private static final class Phone implements DialPolicy.Phone {
  final Context context;final String owner;final AndroidControls.Admission admission;
  Phone(Context c,String owner,AndroidControls.Admission admission){context=c;this.owner=owner;this.admission=admission;}
  public String unavailable(){
   SharedPreferences p=context.getSharedPreferences("gateway",0);
   if(owner.isEmpty()||!p.getBoolean("enabled",false)||!p.getBoolean("controls",false)||!owner.equals(p.getString("controls_for",""))||!p.getBoolean("outgoing",false)||!owner.equals(p.getString("outgoing_for","")))return "disabled";
   if(!context.getSystemService(RoleManager.class).isRoleHeld(RoleManager.ROLE_DIALER))return "role";
   if(context.checkSelfPermission("android.permission.CALL_PHONE")!=PackageManager.PERMISSION_GRANTED||context.checkSelfPermission("android.permission.READ_PHONE_STATE")!=PackageManager.PERMISSION_GRANTED)return "permission";
   try{
    for(Call call:CallService.calls.keySet())if(call.getDetails().getState()!=Call.STATE_DISCONNECTED)return "busy";
    if(context.getSystemService(TelecomManager.class).isInCall())return "busy";
    return account()==null?"account_unavailable":null;
   }catch(RuntimeException e){return "unavailable";}
  }
  public Object account(){
   TelecomManager telecom=context.getSystemService(TelecomManager.class);
   PhoneAccountHandle handle=telecom.getDefaultOutgoingPhoneAccount("tel");
   PhoneAccount account=handle==null?null:telecom.getPhoneAccount(handle);
   return account!=null&&account.isEnabled()&&account.supportsUriScheme("tel")?handle:null;
  }
  public long revision(){
   SharedPreferences p=context.getSharedPreferences("gateway",0);
   List<Object> state=Arrays.<Object>asList(unavailable(),account(),p.getString("control_epoch",""),p.getString("outgoing_epoch",""),CallService.generation,CallService.revision());
   if(!state.equals(observed)){observed=state;AndroidDialer.revision++;}
   return AndroidDialer.revision;
  }
  public String classify(String number){
   try{return context.getSystemService(TelephonyManager.class).isEmergencyNumber(number)?"emergency":null;}
   catch(RuntimeException e){return "unavailable";}
  }
  public boolean commitAllowed(){return admission!=null&&admission.commit();}
  public void dispatch(String number,Object account,String marker){
   Bundle metadata=new Bundle();metadata.putString(ATTEMPT,marker);
   Bundle extras=new Bundle();extras.putBundle(TelecomManager.EXTRA_OUTGOING_CALL_EXTRAS,metadata);
   extras.putParcelable(TelecomManager.EXTRA_PHONE_ACCOUNT_HANDLE,(PhoneAccountHandle)account);
   extras.putInt(TelecomManager.EXTRA_START_CALL_WITH_VIDEO_STATE,VideoProfile.STATE_AUDIO_ONLY);
   context.getSystemService(TelecomManager.class).placeCall(Uri.fromParts("tel",number,null),extras);
  }
 }
 static String snapshot(Context c,String owner,String id)throws Exception{
  setup(c,owner);Phone phone=new Phone(c,owner,null);String reason=phone.unavailable(),offer="";
  if(policy.pending())reason="pending";
  if(reason==null){try{offer=policy.offer(phone);reason=offer.isEmpty()?"offer_capacity":"ready";}catch(RuntimeException e){reason="unavailable";}}
  JSONObject value=new JSONObject();value.put("v",1);value.put("id",id);value.put("generation",generation);
  value.put("can_dial",!offer.isEmpty());value.put("offer",offer);value.put("pending",policy.pending());value.put("reason",reason);
  return value.toString();
 }
 static String command(Context c,String owner,String id,String expectedGeneration,String offer,String number,AndroidControls.Admission admission)throws Exception{
  setup(c,owner);String result=policy.execute(new Phone(c,owner,admission),id,expectedGeneration,offer,number);
  String[] parts=result.split(":",2);JSONObject value=new JSONObject();value.put("v",1);value.put("id",id);value.put("generation",expectedGeneration);
  value.put("outcome",parts[0]);value.put("reason",parts[1]);return value.toString();
 }
 // The caller must fence callbacks from a torn-down/replaced InCallService session.
 static void observedTerminal(Context c,Call call,boolean validSession){
  if(!validSession||call.getDetails().getState()!=Call.STATE_DISCONNECTED)return;
  Bundle extras=call.getDetails().getIntentExtras();if(extras==null)return;
  Object marker=extras.get(ATTEMPT);if(!(marker instanceof String)||!((String)marker).matches("[0-9a-f]{32}"))return;
  try{attempt(c).terminal((String)marker);}catch(IOException e){android.util.Log.e("OmarchyBridge","Outgoing journal unavailable");}
 }
}
