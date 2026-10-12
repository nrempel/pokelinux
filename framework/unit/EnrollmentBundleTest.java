package org.omarchy.bridge;
import java.nio.file.*;
import java.security.*;
import java.security.spec.*;
import java.util.*;
public final class EnrollmentBundleTest {
 interface Check {void run() throws Exception;}
 static int rejected;
 static void reject(Check check)throws Exception{try{check.run();throw new AssertionError("accepted invalid bundle");}catch(IllegalArgumentException|GeneralSecurityException|java.io.IOException expected){rejected++;}}
 public static void main(String[] args)throws Exception{
  byte[] bytes=Files.readAllBytes(Paths.get(args[0]));
  PublicKey key=KeyFactory.getInstance("EC").generatePublic(new X509EncodedKeySpec(Files.readAllBytes(Paths.get(args[1]))));
  EnrollmentBundle valid=EnrollmentBundle.parse(bytes,"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",key);
  if(valid.server.getBasicConstraints()!=-1||valid.client.getExtendedKeyUsage().size()!=1)throw new AssertionError();
  reject(()->EnrollmentBundle.parse(bytes,"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",key));
  KeyPairGenerator generator=KeyPairGenerator.getInstance("EC");generator.initialize(new ECGenParameterSpec("secp256r1"));
  reject(()->EnrollmentBundle.parse(bytes,"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",generator.generateKeyPair().getPublic()));
  reject(()->EnrollmentBundle.parse(Arrays.copyOf(bytes,bytes.length/2),"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",key));
  reject(()->EnrollmentBundle.parse(new byte[65537],"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",key));
  if(args.length>2){try(java.nio.file.DirectoryStream<Path> fixtures=Files.newDirectoryStream(Paths.get(args[2]),"*.zip")){
   for(final Path fixture:fixtures)reject(()->EnrollmentBundle.parse(Files.readAllBytes(fixture),"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",key));
  }}
  System.out.println("EnrollmentBundle: valid chain and "+rejected+" rejection controls passed");
 }
}
