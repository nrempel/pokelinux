package org.omarchy.bridge;
import android.content.Context;
import android.security.keystore.*;
import android.util.AtomicFile;
import android.util.Base64;
import java.io.*;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.security.*;
import java.security.cert.X509Certificate;
import java.security.spec.ECGenParameterSpec;
import java.util.*;
import javax.net.ssl.*;
import org.json.JSONObject;
/** App-owned non-exportable key; only public certificates cross the enrollment boundary. */
public final class Enrollment {
 private static AtomicFile file(Context c,String name){return new AtomicFile(new File(c.getNoBackupFilesDir(),name));}
 private static KeyStore keys()throws Exception{KeyStore k=KeyStore.getInstance("AndroidKeyStore");k.load(null);return k;}
 private static String alias(String id){if(!id.matches("[0-9a-f]{32}"))throw new IllegalArgumentException("Request id");return "omarchy-server-"+id;}
 private static JSONObject read(Context c,String name)throws Exception{
  try(InputStream in=file(c,name).openRead()){return new JSONObject(new String(EnrollmentBundle.readBounded(in,100000),StandardCharsets.UTF_8));}
 }
 private static void write(Context c,String name,byte[] value)throws IOException{
  AtomicFile destination=file(c,name);FileOutputStream out=null;
  try{out=destination.startWrite();out.write(value);destination.finishWrite(out);}
  catch(IOException e){destination.failWrite(out);throw e;}
 }
 public static synchronized byte[] request(Context c)throws Exception{
  JSONObject pending=null;try{pending=read(c,"pending-enrollment.json");}catch(FileNotFoundException ignored){}
  String active=null;try{active=read(c,"enrollment.json").getString("request_id");}catch(FileNotFoundException ignored){}
  if(pending!=null&&!pending.getString("request_id").equals(active))return pending.toString().getBytes(StandardCharsets.UTF_8);
  String id=UUID.randomUUID().toString().replace("-", "");
  KeyPairGenerator generator=KeyPairGenerator.getInstance(KeyProperties.KEY_ALGORITHM_EC,"AndroidKeyStore");
  generator.initialize(new KeyGenParameterSpec.Builder(alias(id),KeyProperties.PURPOSE_SIGN)
   .setAlgorithmParameterSpec(new ECGenParameterSpec("secp256r1"))
   .setDigests(KeyProperties.DIGEST_NONE,KeyProperties.DIGEST_SHA256)
   .setUserAuthenticationRequired(false).build());
  KeyPair pair=generator.generateKeyPair();
  JSONObject value=new JSONObject();value.put("v",1);value.put("request_id",id);value.put("public_key",Base64.encodeToString(pair.getPublic().getEncoded(),Base64.NO_WRAP));
  byte[] bytes=value.toString().getBytes(StandardCharsets.UTF_8);write(c,"pending-enrollment.json",bytes);return bytes;
 }
 static synchronized EnrollmentBundle preview(Context c,byte[] bytes)throws Exception{
  String id=read(c,"pending-enrollment.json").getString("request_id");
  return EnrollmentBundle.parse(bytes,id,keys().getCertificate(alias(id)).getPublicKey());
 }
 public static synchronized String inspect(Context c,byte[] bytes)throws Exception{
  EnrollmentBundle bundle=preview(c,bytes);
  return "Desktop certificate SHA-256\n"+EnrollmentBundle.fingerprint(bundle.client)+"\n\nPairing CA SHA-256\n"+EnrollmentBundle.fingerprint(bundle.ca)+"\n\nExpires "+bundle.client.getNotAfter();
 }
 public static synchronized void activate(Context c,byte[] bytes)throws Exception{
  if(c.getSharedPreferences("gateway",0).getBoolean("enabled",false)||GatewayService.running)throw new IllegalStateException("Stop the bridge before pairing");
  EnrollmentBundle bundle=preview(c,bytes); // Revalidate at commit, not only preview.
  JSONObject value=new JSONObject();value.put("request_id",bundle.requestId);value.put("bundle",Base64.encodeToString(bytes,Base64.NO_WRAP));
  // One durable record is the commit point. No currently active key or certificate is mutated.
  write(c,"enrollment.json",value.toString().getBytes(StandardCharsets.UTF_8));
  c.getSharedPreferences("gateway",0).edit().putBoolean("controls",false).commit();
 }
 static boolean exists(Context c){return file(c,"enrollment.json").getBaseFile().isFile();}
 static EnrollmentBundle active(Context c)throws Exception{
  JSONObject value=read(c,"enrollment.json");String id=value.getString("request_id");
  return EnrollmentBundle.parse(Base64.decode(value.getString("bundle"),Base64.NO_WRAP),id,keys().getCertificate(alias(id)).getPublicKey());
 }
 static SSLContext context(Context c)throws Exception{
  final EnrollmentBundle bundle=active(c);final String name=alias(bundle.requestId);
  final PrivateKey privateKey=(PrivateKey)keys().getKey(name,null);
  if(privateKey==null||privateKey.getEncoded()!=null)throw new GeneralSecurityException("Expected non-exportable Android key");
  X509ExtendedKeyManager manager=new X509ExtendedKeyManager(){
   public String[] getClientAliases(String type,Principal[] issuers){return null;}
   public String chooseClientAlias(String[] types,Principal[] issuers,Socket socket){return null;}
   public String[] getServerAliases(String type,Principal[] issuers){return "EC".equals(type)?new String[]{name}:null;}
   public String chooseServerAlias(String type,Principal[] issuers,Socket socket){return "EC".equals(type)?name:null;}
   public String chooseEngineServerAlias(String type,Principal[] issuers,SSLEngine engine){return "EC".equals(type)?name:null;}
   public X509Certificate[] getCertificateChain(String requested){return name.equals(requested)?new X509Certificate[]{bundle.server,bundle.ca}:null;}
   public PrivateKey getPrivateKey(String requested){return name.equals(requested)?privateKey:null;}
  };
  KeyStore trust=KeyStore.getInstance(KeyStore.getDefaultType());trust.load(null);trust.setCertificateEntry("paired-ca",bundle.ca);
  TrustManagerFactory tm=TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm());tm.init(trust);
  SSLContext context=SSLContext.getInstance("TLSv1.3");context.init(new KeyManager[]{manager},tm.getTrustManagers(),null);return context;
 }
 public static boolean keyIsNonExportable(Context c)throws Exception{
  String id=read(c,"pending-enrollment.json").getString("request_id");return keys().getKey(alias(id),null).getEncoded()==null;
 }
}
