package org.omarchy.audit;

import android.os.Binder;
import android.os.IBinder;
import android.os.Parcel;
import android.os.RemoteException;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;

/** Original command proxy execution against synthetic local endpoints only. */
public final class RegistrationOracle {
    private static final String NAME =
        "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternal";
    private static void require(boolean ok, String label) {
        if (!ok) throw new AssertionError(label);
    }
    private static Object callback(Class<?> type, IBinder binder) {
        return Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[]{type},
            (proxy, method, args) -> {
                if (method.getName().equals("asBinder")) return binder;
                throw new AssertionError("unexpected callback invocation: " + method.getName());
            });
    }
    private static final class Capture extends Binder {
        final int mode;
        final IBinder response = new Binder();
        final IBinder indication = new Binder();
        int calls;
        boolean accept = true;
        Capture(int mode) { this.mode = mode; }
        @Override protected boolean onTransact(int code, Parcel data, Parcel reply, int flags) {
            calls++;
            data.enforceInterface(NAME);
            if (mode == 4) {
                require(code == 1 && flags == IBinder.FLAG_ONEWAY && reply == null, "clear framing");
                require(data.readInt() == Integer.MIN_VALUE, "clear client ID");
                require(data.dataAvail() == 0, "clear trailing data");
                return accept;
            }
            require(code == 5 && flags == 0 && reply != null, "registration framing");
            require(data.readStrongBinder() == (mode == 1 ? null : response), "response handle");
            require(data.readStrongBinder() == (mode == 1 ? null : indication), "indication handle");
            String client = data.readString();
            require(mode == 1 ? client == null : "pokelinux-test".equals(client), "client name");
            require(data.dataAvail() == 0, "registration trailing data");
            if (mode == 3) return false;
            if (mode == 2) reply.writeException(new SecurityException("synthetic denial"));
            else {
                reply.writeNoException();
                reply.writeInt(mode == 1 ? -1 : 42);
            }
            return true;
        }
    }
    private static void rejects(Method method, Object proxy, Class<?> error, Object... args) throws Exception {
        try {
            method.invoke(proxy, args);
            throw new AssertionError("failure was swallowed");
        } catch (InvocationTargetException failure) {
            require(error.isInstance(failure.getCause()), "exception type: " + failure.getCause());
        }
    }
    public static void main(String[] args) throws Exception {
        Class<?> type = Class.forName(NAME);
        Class<?> res = Class.forName(NAME + "Res");
        Class<?> ind = Class.forName(NAME + "Ind");
        Method register = type.getMethod("setResponseFunctions", res, ind, String.class);
        Method clear = type.getMethod("clearResponseFunctions", int.class);
        Method asInterface = Class.forName(NAME + "$Stub").getMethod("asInterface", IBinder.class);
        for (int mode = 0; mode < 5; mode++) {
            Capture capture = new Capture(mode);
            Object proxy = asInterface.invoke(null, capture);
            require(proxy.getClass().getName().equals(NAME + "$Stub$Proxy"), "original proxy");
            Object[] fields = mode == 1 ? new Object[]{null, null, null} :
                new Object[]{callback(res, capture.response), callback(ind, capture.indication), "pokelinux-test"};
            if (mode < 2) {
                require((Integer) register.invoke(proxy, fields) == (mode == 1 ? -1 : 42), "returned ID");
            } else if (mode < 4) {
                rejects(register, proxy, mode == 2 ? SecurityException.class : RemoteException.class, fields);
            } else {
                clear.invoke(proxy, Integer.MIN_VALUE);
                capture.accept = false;
                rejects(clear, proxy, RemoteException.class, Integer.MIN_VALUE);
            }
            require(capture.calls == (mode == 4 ? 2 : 1), "no automatic retries");
            System.out.println("registration\t" + mode + "\tPASS");
        }
        System.exit(0);
    }
}
