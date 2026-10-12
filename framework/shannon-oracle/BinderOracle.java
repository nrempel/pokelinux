package org.omarchy.audit;

import android.os.Binder;
import android.os.IBinder;
import android.os.Parcel;
import android.os.RemoteException;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.Arrays;

/** Executes the original proxy against an in-process capture endpoint, never a radio service. */
public final class BinderOracle {
    private static final String DESCRIPTOR =
        "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternal";

    private static final class Capture extends Binder {
        final int[] expected;
        final byte[] payload;
        final boolean accepted;
        int calls;

        Capture(int[] expected, byte[] payload, boolean accepted) {
            this.expected = expected;
            this.payload = payload;
            this.accepted = accepted;
        }

        @Override protected boolean onTransact(int code, Parcel data, Parcel reply, int flags) {
            calls++;
            require(code == 2, "raw transaction code");
            require(flags == IBinder.FLAG_ONEWAY, "one-way flag");
            require(reply == null, "no reply parcel");
            data.enforceInterface(DESCRIPTOR);
            for (int value : expected) require(data.readInt() == value, "integer order/value");
            require(Arrays.equals(data.createByteArray(), payload), "byte-array nullness/content");
            require(data.dataAvail() == 0, "no trailing parcel fields");
            return accepted;
        }
    }

    private static void require(boolean condition, String label) {
        if (!condition) throw new AssertionError(label);
    }

    private static void invoke(Method method, Method asInterface, Capture capture) throws Exception {
        // Capture attaches no local interface: the original Stub must construct its Proxy.
        Object proxy = asInterface.invoke(null, capture);
        require(proxy.getClass().getName().equals(DESCRIPTOR + "$Stub$Proxy"), "original proxy selected");
        int[] fields = capture.expected;
        method.invoke(proxy, fields[0], fields[1], fields[2], fields[3], capture.payload);
    }

    public static void main(String[] args) throws Exception {
        Class<?> type = Class.forName(DESCRIPTOR);
        Method method = type.getMethod("sendRequestRaw", int.class, int.class, int.class, int.class, byte[].class);
        Method asInterface = Class.forName(DESCRIPTOR + "$Stub").getMethod("asInterface", IBinder.class);
        byte[] sequence = new byte[257];
        for (int i = 0; i < sequence.length; i++) sequence[i] = (byte) i;
        byte[][] payloads = {null, new byte[0], new byte[]{0}, new byte[]{-128, -1, 0, 127}, sequence};
        int[][] fields = {
            {0, 1, 2, 3}, {-1, -2, -3, -4},
            {Integer.MIN_VALUE, Integer.MAX_VALUE, 128, 255},
            {256, 65535, 65536, -128}, {137, 42, 1, 257}
        };
        for (int i = 0; i < payloads.length; i++) {
            Capture capture = new Capture(fields[i], payloads[i], true);
            invoke(method, asInterface, capture);
            require(capture.calls == 1, "one transaction per request");
            System.out.println("binderRaw\t" + i + "\tPASS");
        }
        Capture rejected = new Capture(fields[0], payloads[0], false);
        try {
            invoke(method, asInterface, rejected);
            throw new AssertionError("rejected transaction must throw");
        } catch (InvocationTargetException exception) {
            require(exception.getCause() instanceof RemoteException, "rejection is RemoteException");
            require(rejected.calls == 1, "rejected request is not retried");
        }
        System.out.println("binderRejected\tPASS");
        System.exit(0);
    }
}
