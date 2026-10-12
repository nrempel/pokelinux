package org.pokelinux.smsreceiptpeer;
import android.app.Instrumentation;
import android.os.Bundle;
import android.os.IBinder;
/** Bootstrap only; the independent endpoint uses original generated public NDK Binder code. */
public final class Peer extends Instrumentation {
    static { System.loadLibrary("sms_receipt_peer"); }
    private static native IBinder createEndpoint();
    @Override public void onCreate(Bundle args) {
        super.onCreate(args);
        Bundle status = new Bundle(); status.putBinder("peer", createEndpoint()); sendStatus(37, status);
    }
}
