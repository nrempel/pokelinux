package org.omarchy.audit;

import android.content.ComponentName;
import android.os.Binder;
import android.os.Bundle;
import android.os.IBinder;
import android.os.Parcel;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;

/** Captures the installed framework's own proxies; never starts instrumentation. */
public final class FrameworkBootstrapOracle {
    private static void require(boolean ok, String reason) {
        if (!ok) throw new AssertionError(reason);
    }
    public static void main(String[] args) throws Exception {
        String name = "android.app.IActivityManager";
        Class<?> watcherType = Class.forName("android.app.IInstrumentationWatcher");
        Binder watcher = new Binder();
        Object watcherInterface = Proxy.newProxyInstance(watcherType.getClassLoader(),
            new Class<?>[]{watcherType}, (p, m, a) -> {
                if (m.getName().equals("asBinder")) return watcher;
                throw new AssertionError(m.getName());
            });
        Binder capture = new Binder() {
            @Override protected boolean onTransact(int code, Parcel data, Parcel reply, int flags) {
                data.enforceInterface(name);
                require(flags == 0 && reply != null, "synchronous start");
                ComponentName c = data.readTypedObject(ComponentName.CREATOR);
                require(c.flattenToString().equals("org.pokelinux.radiopeer/org.pokelinux.radiopeer.Peer"), "component");
                require(data.readString() == null, "profile");
                require(data.readInt() == 0, "flags");
                require(data.readTypedObject(Bundle.CREATOR) == null, "arguments");
                require(data.readStrongBinder() == watcher, "watcher");
                require(data.readStrongBinder() == null, "automation");
                require(data.readInt() == 0, "user");
                require(data.readString() == null && data.dataAvail() == 0, "ABI/trailing fields");
                reply.writeNoException(); reply.writeBoolean(true);
                System.out.println("startInstrumentation\t" + code + "\tPASS");
                return true;
            }
        };
        Object manager = Class.forName(name + "$Stub").getMethod("asInterface", IBinder.class).invoke(null, capture);
        Method start = Class.forName(name).getMethod("startInstrumentation", ComponentName.class,
            String.class, int.class, Bundle.class, watcherType,
            Class.forName("android.app.IUiAutomationConnection"), int.class, String.class);
        require((Boolean) start.invoke(manager, new ComponentName("org.pokelinux.radiopeer", "org.pokelinux.radiopeer.Peer"),
            null, 0, null, watcherInterface, null, 0, null), "start result");
        Binder peer = new Binder();
        Binder statusCapture = new Binder() {
            @Override protected boolean onTransact(int code, Parcel data, Parcel reply, int flags) {
                data.enforceInterface("android.app.IInstrumentationWatcher");
                require(code == 1 && flags == 0 && reply != null, "status framing");
                require(data.readInt() == 1, "component presence");
                require("org.pokelinux.radiopeer".equals(data.readString()), "package");
                require("org.pokelinux.radiopeer.Peer".equals(data.readString()), "class");
                require(data.readInt() == 37 && data.readInt() == 1, "result and bundle presence");
                int length = data.readInt();
                require(length > 0 && data.readInt() == 0x4c444e42, "bundle size/magic");
                int begin = data.dataPosition();
                require(data.readInt() == 1 && "peer".equals(data.readString()), "single peer key");
                require(data.readInt() == 15 && data.readStrongBinder() == peer, "binder value");
                require(data.dataPosition() - begin == length, "bundle bounds");
                require(data.readInt() == 0, "SDK37 status trailer");
                require(data.dataAvail() == 0, "status trailing fields");
                reply.writeNoException();
                System.out.println("instrumentationStatus\t1\tPASS");
                return true;
            }
        };
        Object status = Class.forName("android.app.IInstrumentationWatcher$Stub")
            .getMethod("asInterface", IBinder.class).invoke(null, statusCapture);
        Bundle bundle = new Bundle(); bundle.putBinder("peer", peer);
        watcherType.getMethod("instrumentationStatus", ComponentName.class, int.class, Bundle.class)
            .invoke(status, new ComponentName("org.pokelinux.radiopeer", "org.pokelinux.radiopeer.Peer"), 37, bundle);
        System.exit(0);
    }
}
