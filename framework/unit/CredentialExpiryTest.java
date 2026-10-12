package org.omarchy.bridge;
public final class CredentialExpiryTest {
 static void require(boolean value){if(!value)throw new AssertionError();}
 public static void main(String[] args)throws Exception{
  CredentialExpiry expiry=new CredentialExpiry(1000,10000,10500);
  require(expiry.cap(31000)==1500);require(expiry.cap(1200)==1200);
  require(expiry.valid(1499,10499));
  require(!expiry.valid(1500,9000)); // Clock rollback cannot extend this exchange.
  require(!expiry.valid(1100,10500)); // Forward jump fails closed immediately.
  require(!expiry.valid(1500,10500));
  try{new CredentialExpiry(1000,10500,10500);throw new AssertionError();}catch(java.io.IOException expected){}
  try{new CredentialExpiry(1000,10501,10500);throw new AssertionError();}catch(java.io.IOException expected){}
  CredentialExpiry distant=new CredentialExpiry(1000,-10000,Long.MAX_VALUE);
  require(distant.cap(Long.MAX_VALUE)==31000);
  try{new CredentialExpiry(Long.MAX_VALUE,10000,10500);throw new AssertionError();}catch(java.io.IOException expected){}
  Admission command=new Admission(()->1500L,()->9000L,()->false,expiry.cap(5500),10500);
  require(!command.commit());
  System.out.println("CredentialExpiry: exact cutoff, rollback, forward jump, already expired and arithmetic bounds passed");
 }
}
