package org.omarchy.audit;

import java.lang.reflect.Constructor;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.Arrays;

/** Executes the original accumulator only. No radio service or IMS startup. */
public final class IndicationAssemblyOracle {
    private static Constructor<?> constructor;
    private static Method add, data;
    private static void require(boolean ok, String message) {
        if (!ok) throw new AssertionError(message);
    }
    private static Object message(int total) throws Exception {
        // The original constructor stores IDs/length and creates its segment map.
        // Its enclosing instance is unused by the exercised accumulator methods.
        return constructor.newInstance(null, 33, 0, total);
    }
    private static ArrayList<Byte> bytes(int... values) {
        ArrayList<Byte> result = new ArrayList<>();
        for (int value : values) result.add((byte) value);
        return result;
    }
    private static void part(Object message, int index, int remaining, int... values) throws Exception {
        require(((Integer) add.invoke(message, index, values.length, bytes(values))) == remaining, "remaining bytes");
    }
    private static void output(Object message, int... values) throws Exception {
        require(data.invoke(message).equals(bytes(values)), "assembled bytes " + Arrays.toString(values));
    }
    private static void gap(Object message) throws Exception {
        try { data.invoke(message); throw new AssertionError("expected original missing-index failure"); }
        catch (InvocationTargetException error) {
            require(error.getCause() instanceof NullPointerException, "original gap exception");
        }
    }
    private static void pass(String name) { System.out.println("indicationAssembly\t" + name + "\tPASS"); }
    public static void main(String[] args) throws Exception {
        Class<?> outer = Class.forName("com.shannon.imsservice.network.oem.hal.OemSlsiRadioExternalIndBase");
        Class<?> type = Class.forName(outer.getName() + "$SegmentRilMessage");
        constructor = type.getDeclaredConstructor(outer, int.class, int.class, int.class);
        constructor.setAccessible(true);
        add = type.getDeclaredMethod("addDataSegment", int.class, int.class, ArrayList.class);
        data = type.getDeclaredMethod("getMessageData");
        add.setAccessible(true); data.setAccessible(true);
        Object m = message(5);
        part(m, 0, 3, 1, 2); part(m, 1, 0, 3, 4, 5); output(m, 1, 2, 3, 4, 5); pass("ordered");
        m = message(5);
        part(m, 1, 2, 3, 4, 5); part(m, 0, 0, 1, 2); output(m, 1, 2, 3, 4, 5); pass("reversed");
        m = message(3);
        part(m, 0, 2, 1); part(m, 0, 2, 1); part(m, 1, 0, 2, 3); output(m, 1, 2, 3); pass("duplicate");
        m = message(3);
        part(m, 0, 2, 1); part(m, 0, 2, 9, 8); part(m, 1, 0, 2, 3); output(m, 1, 2, 3); pass("conflicting-duplicate");
        m = message(2);
        part(m, 0, 1, 1); part(m, 2, 0, 2); gap(m); pass("gap");
        m = message(2);
        part(m, -1, 1, 1); part(m, 0, 0, 2); gap(m); pass("negative-index");
        m = message(1);
        part(m, 0, -1, 1, 2); output(m, 1, 2); pass("overshoot");
        m = message(0);
        part(m, 0, 0); output(m); pass("empty");
        System.exit(0);
    }
}
