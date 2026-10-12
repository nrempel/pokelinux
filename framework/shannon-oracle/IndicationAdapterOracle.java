package org.omarchy.audit;

import android.os.Parcel;
import java.lang.reflect.*;
import java.util.*;

/** Original callback dispatch and owner; synthetic listener, no service lookup. */
public final class IndicationAdapterOracle {
    private static final String HAL = "com.shannon.imsservice.network.oem.hal.";
    private static final String DESCRIPTOR = "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternalInd";
    private static void require(boolean ok, String message) { if (!ok) throw new AssertionError(message); }
    private static final class Owner {
        final Object adapter, stub;
        final Map<?,?> pending;
        final List<Object> results = new ArrayList<>();
        boolean reject;
        Owner() throws Exception {
            Class<?> listener = Class.forName(HAL + "IOemSlsiRadioExternalIndListener");
            Object capture = Proxy.newProxyInstance(listener.getClassLoader(), new Class<?>[]{listener}, (p, method, args) -> {
                require(method.getName().equals("processUnsolicited"), "listener method");
                results.add(args[0]);
                if (reject) throw new IllegalStateException("injected listener failure");
                return null;
            });
            Class<?> type = Class.forName(HAL + "aidl.OemSlsiRadioExternalIndAidl");
            adapter = type.getConstructor(int.class, listener).newInstance(0, capture);
            stub = type.getMethod("getStub").invoke(adapter);
            pending = (Map<?,?>) type.getMethod("getReceivingSegmentRilMessageMap").invoke(adapter);
        }
        void part(int id, int slot, int index, int total, int... values) throws Exception {
            byte[] bytes = new byte[values.length];
            for (int i=0;i<values.length;i++) bytes[i]=(byte)values[i];
            Parcel input = Parcel.obtain();
            try {
                input.writeInterfaceToken(DESCRIPTOR);
                input.writeInt(id); input.writeInt(slot); input.writeByteArray(bytes);
                input.writeInt(index); input.writeInt(total); input.setDataPosition(0);
                Method dispatch = Class.forName(DESCRIPTOR + "$Stub").getMethod("onTransact", int.class, Parcel.class, Parcel.class, int.class);
                require(Boolean.TRUE.equals(dispatch.invoke(stub, 2, input, null, 1)), "handled callback");
            } finally { input.recycle(); }
        }
        void result(int n, int id, int... values) throws Exception {
            Object result = results.get(n); Class<?> type = result.getClass();
            require(((Integer)type.getMethod("getId").invoke(result)) == id, "message ID");
            require(((Integer)type.getMethod("getChannel").invoke(result)) == 0, "slot");
            require(((Integer)type.getMethod("getSerial").invoke(result)) == -1, "unsolicited serial");
            byte[] expected = new byte[values.length];
            for(int i=0;i<values.length;i++) expected[i]=(byte)values[i];
            require(Arrays.equals((byte[])type.getMethod("getData").invoke(result), expected), "result bytes");
        }
    }
    private static void pass(String name) { System.out.println("indicationAdapter\t"+name+"\tPASS"); }
    public static void main(String[] args) throws Exception {
        Owner o = new Owner();
        o.part(33,1,0,1,9); require(o.pending.isEmpty() && o.results.isEmpty(),"wrong slot ignored"); pass("slot-filter");
        o.part(33,0,0,2,1); o.part(33,0,1,2,2); o.result(0,33,1,2);
        require(o.pending.isEmpty(),"removed after delivery");
        o.part(33,0,0,1,3); o.result(1,33,3);
        require(o.results.size()==2 && o.pending.isEmpty(),"reuse after completion"); pass("reuse");
        o = new Owner();
        o.part(33,0,0,2,1); o.part(34,0,0,2,3);
        o.part(34,0,1,2,4); o.part(33,0,1,2,2);
        o.result(0,34,3,4); o.result(1,33,1,2);
        require(o.results.size()==2 && o.pending.isEmpty(),"IDs isolated"); pass("separate-ids");
        o = new Owner();
        // First fragment from conceptual message A, tail from conceptual message B.
        // The wire contains no discriminator for those conceptual identities.
        o.part(33,0,0,4,1,2); o.part(33,0,1,4,8,9); o.result(0,33,1,2,8,9);
        require(o.results.size()==1 && o.pending.isEmpty(),"mixed bytes emitted"); pass("mixed-instance");
        o = new Owner();
        o.part(33,0,0,3,1); o.part(33,0,1,5,2,3); o.result(0,33,1,2,3,0,0);
        o.part(33,0,0,3,1); o.part(33,0,1,1,2,3); o.result(1,33,1);
        require(o.results.size()==2 && o.pending.isEmpty(),"first total completes, last total sizes result"); pass("total-change");
        o = new Owner(); o.reject = true;
        try { o.part(33,0,0,1,7); throw new AssertionError("listener should throw"); }
        catch(InvocationTargetException error) { require(error.getCause() instanceof IllegalStateException,"listener error propagated"); }
        require(o.pending.size()==1 && o.results.size()==1,"entry retained after failed listener");
        o.reject=false; o.part(33,0,0,1,9); o.result(1,33,7);
        require(o.pending.isEmpty() && o.results.size()==2,"duplicate retries retained first bytes"); pass("listener-failure");
        o = new Owner(); o.part(33,0,0,2,1);
        try { o.part(33,0,2,2,2); throw new AssertionError("gap should throw"); }
        catch(InvocationTargetException error) { require(error.getCause() instanceof NullPointerException,"gap error propagated"); }
        require(o.pending.size()==1 && o.results.isEmpty(),"gap retains entry");
        o.part(33,0,1,2,3); require(o.pending.size()==1 && o.results.isEmpty(),"late gap fill overshoots"); pass("gap-retained");
        System.exit(0);
    }
}
