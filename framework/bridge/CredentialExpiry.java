package org.omarchy.bridge;
import java.io.IOException;

/** Per-exchange credential cutoff anchored once to elapsed time after authentication. */
final class CredentialExpiry {
 private final long wallExpires,elapsedExpires;
 CredentialExpiry(long elapsed,long wall,long expires)throws IOException{
  if(elapsed<0||expires<=wall)throw new IOException("Credential expired");
  wallExpires=expires;
  long remaining;
  try{remaining=Math.min(30000,Math.subtractExact(expires,wall));}
  catch(ArithmeticException overflow){remaining=30000;}
  // No bridge exchange is allowed more than 30 seconds. Capping here also
  // avoids overflow from arbitrarily distant certificate dates.
  try{elapsedExpires=Math.addExact(elapsed,remaining);}
  catch(ArithmeticException overflow){throw new IOException("Credential deadline overflow");}
 }
 long cap(long deadline){return Math.min(deadline,elapsedExpires);}
 boolean valid(long elapsed,long wall){return elapsed<elapsedExpires&&wall<wallExpires;}
}
