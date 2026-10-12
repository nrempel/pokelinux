package org.omarchy.bridge;

import android.app.*;
import android.app.role.RoleManager;
import android.content.Intent;
import android.os.*;
import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.security.*;
import java.security.cert.X509Certificate;
import java.util.*;
import java.util.concurrent.*;
import javax.net.ssl.*;
import org.json.*;

/** Loopback-only authenticated bridge. Commands retain separate admission checks. */
public final class GatewayService extends Service {
 private static final String GENERATION=UUID.randomUUID().toString().replace("-", "");
 private final Set<Connection> sockets=ConcurrentHashMap.newKeySet();
 private final ThreadPoolExecutor workers=new ThreadPoolExecutor(4,4,0,TimeUnit.SECONDS,new ArrayBlockingQueue<Runnable>(4));
 private final WatchWorkers watches=new WatchWorkers();
 private final ScheduledThreadPoolExecutor deadlines=new ScheduledThreadPoolExecutor(1);
 static volatile boolean running,listening;
 static void disable(android.content.Context c){c.getSharedPreferences("gateway",0).edit().putBoolean("enabled",false).putBoolean("controls",false).putBoolean("outgoing",false).putString("control_epoch",UUID.randomUUID().toString()).commit();c.stopService(new Intent(c,GatewayService.class));}
 private volatile boolean stopped;
 private volatile ServerSocket listener;
 private byte[] clientPin;
 private String enrollmentId="";
 private long enrollmentExpires=Long.MAX_VALUE;
 @Override public void onCreate(){
  super.onCreate();
  if(!getSharedPreferences("gateway",0).getBoolean("enabled",false)){stopSelf();return;}
  running=true;
  deadlines.setRemoveOnCancelPolicy(true);
  NotificationManager manager=getSystemService(NotificationManager.class);
  manager.createNotificationChannel(new NotificationChannel("bridge","Android bridge",NotificationManager.IMPORTANCE_LOW));
  startForeground(1,new Notification.Builder(this,"bridge").setSmallIcon(android.R.drawable.stat_sys_phone_call).setContentTitle("Android bridge active").setContentText("Phone status for the paired desktop").setContentIntent(PendingIntent.getActivity(this,1,new Intent(this,DialActivity.class),PendingIntent.FLAG_IMMUTABLE)).addAction(new Notification.Action.Builder(null,"Stop bridge",PendingIntent.getBroadcast(this,2,new Intent(this,StopReceiver.class),PendingIntent.FLAG_IMMUTABLE)).build()).build());
  new Thread(new Runnable(){public void run(){serve();}},"bridge-accept").start();
 }
 @Override public int onStartCommand(Intent intent,int flags,int id){if(!getSharedPreferences("gateway",0).getBoolean("enabled",false)){stopSelf();return START_NOT_STICKY;}return START_STICKY;}
 @Override public IBinder onBind(Intent intent){return null;}
 // One owner closes each socket. Only authenticated, parsed watches may extend
 // their deadline; command deadlines are fixed from accept, including queue time.
 private final class Connection {
  final BridgeTransport transport;
  volatile SSLSocket socket;
  final long accepted=SystemClock.elapsedRealtime();
  final long requestDeadline=accepted+4500;
  long deadline=accepted+5000;
  long expires=enrollmentExpires;
  CredentialExpiry credential;
  ScheduledFuture<?> timer;
  boolean watching;
  volatile boolean cancelled;
  Connection(Socket raw){transport=new BridgeTransport(raw);}
  synchronized void arm(){
   timer=deadlines.schedule(new Runnable(){public void run(){cancel(true);}},Math.max(0,deadline-SystemClock.elapsedRealtime()),TimeUnit.MILLISECONDS);
  }
  synchronized void authenticated(long certificateExpires)throws IOException{
   check();expires=Math.min(enrollmentExpires,certificateExpires);
   credential=new CredentialExpiry(SystemClock.elapsedRealtime(),System.currentTimeMillis(),expires);
   deadline=credential.cap(deadline);timer.cancel(false);arm();
  }
  synchronized void watch(int waitMs)throws IOException{
   if(stopped||cancelled||transport.isClosed()||SystemClock.elapsedRealtime()>=requestDeadline)throw new IOException("Watch admission expired");
   check();if(credential==null)throw new IOException("Authentication required");
   deadline=credential.cap(accepted+(waitMs<=1000?5000:30000));
   watching=true;
   timer.cancel(false);arm();
  }
  synchronized void check()throws IOException{
   if(stopped||cancelled||transport.isClosed()||Thread.currentThread().isInterrupted()
     ||SystemClock.elapsedRealtime()>=deadline||System.currentTimeMillis()>=expires
     ||(credential!=null&&!credential.valid(SystemClock.elapsedRealtime(),System.currentTimeMillis())))throw new IOException("Observation expired");
  }
  synchronized long sampleBudget()throws IOException{
   check();long remaining=deadline-SystemClock.elapsedRealtime();
   if(remaining<=0)throw new IOException("Observation expired");
   return Math.min(2000,remaining);
  }
  void cancel(boolean onlyIfExpired){
   boolean notify;
   synchronized(this){
    if(cancelled||(onlyIfExpired&&SystemClock.elapsedRealtime()<deadline))return;
    cancelled=true;if(timer!=null)timer.cancel(false);notify=watching;watching=false;
   }
   // No TLS operation or network cleanup under the monitor used by main-loop
   // guards. The timer/main thread only aborts the independently owned raw fd.
   transport.abort();sockets.remove(this);if(notify)CallService.changes.changed();
  }
  void finish(){cancel(false);transport.finish();}
 }
 private void serve(){
  try{
   GatewayIdentity identity=GatewayIdentity.open(this);
   SSLContext context=identity.context;clientPin=identity.pin;enrollmentId=identity.enrollment;enrollmentExpires=identity.expires;
   final SSLSocketFactory factory=context.getSocketFactory();
   ServerSocket server=new ServerSocket();
   listener=server;
   if(stopped){server.close();return;}
   server.setReuseAddress(true);
   server.bind(new InetSocketAddress(InetAddress.getByName("127.0.0.1"),37179),8);listening=true;
   while(!stopped){
    final Socket raw=server.accept();raw.setSoTimeout(5000);
    final Connection connection=new Connection(raw);sockets.add(connection);
    if(stopped){connection.cancel(false);break;}
    try{
     connection.arm();
     workers.execute(new Runnable(){public void run(){boolean transferred=false;try{transferred=exchange(connection,factory);}finally{if(!transferred)connection.finish();}}});
    }catch(RejectedExecutionException e){connection.finish();}
   }
  }catch(Exception e){if(!stopped){android.util.Log.e("OmarchyBridge","Gateway stopped: "+e.getClass().getSimpleName());new Handler(getMainLooper()).post(new Runnable(){public void run(){stopSelf();}});}}
  finally{listening=false;ServerSocket server=listener;if(server!=null)try{server.close();}catch(IOException ignored){}for(Connection connection:sockets)connection.cancel(false);}
 }
 private boolean exchange(final Connection connection,SSLSocketFactory factory){
  final long requestDeadline=connection.requestDeadline;
  try{
   connection.check();
   final SSLSocket socket=(SSLSocket)factory.createSocket(connection.transport.raw,"127.0.0.1",connection.transport.raw.getPort(),true);
   connection.transport.attach(socket);connection.socket=socket;
   socket.setUseClientMode(false);socket.setEnabledProtocols(new String[]{"TLSv1.3"});socket.setNeedClientAuth(true);socket.setSoTimeout(5000);
   connection.check();socket.startHandshake();
   X509Certificate certificate=(X509Certificate)socket.getSession().getPeerCertificates()[0];
   certificate.checkValidity();
   if(System.currentTimeMillis()>=enrollmentExpires)throw new GeneralSecurityException("Enrollment expired");
   if(!MessageDigest.isEqual(clientPin,MessageDigest.getInstance("SHA-256").digest(certificate.getEncoded())))throw new GeneralSecurityException("Peer pin mismatch");
   connection.authenticated(certificate.getNotAfter().getTime());
   DataInputStream input=new DataInputStream(socket.getInputStream());int length=input.readInt();
   if(length<1||length>4096)throw new IOException("Frame size");
   byte[] body=new byte[length];input.readFully(body);
   final WireRequest request=WireRequest.parse(body);
   connection.check();
   if("watch".equals(request.op)){
    connection.watch(request.waitMs);
    watches.execute(request.waitMs,new Runnable(){public void run(){try{watch(connection,request);}finally{connection.finish();}}});
    return true;
   }
   final Admission gate=new Admission(new java.util.function.LongSupplier(){public long getAsLong(){return SystemClock.elapsedRealtime();}},new java.util.function.LongSupplier(){public long getAsLong(){return System.currentTimeMillis();}},new java.util.function.BooleanSupplier(){public boolean getAsBoolean(){return stopped||connection.cancelled||socket.isClosed();}},Math.min(requestDeadline,connection.deadline),connection.expires);
   final Callable<String> task;
   if("status".equals(request.op)){
    final String id=request.id;final int phoneState=PhoneObservation.read(this);
    task=new Callable<String>(){public String call()throws Exception{return snapshot(id,phoneState);}};
   }else if("controls".equals(request.op)){
    final String id=request.id;
    task=new Callable<String>(){public String call()throws Exception{return AndroidControls.snapshot(GatewayService.this,enrollmentId,id);}};
   }else if("dial_status".equals(request.op)){
    final String id=request.id;
    task=new Callable<String>(){public String call()throws Exception{return AndroidDialer.snapshot(GatewayService.this,enrollmentId,id);}};
   }else if("dial".equals(request.op)){
    final String id=request.id,generation=request.generation,offer=request.offer,number=request.number;
    task=new Callable<String>(){public String call()throws Exception{return AndroidDialer.command(GatewayService.this,enrollmentId,id,generation,offer,number,new AndroidControls.Admission(){public boolean commit(){return gate.commit();}});}};
   }else if("control".equals(request.op)){
    final String id=request.id,generation=request.generation,action=request.action;
    final int callId=request.callId,state=request.state;final long revision=request.revision;
    task=new Callable<String>(){public String call()throws Exception{
     return AndroidControls.command(GatewayService.this,enrollmentId,id,generation,callId,revision,state,action,requestDeadline,new AndroidControls.Admission(){public boolean commit(){
      return gate.commit();
     }});
    }};
   }else throw new IOException("Request schema");
   if(System.currentTimeMillis()>=enrollmentExpires)throw new GeneralSecurityException("Enrollment expired");
   FutureTask<String> observation=new FutureTask<>(task);
   Handler handler=new Handler(getMainLooper());handler.post(observation);
   String response;
   try{response=observation.get(2,TimeUnit.SECONDS);}finally{gate.cancel();handler.removeCallbacks(observation);observation.cancel(false);}
   byte[] bytes=response.getBytes(StandardCharsets.UTF_8);
   if(bytes.length>65536)throw new IOException("Response size");
   DataOutputStream output=new DataOutputStream(socket.getOutputStream());output.writeInt(bytes.length);output.write(bytes);output.flush();
  }catch(Exception e){/* Close on any protocol/auth failure; no peer data in logs. */}
  return false;
 }
 private void watch(final Connection connection,final WireRequest request){
  try{
   final Handler handler=new Handler(getMainLooper());
   RingingRegistry.Snapshot current=CallWatch.read(CallService.changes,new Callable<RingingRegistry.Snapshot>(){public RingingRegistry.Snapshot call()throws Exception{
    FutureTask<RingingRegistry.Snapshot> sample=new FutureTask<>(new Callable<RingingRegistry.Snapshot>(){public RingingRegistry.Snapshot call()throws Exception{
     connection.check();
     if(!getSystemService(RoleManager.class).isRoleHeld(RoleManager.ROLE_DIALER))throw new SecurityException("Dialer role unavailable");
     return CallService.ringingSnapshot();
    }});
    if(!handler.post(sample))throw new IOException("Main looper unavailable");
    try{return sample.get(connection.sampleBudget(),TimeUnit.MILLISECONDS);}
    finally{handler.removeCallbacks(sample);sample.cancel(false);}
   }},new CallWatch.Guard(){public void check()throws IOException{connection.check();}},new java.util.function.LongSupplier(){public long getAsLong(){return SystemClock.elapsedRealtime();}},request.afterGeneration,request.afterRevision,request.waitMs,connection.deadline);
   JSONObject value=new JSONObject();value.put("v",1);value.put("id",request.id);
   value.put("generation",current.generation);value.put("revision",current.revision);
   value.put("source","android.telecom.InCallService");value.put("source_state",current.sourceState);
   JSONArray calls=new JSONArray();
   for(Map.Entry<Integer,String> entry:current.ringing.entrySet()){
    JSONObject call=new JSONObject();call.put("id",entry.getKey());call.put("episode",entry.getValue());calls.put(call);
   }
   value.put("ringing",calls);byte[] bytes=value.toString().getBytes(StandardCharsets.UTF_8);
   if(bytes.length>65536)throw new IOException("Response size");
   connection.check();
   DataOutputStream output=new DataOutputStream(connection.socket.getOutputStream());
   output.writeInt(bytes.length);output.write(bytes);output.flush();
  }catch(Exception e){/* A failed observation sends no cached state or peer data to logs. */}
 }
 private String snapshot(String id,int phoneState)throws JSONException{
  boolean role=getSystemService(RoleManager.class).isRoleHeld(RoleManager.ROLE_DIALER);
  boolean available=role&&CallService.connected&&!CallService.calls.isEmpty();
  JSONObject value=new JSONObject();value.put("v",1);value.put("id",id);value.put("generation",GENERATION);
  value.put("observed_ms",SystemClock.elapsedRealtime());value.put("dialer_role",role);
  value.put("availability",available?"observed":"unavailable");value.put("source","android.telecom.InCallService");
  JSONArray calls=new JSONArray();
  if(available){
   if(CallService.calls.size()>64)throw new IllegalStateException("Call limit");
   for(Map.Entry<android.telecom.Call,Integer> entry:CallService.calls.entrySet()){
    JSONObject call=new JSONObject();call.put("id",entry.getValue());call.put("state",entry.getKey().getDetails().getState());calls.put(call);
   }
  }
  value.put("calls",calls);
  if(checkSelfPermission("android.permission.READ_PHONE_STATE")!=android.content.pm.PackageManager.PERMISSION_GRANTED)phoneState=-1;
  JSONObject phone=new JSONObject();phone.put("availability",phoneState<0?"unavailable":"observed");phone.put("state",phoneState<0?JSONObject.NULL:Integer.valueOf(phoneState));phone.put("source","android.telephony.TelephonyCallback");phone.put("scope","default_subscription");value.put("phone_state",phone);
  return value.toString();
 }
 @Override public void onDestroy(){
  running=false;listening=false;stopped=true;ServerSocket server=listener;if(server!=null)try{server.close();}catch(IOException ignored){}
  for(Connection connection:sockets)connection.cancel(false);CallService.changes.changed();workers.shutdownNow();watches.shutdownNow();deadlines.shutdownNow();super.onDestroy();
 }
}
