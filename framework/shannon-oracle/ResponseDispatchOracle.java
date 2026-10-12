package org.omarchy.audit;

import android.os.Parcel;
import android.os.Parcelable;
import android.util.SparseArray;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;

/** Executes the original response receiver with a controlled pending-request map. */
public final class ResponseDispatchOracle {
    private static final String OEM = "com.shannon.imsservice.network.oem.";
    private static final String VENDOR = "vendor.samsung_slsi.telephony.hardware.radioExternal.";
    private static void require(boolean value, String message) {
        if (!value) throw new AssertionError(message);
    }
    private static void set(Object target, String name, Object value) throws Exception {
        Field f = target.getClass().getDeclaredField(name);
        f.setAccessible(true);
        f.set(target, value);
    }
    private static Object allocate(Class<?> type) throws Exception {
        Class<?> unsafe = Class.forName("sun.misc.Unsafe");
        for (Field f : unsafe.getDeclaredFields()) {
            if (Modifier.isStatic(f.getModifiers()) && f.getType() == unsafe) {
                f.setAccessible(true);
                return unsafe.getMethod("allocateInstance", Class.class).invoke(f.get(null), type);
            }
        }
        throw new AssertionError("test allocator unavailable");
    }
    public static void main(String[] args) throws Exception {
        Class<?> rilType = Class.forName(OEM + "OemRIL");
        Class<?> requestType = Class.forName(OEM + "OemRIL$RILRequest");
        Class<?> infoType = Class.forName(VENDOR + "RadioExternalResponseInfo");
        Class<?> stubType = Class.forName(OEM + "hal.aidl.OemSlsiRadioExternalResAidl$OemSlsiRadioExternalResStub");
        Class<?> statusType = Class.forName(OEM + "OemRILConstants$Status");
        Object success = statusType.getField("STATUS_SUCCESS").get(null);
        int status = (Integer) statusType.getMethod("getCode").invoke(success);
        Constructor<?> constructor = stubType.getDeclaredConstructor(rilType);
        constructor.setAccessible(true);
        Method dispatch = Class.forName(VENDOR + "IOemSlsiRadioExternalRes$Stub")
            .getMethod("onTransact", int.class, Parcel.class, Parcel.class, int.class);
        for (int test = 0; test < 4; test++) {
            Object ril = allocate(rilType);
            Object request = allocate(requestType);
            set(request, "mSerial", 42);
            set(request, "mRequest", 137);
            SparseArray<Object> pending = new SparseArray<>();
            pending.put(42, request);
            set(ril, "mRequestList", pending);
            Object stub = constructor.newInstance(ril);
            Parcelable info = (Parcelable) infoType.getConstructor().newInstance();
            set(info, "serial", test == 1 ? 99 : 42);
            set(info, "error", status);
            // Deliberately different from local routing and actual payload length.
            set(info, "slotId", 99);
            set(info, "rilcMsgId", 999);
            set(info, "length", 100);
            Parcel data = Parcel.obtain();
            try {
                data.writeInterfaceToken(VENDOR + "IOemSlsiRadioExternalRes");
                int argumentsStart = data.dataPosition();
                data.writeTypedObject(info, 0);
                data.writeByteArray(new byte[]{1, 2, 3});
                if (test == 2) { data.writeInt(0); data.writeInt(3); }
                if (test == 3) data.writeInt(123);
                byte[] encoded = data.marshall();
                StringBuilder hex = new StringBuilder();
                for (int offset = argumentsStart; offset < encoded.length; offset++) {
                    hex.append(String.format(java.util.Locale.ROOT, "%02x", encoded[offset] & 255));
                }
                System.out.println("responseArgs\t" + test + "\t" + (test == 2 ? 2 : 1) + "\t" + hex);
                data.setDataPosition(0);
                boolean rejected = false;
                try {
                    require(Boolean.TRUE.equals(dispatch.invoke(stub, test == 2 ? 2 : 1, data, null, 1)), "handled transaction");
                } catch (InvocationTargetException error) {
                    if (!(error.getCause() instanceof android.os.BadParcelableException)) throw error;
                    rejected = true;
                }
                require(rejected == (test == 3), "trailing fields rejected before dispatch");
                require(pending.get(42) == (test == 0 ? null : request), "request completion/retention");
                System.out.println("responseDispatch\t" + test + "\tPASS");
            } finally {
                data.recycle();
            }
        }
        System.exit(0);
    }
}
