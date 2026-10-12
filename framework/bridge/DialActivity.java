package org.omarchy.bridge;
import android.Manifest;
import android.app.*;
import android.app.role.RoleManager;
import android.content.*;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.net.Uri;
import android.os.*;
import android.telecom.*;
import android.view.*;
import android.widget.*;
import java.io.*;
import java.security.MessageDigest;
import java.util.*;
import java.util.concurrent.*;

/** Local phone UI and explicit bridge enrollment. Launch never grants authorization. */
public final class DialActivity extends Activity {
 private LinearLayout content;
 private TextView state;
 private EditText number;
 private boolean bridgePage,busy;
 private byte[] exported;
 private long shownRevision=-1;
 private String draftNumber="";
 private final Handler handler=new Handler(Looper.getMainLooper());
 private final ExecutorService io=Executors.newSingleThreadExecutor();
 private final Runnable refresh=new Runnable(){public void run(){
  if(state!=null&&bridgePage)state.setText(GatewayService.listening?"Bridge active":enabled()?"Bridge unavailable":"Bridge off");
  if(!bridgePage&&shownRevision!=CallService.revision()){render();}
  handler.postDelayed(this,1000);
 }};
 private int dp(int value){return (int)(value*getResources().getDisplayMetrics().density+.5f);}
 private boolean enabled(){return getSharedPreferences("gateway",0).getBoolean("enabled",false);}
 private boolean role(){return getSystemService(RoleManager.class).isRoleHeld(RoleManager.ROLE_DIALER);}
 @Override public void onCreate(Bundle saved){
  setTheme(android.R.style.Theme_Material_NoActionBar);super.onCreate(saved);
  setShowWhenLocked(true);
  if(saved!=null)bridgePage=saved.getBoolean("bridgePage");
  if(enabled())try{startForegroundService(new Intent(this,GatewayService.class));}catch(RuntimeException ignored){}
  render();handler.post(refresh);
 }
 @Override protected void onSaveInstanceState(Bundle out){super.onSaveInstanceState(out);out.putBoolean("bridgePage",bridgePage);}
 @Override protected void onNewIntent(Intent intent){super.onNewIntent(intent);boolean repeated=Objects.equals(getIntent().getData(),intent.getData());setIntent(intent);if(!repeated){bridgePage=false;draftNumber="";}render();}
 @Override protected void onResume(){super.onResume();if(content!=null)render();}
 @Override protected void onDestroy(){handler.removeCallbacks(refresh);io.shutdownNow();super.onDestroy();}
 private TextView text(String value,int size){TextView v=new TextView(this);v.setText(value);v.setTextSize(size);v.setTextColor(Color.rgb(236,239,241));v.setPadding(0,dp(8),0,dp(8));content.addView(v);return v;}
 private Button button(String label,final Runnable action){
  Button b=new Button(this);b.setText(label);b.setAllCaps(false);b.setMinHeight(dp(48));b.setOnClickListener(new View.OnClickListener(){public void onClick(View v){if(!busy)try{action.run();}catch(RuntimeException e){error("This action is unavailable. Check permissions and try again.");}}});content.addView(b,new LinearLayout.LayoutParams(-1,dp(52)));return b;
 }
 private void error(String message){if(!isDestroyed())new AlertDialog.Builder(this).setTitle("Action unavailable").setMessage(message).setPositiveButton("OK",null).show();}
 private void render(){
  if(number!=null){draftNumber=number.getText().toString();number=null;}
  shownRevision=CallService.revision();
  LinearLayout root=new LinearLayout(this);root.setOrientation(LinearLayout.VERTICAL);root.setBackgroundColor(Color.rgb(16,19,22));root.setPadding(dp(18),dp(24),dp(18),dp(18));
  root.setOnApplyWindowInsetsListener(new View.OnApplyWindowInsetsListener(){public WindowInsets onApplyWindowInsets(View v,WindowInsets insets){android.graphics.Insets bars=insets.getInsets(WindowInsets.Type.systemBars());v.setPadding(dp(18),bars.top+dp(12),dp(18),bars.bottom+dp(12));return insets;}});
  TextView title=new TextView(this);title.setText("Omarchy Phone");title.setTextSize(25);title.setTextColor(Color.WHITE);root.addView(title);
  LinearLayout nav=new LinearLayout(this);
  for(final boolean page:new boolean[]{false,true}){Button b=new Button(this);b.setText(page?"Bridge":"Calls");b.setAllCaps(false);b.setSelected(page==bridgePage);if(page==bridgePage){b.setBackgroundTintList(android.content.res.ColorStateList.valueOf(Color.rgb(242,183,85)));b.setTextColor(Color.rgb(16,19,22));}b.setOnClickListener(new View.OnClickListener(){public void onClick(View v){bridgePage=page;render();}});nav.addView(b,new LinearLayout.LayoutParams(0,dp(52),1));}root.addView(nav);
  ScrollView scroll=new ScrollView(this);content=new LinearLayout(this);content.setOrientation(LinearLayout.VERTICAL);scroll.addView(content);root.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));setContentView(root);
  if(bridgePage)settings();else phone();
 }
 private void settings(){
  state=text(GatewayService.listening?"Bridge active":enabled()?"Bridge unavailable":"Bridge off",25);
  text(Enrollment.exists(this)?"Desktop paired":"No desktop paired",16);
  button("Export pairing request",new Runnable(){public void run(){exportRequest();}});
  button("Import pairing response",new Runnable(){public void run(){
   if(enabled()||GatewayService.running){error("Stop the bridge before you import a pairing response.");return;}
   Intent pick=new Intent(Intent.ACTION_OPEN_DOCUMENT).setType("*/*").addCategory(Intent.CATEGORY_OPENABLE);startActivityForResult(pick,12);
  }});
  Button start=button(enabled()?"Stop bridge":"Start bridge",new Runnable(){public void run(){
   if(enabled()){GatewayService.disable(DialActivity.this);render();return;}
   try{Enrollment.active(DialActivity.this);getSharedPreferences("gateway",0).edit().putBoolean("enabled",true).commit();startForegroundService(new Intent(DialActivity.this,GatewayService.class));render();}
   catch(Exception e){getSharedPreferences("gateway",0).edit().putBoolean("enabled",false).commit();error("Pair a desktop with valid certificates before starting the bridge.");}
  }});start.setEnabled(Enrollment.exists(this)||enabled());start.setBackgroundTintList(android.content.res.ColorStateList.valueOf(Color.rgb(242,183,85)));start.setTextColor(Color.rgb(16,19,22));
  Switch boot=new Switch(this);boot.setText("Start after reboot and unlock");boot.setMinHeight(dp(52));boot.setChecked(getSharedPreferences("gateway",0).getBoolean("boot",false));boot.setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener(){public void onCheckedChanged(CompoundButton b,boolean checked){getSharedPreferences("gateway",0).edit().putBoolean("boot",checked).commit();}});content.addView(boot);
  button(checkSelfPermission(Manifest.permission.READ_PHONE_STATE)==PackageManager.PERMISSION_GRANTED?"Phone status permission granted":"Allow phone status",new Runnable(){public void run(){requestPermissions(new String[]{Manifest.permission.READ_PHONE_STATE},21);}});
  button(role()?"Default phone app selected":"Set default phone app",new Runnable(){public void run(){if(!role())startActivityForResult(getSystemService(RoleManager.class).createRequestRoleIntent(RoleManager.ROLE_DIALER),22);}});
  Switch controls=new Switch(this);controls.setText("Desktop call controls");controls.setMinHeight(dp(52));controls.setChecked(getSharedPreferences("gateway",0).getBoolean("controls",false));controls.setEnabled(Enrollment.exists(this)&&role());controls.setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener(){public void onCheckedChanged(CompoundButton b,boolean checked){try{String pairing=checked?Enrollment.active(DialActivity.this).requestId:"";getSharedPreferences("gateway",0).edit().putBoolean("controls",checked).putString("controls_for",pairing).putString("control_epoch",UUID.randomUUID().toString()).commit();}catch(Exception e){b.setChecked(false);error("Pair a desktop with valid certificates first.");}}});content.addView(controls);
  Switch outgoing=new Switch(this);outgoing.setText("Desktop outgoing calls");outgoing.setMinHeight(dp(52));
  outgoing.setChecked(getSharedPreferences("gateway",0).getBoolean("outgoing",false));outgoing.setEnabled(Enrollment.exists(this)&&role());
  outgoing.setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener(){public void onCheckedChanged(CompoundButton b,boolean checked){
   if(checked&&(checkSelfPermission(Manifest.permission.CALL_PHONE)!=PackageManager.PERMISSION_GRANTED||checkSelfPermission(Manifest.permission.READ_PHONE_STATE)!=PackageManager.PERMISSION_GRANTED)){
    b.setChecked(false);requestPermissions(new String[]{Manifest.permission.CALL_PHONE,Manifest.permission.READ_PHONE_STATE},25);return;
   }
   try{String pairing=checked?Enrollment.active(DialActivity.this).requestId:"";
    if(!getSharedPreferences("gateway",0).edit().putBoolean("outgoing",checked).putString("outgoing_for",pairing).putString("outgoing_epoch",UUID.randomUUID().toString()).commit())throw new IOException();
   }catch(Exception e){b.setChecked(false);error("Outgoing call authorization could not be saved.");}
  }});content.addView(outgoing);
  button("Allow call notifications",new Runnable(){public void run(){requestPermissions(new String[]{Manifest.permission.POST_NOTIFICATIONS},23);}});
 }
 private void phone(){
  state=null;
  if(CallService.notificationFailed)text("Call notification unavailable. Keep this screen open.",14);
  if(!getSystemService(NotificationManager.class).areNotificationsEnabled())text("Call alerts are off. Enable notifications in Bridge settings.",14);
  for(final Map.Entry<Call,Integer> entry:CallService.calls.entrySet()){
   final Call call=entry.getKey();text(CallService.label(call),18);int current=call.getDetails().getState();
   text(current==Call.STATE_RINGING?"Incoming call":current==Call.STATE_ACTIVE?"Call in progress":"Call state: "+current,20);
   if(current==Call.STATE_RINGING){
    button("Answer",new Runnable(){public void run(){CallService.localAction(CallService.generation,entry.getValue(),"answer");handler.postDelayed(new Runnable(){public void run(){render();}},400);}});
    button("Decline",new Runnable(){public void run(){CallService.localAction(CallService.generation,entry.getValue(),"reject");handler.postDelayed(new Runnable(){public void run(){render();}},400);}});
   }else button("End call",new Runnable(){public void run(){CallService.localAction(CallService.generation,entry.getValue(),"disconnect");handler.postDelayed(new Runnable(){public void run(){render();}},400);}});
  }
  text("Phone number",16);number=new EditText(this);number.setInputType(android.text.InputType.TYPE_CLASS_PHONE);number.setSingleLine(true);number.setContentDescription("Phone number");number.setMinHeight(dp(52));
  number.setText(draftNumber);Uri data=getIntent().getData();if(data!=null&&"tel".equals(data.getScheme())&&draftNumber.isEmpty())number.setText(data.getSchemeSpecificPart());content.addView(number);
  button("Call",new Runnable(){public void run(){
   if(!role()){error("Select Omarchy Phone as the default phone app in Bridge settings.");return;}
   if(checkSelfPermission(Manifest.permission.CALL_PHONE)!=PackageManager.PERMISSION_GRANTED){requestPermissions(new String[]{Manifest.permission.CALL_PHONE},24);return;}
   String digits=number.getText().toString().trim();if(!digits.matches("[+0-9*#() -]{1,40}")){number.setError("Enter a phone number");return;}
   getSystemService(TelecomManager.class).placeCall(Uri.fromParts("tel",digits,null),new Bundle());
  }});
 }
 private void exportRequest(){
  busy=true;io.execute(new Runnable(){public void run(){try{
   final byte[] bytes=Enrollment.request(DialActivity.this);
   runOnUiThread(new Runnable(){public void run(){busy=false;if(isDestroyed())return;exported=bytes;Intent save=new Intent(Intent.ACTION_CREATE_DOCUMENT).setType("application/json").addCategory(Intent.CATEGORY_OPENABLE).putExtra(Intent.EXTRA_TITLE,"omarchy-pairing-request.json");startActivityForResult(save,11);}});
  }catch(Exception e){runOnUiThread(new Runnable(){public void run(){busy=false;error("Could not create the pairing request.");}});}}});
 }
 private static String hash(byte[] bytes)throws Exception{StringBuilder out=new StringBuilder();for(byte b:MessageDigest.getInstance("SHA-256").digest(bytes))out.append(String.format(Locale.ROOT,"%02x",b&255));return out.toString();}
 @Override protected void onActivityResult(int request,int result,Intent data){
  super.onActivityResult(request,result,data);if(result!=RESULT_OK||data==null||data.getData()==null)return;
  final Uri uri=data.getData();final int operation=request;if(operation!=11&&operation!=12)return;
  final byte[] requestBytes=exported;if(operation==11&&requestBytes==null){error("Export the pairing request again.");return;}
  busy=true;io.execute(new Runnable(){public void run(){try{
   if(operation==11){try(OutputStream out=getContentResolver().openOutputStream(uri,"wt")){if(out==null)throw new IOException();out.write(requestBytes);}final String fingerprint=hash(requestBytes);runOnUiThread(new Runnable(){public void run(){busy=false;if(!isDestroyed())new AlertDialog.Builder(DialActivity.this).setTitle("Request exported").setMessage("Compare this request SHA-256 with the enrollment tool:\n\n"+fingerprint).setPositiveButton("OK",null).show();}});}
   else{
    final byte[] candidate;try(InputStream in=getContentResolver().openInputStream(uri)){if(in==null)throw new IOException();candidate=EnrollmentBundle.readBounded(in,65536);}
    final String description="Compare this response SHA-256 with the enrollment tool:\n\n"+hash(candidate)+"\n\n"+Enrollment.inspect(DialActivity.this,candidate);
    runOnUiThread(new Runnable(){public void run(){busy=false;if(isDestroyed())return;new AlertDialog.Builder(DialActivity.this).setTitle("Pair this desktop?").setMessage(description).setNegativeButton("Cancel",null).setPositiveButton("Pair desktop",new DialogInterface.OnClickListener(){public void onClick(DialogInterface dialog,int which){activate(candidate);}}).show();}});
   }
  }catch(Exception e){runOnUiThread(new Runnable(){public void run(){busy=false;error("The pairing file could not be read or validated. Existing pairing is unchanged.");}});}}});
 }
 private void activate(final byte[] candidate){busy=true;io.execute(new Runnable(){public void run(){try{Enrollment.activate(DialActivity.this,candidate);runOnUiThread(new Runnable(){public void run(){busy=false;if(!isDestroyed())render();}});}catch(Exception e){runOnUiThread(new Runnable(){public void run(){busy=false;error("Pairing was not changed. Stop the bridge and check the response file.");}});}}});}
 @Override public void onRequestPermissionsResult(int request,String[] permissions,int[] results){super.onRequestPermissionsResult(request,permissions,results);render();}
}
