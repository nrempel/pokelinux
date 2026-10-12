package org.omarchy.bridge;
import java.io.IOException;
import java.net.Socket;

/** Own the raw socket independently of TLS, whose close may perform network I/O. */
final class BridgeTransport {
 final Socket raw;
 private volatile Socket tls;
 BridgeTransport(Socket raw){this.raw=raw;}
 // Only the authentication worker attaches; ownership then passes to at most
 // one watch worker. Cancellation can race either worker, but never attaches.
 void attach(Socket wrapper)throws IOException{
  if(tls!=null)throw new IllegalStateException("TLS already attached");
  tls=wrapper;
  if(raw.isClosed())throw new IOException("Transport already closed");
 }
 boolean isClosed(){return raw.isClosed();}
 // Safe cancellation path: never call TLS code or hold a connection monitor.
 void abort(){try{raw.close();}catch(IOException ignored){}}
 // Worker cleanup only. The raw socket is already closed before TLS cleanup.
 void finish(){abort();Socket wrapper=tls;if(wrapper!=null)try{wrapper.close();}catch(IOException ignored){}}
}
