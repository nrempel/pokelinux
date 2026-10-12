package org.omarchy.bridge;
import android.content.Context;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.security.*;
import javax.net.ssl.*;
/** TEST ONLY: separately built compatibility fixture for malformed peer-certificate cases. */
final class GatewayIdentity {
 final SSLContext context;final byte[] pin;final String enrollment="";final long expires=Long.MAX_VALUE;
 GatewayIdentity(SSLContext c,byte[] p){context=c;pin=p;}
 static boolean exists(Context c){return new File(c.getNoBackupFilesDir(),"identity.p12").isFile();}
 static KeyStore load(Context c,String name,char[] password)throws Exception{
  KeyStore store=KeyStore.getInstance("PKCS12");try(InputStream in=new FileInputStream(new File(c.getNoBackupFilesDir(),name))){store.load(in,password);}return store;
 }
 static String small(Context c,String name)throws IOException{
  try(InputStream in=new FileInputStream(new File(c.getNoBackupFilesDir(),name))){ByteArrayOutputStream out=new ByteArrayOutputStream();int b;while((b=in.read())!=-1){if(out.size()>=256)throw new IOException();out.write(b);}return new String(out.toByteArray(),StandardCharsets.US_ASCII).trim();}
 }
 static GatewayIdentity open(Context c)throws Exception{
  char[] password=small(c,"tls-password").toCharArray();String value=small(c,"client-sha256");if(!value.matches("[0-9a-f]{64}"))throw new GeneralSecurityException();
  byte[] pin=new byte[32];for(int i=0;i<32;i++)pin[i]=(byte)Integer.parseInt(value.substring(i*2,i*2+2),16);
  KeyManagerFactory km=KeyManagerFactory.getInstance(KeyManagerFactory.getDefaultAlgorithm());km.init(load(c,"identity.p12",password),password);
  TrustManagerFactory tm=TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm());tm.init(load(c,"trust.p12",password));java.util.Arrays.fill(password,'\0');
  SSLContext tls=SSLContext.getInstance("TLSv1.3");tls.init(km.getKeyManagers(),tm.getTrustManagers(),null);return new GatewayIdentity(tls,pin);
 }
}
