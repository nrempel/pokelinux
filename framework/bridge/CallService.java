package org.omarchy.bridge;
import android.app.*;
import android.content.*;
import android.net.Uri;
import android.os.*;
import android.telecom.*;
import java.util.*;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.ConcurrentHashMap;
/** Android owns calls. Registry loss invalidates observations before Telecom teardown. */
public final class CallService extends InCallService {
 static final ConcurrentHashMap<Call,Integer> calls=new ConcurrentHashMap<>();
 static volatile boolean connected;
 private static final CallRevision clock=new CallRevision();
 static final CallChanges changes=new CallChanges();
 private static final RingingRegistry ringing=new RingingRegistry();
 // Callbacks and gateway sampling run on the main looper. Sample every observed
 // transition so that leaving and re-entering ringing creates a new episode,
 // even when both transitions occur between two network watch requests.
 static RingingRegistry.Snapshot ringingSnapshot(){
  Set<Integer> ids=new HashSet<>();
  if(connected)for(Map.Entry<Call,Integer> entry:calls.entrySet()){
   if(entry.getValue()<1||calls.size()>64)return ringing.observe(generation,"unavailable",Collections.<Integer>emptySet());
   if(entry.getKey().getDetails().getState()==Call.STATE_RINGING)ids.add(entry.getValue());
  }
  return ringing.observe(generation,connected?"observed":"detached",ids);
 }
 static void changed(){clock.changed();ringingSnapshot();changes.changed();}
 static long revision(){
  Map<Integer,List<Object>> current=new HashMap<>();
  for(Map.Entry<Call,Integer> entry:calls.entrySet()){
   Call call=entry.getKey();current.put(entry.getValue(),Arrays.<Object>asList(call.getDetails(),call.getDetails().getState(),call.getParent(),new ArrayList<>(call.getChildren())));
  }
  return clock.observe(current);
 }
 static volatile String generation=UUID.randomUUID().toString().replace("-", "");
 static final AtomicInteger sequence=new AtomicInteger();
 private String observationSession="";
 private final Map<Call,String> callSessions=new HashMap<>();
 private final Map<Call,Boolean> callbackRinging=new HashMap<>();
 private int foregroundId;
 private final Set<Integer> alerted=new HashSet<>();
 static volatile boolean notificationFailed;
 private final Map<Call,Call.Callback> callbacks=new HashMap<>();
 static void invalidate(){connected=false;calls.clear();generation=UUID.randomUUID().toString().replace("-", "");changed();}
 @Override public IBinder onBind(Intent intent){observationSession=UUID.randomUUID().toString();connected=true;changed();return super.onBind(intent);}
 @Override public boolean onUnbind(Intent intent){observationSession="";callbackRinging.clear();clearNotifications();invalidate();return super.onUnbind(intent);}
 @Override public void onCallAdded(final Call call){
  super.onCallAdded(call);calls.put(call,sequence.incrementAndGet());
  final String session=observationSession;callSessions.put(call,session);
  Call.Callback callback=new Call.Callback(){
   @Override public void onStateChanged(Call changed,int state){observeCallState(changed,session,state);}
   @Override public void onDetailsChanged(Call changed,Call.Details details){observeCallChange(changed,session);}
   @Override public void onChildrenChanged(Call changed,List<Call> children){observeCallChange(changed,session);}
   @Override public void onParentChanged(Call changed,Call parent){observeCallChange(changed,session);}
  };
  callbacks.put(call,callback);call.registerCallback(callback,new Handler(getMainLooper()));callbackRinging.put(call,call.getDetails().getState()==Call.STATE_RINGING);observeCallChange(call,session);
 }
 @Override public void onCallRemoved(Call call){
  String session=callSessions.remove(call);callbackRinging.remove(call);observeOutgoing(call,session);
  Integer id=calls.remove(call);if(id!=null)alerted.remove(id);changed();Call.Callback callback=callbacks.remove(call);if(callback!=null)call.unregisterCallback(callback);
  if(id!=null){
   if(foregroundId==1000+id){stopForeground(STOP_FOREGROUND_REMOVE);foregroundId=0;}
   getSystemService(NotificationManager.class).cancel(1000+id);
   if(!calls.isEmpty())notifyCall(calls.keySet().iterator().next());
  }super.onCallRemoved(call);
 }
 private boolean currentCall(Call call,String session){
  return connected&&session!=null&&!session.isEmpty()&&session.equals(observationSession)
    &&session.equals(callSessions.get(call))&&calls.containsKey(call);
 }
 private void observeCallState(Call call,String session,int state){
  if(!currentCall(call,session))return;
  boolean nowRinging=state==Call.STATE_RINGING;
  Boolean wasRinging=callbackRinging.put(call,nowRinging);
  if(Boolean.TRUE.equals(wasRinging)&&!nowRinging)ringing.leftRinging(generation,calls.get(call));
  observeCallChange(call,session);
 }
 private void observeCallChange(Call call,String session){
  if(!currentCall(call,session))return;
  changed();observeOutgoing(call,session);notifyCall(call);
 }
 private void observeOutgoing(Call call,String session){AndroidDialer.observedTerminal(this,call,connected&&session!=null&&!session.isEmpty()&&session.equals(observationSession));}
 static String label(Call call){
  Call.Details details=call.getDetails();Uri handle=details.getHandle();
  return details.getHandlePresentation()==TelecomManager.PRESENTATION_ALLOWED&&handle!=null?handle.getSchemeSpecificPart():"Unknown caller";
 }
 private PendingIntent action(int id,String action){
  Intent intent=new Intent(this,CallActionReceiver.class).setData(Uri.parse("omarchy-call://"+generation+"/"+id+"/"+action));
  intent.putExtra("generation",generation).putExtra("call_id",id).putExtra("action",action);
  return PendingIntent.getBroadcast(this,0,intent,PendingIntent.FLAG_IMMUTABLE);
 }
 private void notifyCall(Call call){
  Integer id=calls.get(call);if(id==null||!connected)return;
  NotificationManager manager=getSystemService(NotificationManager.class);int state=call.getDetails().getState();
  if(state==Call.STATE_DISCONNECTED){if(foregroundId==1000+id){stopForeground(STOP_FOREGROUND_REMOVE);foregroundId=0;}manager.cancel(1000+id);return;}
  boolean ringing=state==Call.STATE_RINGING;
  NotificationChannel incoming=new NotificationChannel("incoming-calls","Incoming calls",NotificationManager.IMPORTANCE_HIGH);
  incoming.setSound(android.provider.Settings.System.DEFAULT_RINGTONE_URI,new android.media.AudioAttributes.Builder().setUsage(android.media.AudioAttributes.USAGE_NOTIFICATION_RINGTONE).build());
  manager.createNotificationChannel(incoming);manager.createNotificationChannel(new NotificationChannel("ongoing-calls","Ongoing calls",NotificationManager.IMPORTANCE_LOW));
  Intent open=new Intent(this,DialActivity.class).setData(Uri.parse("omarchy-call://"+generation+"/"+id)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK|Intent.FLAG_ACTIVITY_SINGLE_TOP);
  PendingIntent content=PendingIntent.getActivity(this,0,open,PendingIntent.FLAG_IMMUTABLE);
  Person person=new Person.Builder().setName(label(call)).setImportant(true).build();
  Notification.Builder notice=new Notification.Builder(this,ringing?"incoming-calls":"ongoing-calls")
   .setSmallIcon(android.R.drawable.stat_sys_phone_call).setContentTitle(ringing?"Incoming call":"Call in progress")
   .setContentIntent(content).setOngoing(true).setOnlyAlertOnce(true).setCategory(Notification.CATEGORY_CALL)
   .setStyle(ringing?Notification.CallStyle.forIncomingCall(person,action(id,"reject"),action(id,"answer")):Notification.CallStyle.forOngoingCall(person,action(id,"disconnect")));
  if(ringing&&alerted.add(id)&&(Build.VERSION.SDK_INT<34||manager.canUseFullScreenIntent()))notice.setFullScreenIntent(content,true);
  try{startForeground(1000+id,notice.build(),android.content.pm.ServiceInfo.FOREGROUND_SERVICE_TYPE_PHONE_CALL);foregroundId=1000+id;notificationFailed=false;}catch(RuntimeException e){notificationFailed=true;android.util.Log.e("OmarchyBridge","Call notification unavailable: "+e.getClass().getSimpleName());}
 }
 static void localAction(String expectedGeneration,int id,String action){
  if(!connected||!generation.equals(expectedGeneration))return;
  for(Map.Entry<Call,Integer> entry:calls.entrySet())if(entry.getValue()==id){
   Call call=entry.getKey();int state=call.getDetails().getState();
   if(action.equals("answer")&&state==Call.STATE_RINGING){changed();call.answer(VideoProfile.STATE_AUDIO_ONLY);}
   else if(action.equals("reject")&&state==Call.STATE_RINGING){changed();call.reject(Call.REJECT_REASON_DECLINED);}
   else if(action.equals("disconnect")&&(state==Call.STATE_ACTIVE||state==Call.STATE_HOLDING||state==Call.STATE_DIALING||state==Call.STATE_CONNECTING)){changed();call.disconnect();}
   return;
  }
 }
 private void clearNotifications(){stopForeground(STOP_FOREGROUND_REMOVE);foregroundId=0;NotificationManager manager=getSystemService(NotificationManager.class);for(Integer id:calls.values())manager.cancel(1000+id);}
 @Override public void onDestroy(){observationSession="";callSessions.clear();callbackRinging.clear();clearNotifications();invalidate();for(Map.Entry<Call,Call.Callback> entry:callbacks.entrySet())entry.getKey().unregisterCallback(entry.getValue());callbacks.clear();super.onDestroy();}
}
