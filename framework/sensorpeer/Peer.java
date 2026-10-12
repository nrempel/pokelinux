package org.pokelinux.sensorpeer;
import android.app.Instrumentation;
import android.os.Bundle;
import android.os.IBinder;
/** Disposable bootstrap only; queues and dispatch are original public native code. */
public final class Peer extends Instrumentation {
    static { System.loadLibrary("sensor_boundary_peer"); }
    private static native IBinder createEndpoint();
    @Override public void onCreate(Bundle arguments) {
        super.onCreate(arguments);
        Bundle status=new Bundle();status.putBinder("peer",createEndpoint());sendStatus(37,status);
    }
}
