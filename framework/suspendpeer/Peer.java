package org.pokelinux.suspendpeer;
import android.app.Instrumentation;
import android.os.Bundle;
import android.os.IBinder;
/** Disposable discovery bootstrap; all probe dispatch is generated native NDK. */
public final class Peer extends Instrumentation {
    static { System.loadLibrary("suspend_boundary_peer"); }
    private static native IBinder createEndpoint();
    @Override public void onCreate(Bundle arguments) {
        super.onCreate(arguments);
        Bundle status = new Bundle();
        status.putBinder("peer", createEndpoint());
        sendStatus(37, status);
    }
}
