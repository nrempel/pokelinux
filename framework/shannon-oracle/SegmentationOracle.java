package org.omarchy.audit;

import android.os.Binder;
import android.os.IBinder;
import android.os.Parcel;
import android.util.SparseArray;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;

/** Original method execution with explicitly injected collaborators; no service lookup. */
public final class SegmentationOracle {
    private static final String OEM = "com.shannon.imsservice.network.oem.";
    private static final String DESCRIPTOR =
        "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternal";

    private static void require(boolean value, String message) {
        if (!value) throw new AssertionError(message);
    }

    private static void set(Object target, String name, Object value) throws Exception {
        Field field = target.getClass().getDeclaredField(name);
        field.setAccessible(true);
        field.set(target, value);
    }

    private static Object allocate(Class<?> type) throws Exception {
        // Test-only allocation bypasses constructors that connect to real Android services.
        // All fields used by the exercised path are injected below. This does not test startup.
        Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
        for (Field field : unsafeType.getDeclaredFields()) {
            if (Modifier.isStatic(field.getModifiers()) && field.getType() == unsafeType) {
                field.setAccessible(true);
                Object unsafe = field.get(null);
                return unsafeType.getMethod("allocateInstance", Class.class).invoke(unsafe, type);
            }
        }
        throw new AssertionError("Test allocator unavailable");
    }

    private static final class Capture extends Binder {
        final byte[] input;
        final StringBuilder lengths = new StringBuilder();
        int calls;
        int bytes;
        Capture(byte[] input) { this.input = input; }
        @Override protected boolean onTransact(int code, Parcel data, Parcel reply, int flags) {
            require(code == 3 && flags == IBinder.FLAG_ONEWAY && reply == null, "segment transaction");
            data.enforceInterface(DESCRIPTOR);
            require(data.readInt() == 42, "serial");
            require(data.readInt() == 17, "client");
            require(data.readInt() == 137, "request");
            require(data.readInt() == 0, "instance");
            byte[] chunk = data.createByteArray();
            require(chunk != null, "segment payload");
            require(data.readInt() == calls, "segment index");
            require(data.readInt() == input.length, "total length");
            require(data.dataAvail() == 0, "no trailing data");
            require(bytes + chunk.length <= input.length, "bounded content");
            for (int i = 0; i < chunk.length; i++) require(chunk[i] == input[bytes + i], "original payload bytes");
            if (calls != 0) lengths.append(',');
            lengths.append(chunk.length);
            bytes += chunk.length;
            calls++;
            return true;
        }
    }

    public static void main(String[] args) throws Exception {
        Class<?> rilType = Class.forName(OEM + "OemRIL");
        Class<?> requestType = Class.forName(OEM + "OemRIL$RILRequest");
        Class<?> adapterType = Class.forName(OEM + "hal.aidl.OemSlsiRadioExternalProxyAidl");
        Class<?> writerType = Class.forName(OEM + "io.DataWriter");
        Method send = rilType.getDeclaredMethod("sendMultiFrame", requestType);
        send.setAccessible(true);
        Method asInterface = Class.forName(DESCRIPTOR + "$Stub").getMethod("asInterface", IBinder.class);
        for (int size : new int[]{2049, 4095, 4096, 4097, 6144}) {
            byte[] input = new byte[size];
            for (int i = 0; i < size; i++) input[i] = (byte) ((i * 31 + i / 2048) & 255);
            Capture capture = new Capture(input);
            Object proxy = asInterface.invoke(null, capture);
            require(proxy.getClass().getName().equals(DESCRIPTOR + "$Stub$Proxy"), "original generated proxy");
            Object adapter = allocate(adapterType);
            set(adapter, "mAidlIntf", proxy);
            Object ril = allocate(rilType);
            SparseArray<Object> requests = new SparseArray<>();
            set(ril, "mRequestList", requests);
            set(ril, "mOemSlsiRadioExternalProxy", adapter);
            set(ril, "mClientId", 17);
            set(ril, "mInstanceId", 0);
            Object request = allocate(requestType);
            Object writer = writerType.getConstructor().newInstance();
            writerType.getMethod("writeBytes", byte[].class).invoke(writer, (Object) input);
            set(request, "mOut", writer);
            set(request, "mSerial", 42);
            set(request, "mRequest", 137);
            send.invoke(ril, request);
            require(requests.get(42) == request, "request retained for response");
            require(capture.calls > 0, "original method reached capture");
            System.out.println("segments\t" + size + "\t" + capture.lengths + "\t" + capture.bytes);
        }
        System.exit(0);
    }
}
