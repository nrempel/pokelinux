package org.omarchy.bridge;
import java.io.*;
import java.net.*;
import java.security.*;
import java.util.concurrent.*;
import javax.net.ssl.*;
/** Real host-JSSE mutual TLS. Does not claim Android Conscrypt coverage. */
public final class BridgeTransportTlsTest {
 static SSLContext context(String own,String peer)throws Exception{
  char[] password="disposable-test".toCharArray();KeyStore keys=KeyStore.getInstance("PKCS12"),other=KeyStore.getInstance("PKCS12"),trust=KeyStore.getInstance("PKCS12");
  try(InputStream input=new FileInputStream(own)){keys.load(input,password);}
  try(InputStream input=new FileInputStream(peer)){other.load(input,password);}
  trust.load(null,null);trust.setCertificateEntry("peer",other.getCertificate("identity"));
  KeyManagerFactory km=KeyManagerFactory.getInstance(KeyManagerFactory.getDefaultAlgorithm());km.init(keys,password);
  TrustManagerFactory tm=TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm());tm.init(trust);
  SSLContext result=SSLContext.getInstance("TLSv1.3");result.init(km.getKeyManagers(),tm.getTrustManagers(),null);return result;
 }
 public static void main(String[] args)throws Exception{
  SSLContext server=context(args[0],args[1]),client=context(args[1],args[0]);
  ExecutorService worker=Executors.newSingleThreadExecutor();CompletableFuture<BridgeTransport> accepted=new CompletableFuture<>();CountDownLatch waiting=new CountDownLatch(1);
  try(ServerSocket listener=new ServerSocket(0,1,InetAddress.getLoopbackAddress())){
   Future<?> task=worker.submit(()->{
    try{
     Socket raw=listener.accept();BridgeTransport transport=new BridgeTransport(raw);accepted.complete(transport);
     try{
      SSLSocket tls=(SSLSocket)server.getSocketFactory().createSocket(raw,"localhost",raw.getPort(),true);transport.attach(tls);
      tls.setUseClientMode(false);tls.setEnabledProtocols(new String[]{"TLSv1.3"});tls.setNeedClientAuth(true);tls.setSoTimeout(5000);tls.startHandshake();
      if(tls.getSession().getPeerCertificates().length<1||tls.getInputStream().read()!=1)throw new AssertionError();
      tls.getOutputStream().write(2);tls.getOutputStream().flush();waiting.countDown();
      try{tls.getInputStream().read();throw new AssertionError("read survived abort");}catch(IOException expected){}
     }finally{transport.finish();}
    }catch(Exception e){throw new RuntimeException(e);}
   });
   try(SSLSocket tls=(SSLSocket)client.getSocketFactory().createSocket("localhost",listener.getLocalPort())){
    tls.setSoTimeout(5000);SSLParameters parameters=tls.getSSLParameters();parameters.setEndpointIdentificationAlgorithm("HTTPS");tls.setSSLParameters(parameters);tls.startHandshake();
    tls.getOutputStream().write(1);tls.getOutputStream().flush();if(tls.getInputStream().read()!=2)throw new AssertionError();
    if(!waiting.await(1,TimeUnit.SECONDS))throw new AssertionError();
    accepted.get(1,TimeUnit.SECONDS).abort();task.get(2,TimeUnit.SECONDS);
   }
  }finally{if(accepted.isDone())accepted.get().abort();worker.shutdownNow();}
  System.out.println("BridgeTransport TLS: server-mode wrapped socket, mutual authentication, exchange and raw abort passed on host JSSE");
 }
}
