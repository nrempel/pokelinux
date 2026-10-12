package org.omarchy.bridge;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.security.*;
import java.security.cert.*;
import java.security.interfaces.ECPublicKey;
import java.util.*;
import java.util.zip.*;
/** Strict, bounded public-only enrollment format. Does not write or activate state. */
final class EnrollmentBundle {
 final X509Certificate server,client,ca;
 final String requestId;
 final byte[] encoded;
 private EnrollmentBundle(String id,byte[] bytes,X509Certificate server,X509Certificate client,X509Certificate ca){
  this.requestId=id;this.encoded=bytes.clone();this.server=server;this.client=client;this.ca=ca;
 }
 static byte[] readBounded(InputStream in,int limit)throws IOException{
  ByteArrayOutputStream out=new ByteArrayOutputStream();byte[] buffer=new byte[4096];int n;
  while((n=in.read(buffer))!=-1){if(n>limit-out.size())throw new IOException("Size limit");out.write(buffer,0,n);}return out.toByteArray();
 }
 static EnrollmentBundle parse(byte[] bytes,String id,PublicKey publicKey)throws Exception{
  if(bytes.length>65536)throw new IOException("Bundle too large");
  Set<String> names=new HashSet<>(Arrays.asList("request-id","server.der","client.der","ca.der"));
  Map<String,byte[]> parts=new HashMap<>();int total=0;
  try(ZipInputStream in=new ZipInputStream(new ByteArrayInputStream(bytes))){
   ZipEntry entry;while((entry=in.getNextEntry())!=null){
    if(entry.isDirectory()||!names.contains(entry.getName())||parts.containsKey(entry.getName()))throw new IOException("Bundle entries");
    byte[] value=readBounded(in,16384);total+=value.length;if(total>32768)throw new IOException("Expanded size");
    parts.put(entry.getName(),value);
   }
  }
  if(!parts.keySet().equals(names))throw new IOException("Missing bundle entries");
  if(id==null||!id.matches("[0-9a-f]{32}")||!Arrays.equals(parts.get("request-id"),id.getBytes(StandardCharsets.US_ASCII)))throw new GeneralSecurityException("Request mismatch");
  X509Certificate server=certificate(parts.get("server.der")),client=certificate(parts.get("client.der")),ca=certificate(parts.get("ca.der"));
  if(!MessageDigest.isEqual(server.getPublicKey().getEncoded(),publicKey.getEncoded()))throw new GeneralSecurityException("Key mismatch");
  if(ca.getBasicConstraints()!=0||!ca.getSubjectX500Principal().equals(ca.getIssuerX500Principal()))throw new GeneralSecurityException("CA constraints");
  ca.verify(ca.getPublicKey());
  if(ca.getKeyUsage()==null||ca.getKeyUsage().length<6||!ca.getKeyUsage()[5])throw new GeneralSecurityException("CA key usage");
  validateLeaf(server,ca,"1.3.6.1.5.5.7.3.1");validateLeaf(client,ca,"1.3.6.1.5.5.7.3.2");
  Collection<List<?>> sans=server.getSubjectAlternativeNames();
  if(sans==null||sans.size()!=1)throw new GeneralSecurityException("Server name");
  List<?> san=sans.iterator().next();if(!Integer.valueOf(2).equals(san.get(0))||!"omarchy-android".equals(san.get(1)))throw new GeneralSecurityException("Server name");
  return new EnrollmentBundle(id,bytes,server,client,ca);
 }
 private static X509Certificate certificate(byte[] bytes)throws Exception{
  ByteArrayInputStream in=new ByteArrayInputStream(bytes);
  X509Certificate cert=(X509Certificate)CertificateFactory.getInstance("X.509").generateCertificate(in);
  if(in.available()!=0||!Arrays.equals(cert.getEncoded(),bytes))throw new GeneralSecurityException("Non-canonical certificate");
  cert.checkValidity();
  if(cert.getNotAfter().getTime()-System.currentTimeMillis()>91L*86400000)throw new GeneralSecurityException("Certificate lifetime");
  if(!(cert.getPublicKey() instanceof ECPublicKey)||((ECPublicKey)cert.getPublicKey()).getParams().getOrder().bitLength()!=256)throw new GeneralSecurityException("Expected EC P-256 key");
  if(!"1.2.840.10045.4.3.2".equals(cert.getSigAlgOID()))throw new GeneralSecurityException("Expected ECDSA SHA-256");
  return cert;
 }
 private static void validateLeaf(X509Certificate leaf,X509Certificate ca,String purpose)throws Exception{
  if(leaf.getBasicConstraints()!=-1||!Collections.singletonList(purpose).equals(leaf.getExtendedKeyUsage()))throw new GeneralSecurityException("Certificate purpose");
  boolean[] usage=leaf.getKeyUsage();if(usage==null||!usage[0])throw new GeneralSecurityException("Certificate key usage");
  PKIXParameters params=new PKIXParameters(Collections.singleton(new TrustAnchor(ca,null)));params.setRevocationEnabled(false);
  CertPath path=CertificateFactory.getInstance("X.509").generateCertPath(Collections.singletonList(leaf));
  CertPathValidator.getInstance("PKIX").validate(path,params);
 }
 static String fingerprint(java.security.cert.Certificate certificate)throws GeneralSecurityException{
  byte[] hash=MessageDigest.getInstance("SHA-256").digest(certificate.getEncoded());StringBuilder text=new StringBuilder();
  for(byte b:hash)text.append(String.format(Locale.ROOT,"%02x",b&255));return text.toString();
 }
}
