package org.omarchy.bridge;
import android.app.role.RoleManager;
import android.content.Context;
import android.content.SharedPreferences;
import android.os.SystemClock;
import android.telecom.*;
import java.util.*;
import org.json.*;
/** Main-looper adapter: policy decisions and Telecom invocation share one short commit. */
final class AndroidControls {
 interface Admission {boolean commit();}
 private static String scope="",generation="";
 private static ControlPolicy policy;
 private static boolean role(Context c){return c.getSystemService(RoleManager.class).isRoleHeld(RoleManager.ROLE_DIALER);}
 private static boolean enabled(Context c,String enrollment){
  SharedPreferences prefs=c.getSharedPreferences("gateway",0);
  return !enrollment.isEmpty()&&prefs.getBoolean("enabled",false)&&prefs.getBoolean("controls",false)&&enrollment.equals(prefs.getString("controls_for",""));
 }
 private static void refresh(Context c,String enrollment){
  SharedPreferences p=c.getSharedPreferences("gateway",0);
  String current=CallService.generation+":"+p.getString("control_epoch","")+":"+enrollment+":"+p.getString("controls_for","")+":"+p.getBoolean("controls",false)+":"+p.getBoolean("enabled",false)+":"+role(c);
  if(!current.equals(scope)){scope=current;generation=UUID.randomUUID().toString().replace("-", "");policy=new ControlPolicy(generation,1024);}
 }
 private static Call find(int id){for(Map.Entry<Call,Integer> entry:CallService.calls.entrySet())if(entry.getValue()==id)return entry.getKey();return null;}
 private static boolean simple(Call call){
  if(call==null||CallService.calls.size()!=1||call.getParent()!=null||!call.getChildren().isEmpty())return false;
  return !call.getDetails().hasProperty(Call.Details.PROPERTY_CONFERENCE)&&!call.getDetails().hasProperty(Call.Details.PROPERTY_IS_EXTERNAL_CALL);
 }
 static String snapshot(Context c,String enrollment,String requestId)throws JSONException{
  refresh(c,enrollment);boolean held=role(c),available=held&&CallService.connected&&!CallService.calls.isEmpty();
  boolean authorized=enabled(c,enrollment)&&held;
  JSONObject value=new JSONObject();value.put("v",1);value.put("id",requestId);value.put("generation",generation);value.put("revision",CallService.revision());
  value.put("controls_allowed",authorized);value.put("availability",available?"observed":"unavailable");value.put("source","android.telecom.InCallService");
  JSONArray calls=new JSONArray();
  if(available){
   if(CallService.calls.size()>64)throw new IllegalStateException("Call limit");
   for(Map.Entry<Call,Integer> entry:CallService.calls.entrySet()){
    Call call=entry.getKey();int state=call.getDetails().getState();JSONObject item=new JSONObject();item.put("id",entry.getValue());item.put("state",state);JSONArray actions=new JSONArray();
    if(authorized&&simple(call)){
     if(state==Call.STATE_RINGING){actions.put("answer");actions.put("reject");}
     else if(state==Call.STATE_ACTIVE||state==Call.STATE_HOLDING||state==Call.STATE_DIALING||state==Call.STATE_CONNECTING)actions.put("disconnect");
    }
    item.put("actions",actions);calls.put(item);
   }
  }
  value.put("calls",calls);return value.toString();
 }
 static String command(final Context c,final String enrollment,String id,String expectedGeneration,final int callId,long revision,int state,String action,long deadline,final Admission admission)throws JSONException{
  refresh(c,enrollment);
  String result=policy.execute(new ControlPolicy.Phone(){
   public boolean enabled(){return AndroidControls.enabled(c,enrollment)&&role(c);}
   public boolean available(){return role(c)&&CallService.connected&&simple(find(callId));}
   public int state(int id){Call call=find(id);return call==null?-1:call.getDetails().getState();}
   public long revision(int id){return CallService.revision();}
   public void consumeRevision(int id){CallService.changed();}
   public boolean commitAllowed(){return admission.commit();}
   public void dispatch(int id,String action){
    Call call=find(id);if(call==null)throw new IllegalStateException("Call removed");
    if(action.equals("answer"))call.answer(VideoProfile.STATE_AUDIO_ONLY);
    else if(action.equals("reject"))call.reject(Call.REJECT_REASON_DECLINED);
    else if(action.equals("disconnect"))call.disconnect();
    else throw new IllegalArgumentException("Action");
   }
  },id,expectedGeneration,callId,revision,state,action,deadline,SystemClock.elapsedRealtime());
  String[] parts=result.split(":",2);JSONObject response=new JSONObject();response.put("v",1);response.put("id",id);response.put("generation",expectedGeneration);response.put("outcome",parts[0]);response.put("reason",parts[1]);return response.toString();
 }
}
