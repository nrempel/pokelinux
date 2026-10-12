package org.omarchy.bridge;
import android.content.Context;
import java.security.MessageDigest;
import javax.net.ssl.SSLContext;
/** Production identity: only an explicitly selected AndroidKeyStore enrollment. */
final class GatewayIdentity {
 final SSLContext context;
 final byte[] pin;
 final String enrollment;
 final long expires;
 GatewayIdentity(SSLContext context,byte[] pin,String enrollment,long expires){this.context=context;this.pin=pin;this.enrollment=enrollment;this.expires=expires;}
 static boolean exists(Context c){return Enrollment.exists(c);}
 static GatewayIdentity open(Context c)throws Exception{
  EnrollmentBundle value=Enrollment.active(c);
  return new GatewayIdentity(Enrollment.context(c),MessageDigest.getInstance("SHA-256").digest(value.client.getEncoded()),value.requestId,
   Math.min(value.server.getNotAfter().getTime(),Math.min(value.client.getNotAfter().getTime(),value.ca.getNotAfter().getTime())));
 }
}
