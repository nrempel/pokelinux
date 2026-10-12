package org.pokelinux.radiopeer;

import android.app.Instrumentation;
import android.os.*;
import java.io.ByteArrayOutputStream;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

/** Independent synthetic endpoint. No vendor code, network or modem access. */
public final class Peer extends Instrumentation {
    private static final String RADIO = "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternal";
    private volatile IBinder response;
    private volatile IBinder indication;
    private volatile IBinder previousResponse;
    private final ByteArrayOutputStream segments = new ByteArrayOutputStream();
    private int segmentIndex, segmentSerial, segmentTotal;
    private volatile int rawCalls, segmentCalls;
    private volatile boolean cleared;
    private final CountDownLatch registrationEntered = new CountDownLatch(1);
    private final CountDownLatch registrationRelease = new CountDownLatch(1);
    private static void awaitLatch(CountDownLatch latch) {
        try { require(latch.await(10, TimeUnit.SECONDS), "registration latch timeout"); }
        catch (InterruptedException error) {
            Thread.currentThread().interrupt();
            throw new IllegalStateException(error);
        }
    }
    private static void require(boolean ok, String reason) {
        if (!ok) throw new IllegalArgumentException(reason);
    }
    private static Parcel responseParcel(int serial, byte[] bytes) {
        Parcel data = Parcel.obtain();
        data.writeInterfaceToken(RADIO + "Res");
        data.writeInt(1); data.writeInt(24);
        for (int value : new int[]{0, serial, 137, bytes.length, 0}) data.writeInt(value);
        data.writeByteArray(bytes);
        return data;
    }
    private void respond(int serial, byte[] bytes) throws RemoteException {
        Parcel data = responseParcel(serial, bytes);
        try {
            require(response.transact(1, data, null, IBinder.FLAG_ONEWAY), "callback submission");
        } finally { data.recycle(); }
    }
    private void indicate(int message, byte[] bytes, int index, int total) throws RemoteException {
        Parcel data = Parcel.obtain();
        try {
            data.writeInterfaceToken(RADIO + "Ind");
            data.writeInt(message); data.writeInt(0); data.writeByteArray(bytes);
            if (index >= 0) { data.writeInt(index); data.writeInt(total); }
            require(indication.transact(index < 0 ? 1 : 2, data, null, IBinder.FLAG_ONEWAY), "indication submission");
        } finally { data.recycle(); }
    }
    private final Binder endpoint = new Binder() {
        { attachInterface(null, RADIO); }
        @Override protected boolean onTransact(int code, Parcel data, Parcel reply, int flags) throws RemoteException {
            if (code == INTERFACE_TRANSACTION) return super.onTransact(code, data, reply, flags);
            data.enforceInterface(RADIO);
            // Linux Binder transport adds TF_ACCEPT_FDS (0x10); it does not change call mode.
            int callFlags = flags & ~0x10;
            if (code == 5) {
                require(callFlags == 0, "registration flags: " + flags);
                response = data.readStrongBinder();
                indication = data.readStrongBinder();
                require(response != null && indication != null && response != indication, "distinct callbacks");
                require((RADIO + "Res").equals(response.getInterfaceDescriptor()), "response descriptor");
                require((RADIO + "Ind").equals(indication.getInterfaceDescriptor()), "indication descriptor");
                String client = data.readString();
                data.enforceNoDataAvail();
                if ("pokelinux-die-register-test".equals(client)) {
                    // Deliberately exit this test process before writing a sync reply.
                    android.os.Process.killProcess(android.os.Process.myPid());
                    return true;
                }
                if ("pokelinux-permanent-hold-test".equals(client)) {
                    // Deliberate permanent test-only Binder blockage. The outer
                    // harness owns and force-stops this whole synthetic peer.
                    registrationEntered.countDown();
                    try { new CountDownLatch(1).await(); }
                    catch (InterruptedException error) {
                        Thread.currentThread().interrupt(); throw new IllegalStateException(error);
                    }
                    throw new IllegalStateException("unreachable permanent release");
                }
                if ("pokelinux-hold-register-test".equals(client)) {
                    // Independent, bounded synthetic barrier. No modem access.
                    registrationEntered.countDown();
                    awaitLatch(registrationRelease);
                    reply.writeNoException(); reply.writeInt(42);
                    return true;
                }
                require("pokelinux-ipc-test".equals(client), "client name");
                // Nested synchronous metadata call before registration returns.
                cleared = false;
                Parcel in = Parcel.obtain(), out = Parcel.obtain();
                try {
                    in.writeInterfaceToken(RADIO + "Res");
                    require(response.transact(16777215, in, out, 0), "version transaction");
                    out.readException(); require(out.readInt() == 1, "response version");
                    out.enforceNoDataAvail();
                } finally { in.recycle(); out.recycle(); }
                respond(7, new byte[]{7});
                indicate(33, new byte[]{1, 2, 3}, -1, 0);
                indicate(34, new byte[]{4, 5}, 0, 5);
                indicate(34, new byte[]{6, 7, 8}, 1, 5);
                reply.writeNoException(); reply.writeInt(42);
                return true;
            }
            if (code == 2 || code == 3) {
                require(callFlags == IBinder.FLAG_ONEWAY && response != null && !cleared, "request state/flags");
                int serial = data.readInt();
                require(data.readInt() == 42 && data.readInt() == 137 && data.readInt() == 0, "routing IDs");
                byte[] bytes = data.createByteArray();
                require(bytes != null && bytes.length <= 2048, "part size");
                if (code == 2) { data.enforceNoDataAvail(); rawCalls++; respond(serial, bytes); }
                else {
                    int index = data.readInt(), total = data.readInt(); data.enforceNoDataAvail();
                    require(total > 2048 && total <= 4096, "test total bound");
                    if (index == 0) { require(segments.size() == 0, "no overlapping message"); segmentSerial=serial; segmentTotal=total; segmentIndex=0; }
                    require(serial == segmentSerial && total == segmentTotal && index == segmentIndex++, "segment order");
                    require(segments.size() + bytes.length <= total, "segment bound");
                    segments.write(bytes, 0, bytes.length); segmentCalls++;
                    if (segments.size() == total) { respond(serial, segments.toByteArray()); segments.reset(); }
                }
                return true;
            }
            if (code == 1) {
                require(callFlags == IBinder.FLAG_ONEWAY && data.readInt() == 42, "clear framing");
                data.enforceNoDataAvail(); cleared = true; previousResponse = response; response = null; indication = null; return true;
            }
            if (code == 105) { // Test-only bounded inbox pressure, not a vendor transaction.
                require(callFlags == 0 && indication != null && !cleared, "burst state/flags");
                data.enforceNoDataAvail();
                for (int i = 0; i < 5; i++) indicate(99, new byte[]{1}, -1, 0);
                reply.writeNoException(); return true;
            }
            if (code == 106) { // Test-only delayed malformed callback from this process.
                require(callFlags == 0 && indication != null && !cleared, "malformed trigger state/flags");
                data.enforceNoDataAvail();
                final IBinder target = indication;
                new Thread(new Runnable() { @Override public void run() {
                    try {
                        Thread.sleep(100);
                        Parcel input = Parcel.obtain();
                        try {
                            input.writeInterfaceToken(RADIO + "Ind");
                            input.writeInt(99); input.writeInt(0);
                            // Required byte array deliberately absent.
                            require(target.transact(1, input, null, IBinder.FLAG_ONEWAY), "malformed submission");
                        } finally { input.recycle(); }
                    } catch (InterruptedException error) { Thread.currentThread().interrupt(); }
                    catch (RemoteException error) { throw new IllegalStateException(error); }
                } }, "malformed-callback-test").start();
                reply.writeNoException(); return true;
            }
            if (code == 107) { // Exit only this disposable test peer, after replying.
                require(callFlags == 0 && indication != null && !cleared, "exit state/flags");
                data.enforceNoDataAvail();
                new Thread(new Runnable() { @Override public void run() {
                    try { Thread.sleep(100); }
                    catch (InterruptedException error) { Thread.currentThread().interrupt(); return; }
                    android.os.Process.killProcess(android.os.Process.myPid());
                } }, "peer-exit-test").start();
                reply.writeNoException(); return true;
            }
            if (code == 108) { // Late callback to a still-live, failed old endpoint.
                require(callFlags == 0 && previousResponse != null && response != null && !cleared, "late callback state/flags");
                data.enforceNoDataAvail();
                require(previousResponse.pingBinder(), "old endpoint live before late callback");
                Parcel input = responseParcel(777, new byte[]{99}), output = Parcel.obtain();
                String rejection = null;
                try {
                    // Synchronous test-only delivery makes rejection observable;
                    // the actual vendor sender's flag mode is still unqualified.
                    require(previousResponse.transact(1, input, output, 0), "late callback transaction handled");
                } catch (RemoteException error) { rejection = error.getClass().getName(); }
                finally { input.recycle(); output.recycle(); }
                require(rejection != null && previousResponse.pingBinder(), "old live endpoint rejected late callback");
                input = responseParcel(777, new byte[]{7}); output = Parcel.obtain();
                try { require(response.transact(1, input, output, 0), "new endpoint accepts same-mode callback"); }
                finally { input.recycle(); output.recycle(); }
                reply.writeNoException(); reply.writeString(rejection); return true;
            }
            if (code == 109 || code == 110) { // Test-only registration barriers.
                require(callFlags == 0, "registration barrier flags");
                data.enforceNoDataAvail();
                if (code == 110) awaitLatch(registrationEntered);
                else registrationRelease.countDown();
                reply.writeNoException(); reply.writeInt(1); return true;
            }
            if (code == 100) { // Test-only observation, never called by production transport.
                data.enforceNoDataAvail(); reply.writeNoException();
                reply.writeInt(rawCalls); reply.writeInt(segmentCalls); reply.writeInt(cleared ? 1 : 0);
                reply.writeInt(android.os.Process.myPid()); reply.writeInt(android.os.Process.myUid());
                try { reply.writeString(new String(Files.readAllBytes(Paths.get("/proc/self/attr/current"))).trim()); }
                catch (Exception e) { throw new IllegalStateException(e); }
                return true;
            }
            return false;
        }
    };
    @Override public void onCreate(Bundle args) { start(); }
    @Override public void onStart() {
        Bundle result = new Bundle(); result.putBinder("peer", endpoint); sendStatus(37, result);
        // A bounded disposable peer lifetime; harness force-stops it on completion.
        try { Thread.sleep(45000); } catch (InterruptedException e) { Thread.currentThread().interrupt(); }
        finish(0, new Bundle());
    }
}
