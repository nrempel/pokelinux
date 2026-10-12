package org.pokelinux.camerapeer;
import android.app.Instrumentation;
import android.os.Bundle;
import android.os.IBinder;
/** Disposable Java bootstrap only. All probe dispatch is original generated NDK code. */
public final class Peer extends Instrumentation {
    static { System.loadLibrary("camera_boundary_peer"); }
    private static native IBinder createEndpoint();
    @Override public void onCreate(Bundle arguments) {
        super.onCreate(arguments);
        Bundle status = new Bundle();
        status.putBinder("peer", createEndpoint());
        sendStatus(37, status);
    }
}
