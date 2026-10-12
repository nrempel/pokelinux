package org.omarchy.audit;

import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.concurrent.ConcurrentHashMap;
import java.util.Map;

/** Original APK codecs and controlled local state only. Never discovers a radio service. */
public final class SmsTransportOracle {
    private static final String DATA = "com.shannon.imsservice.network.adaptor.data.";
    private static final String SMS = "com.shannon.imsservice.sms.";
    private static void require(boolean ok, String label) {
        if (!ok) throw new AssertionError(label);
    }
    private static Method method(Class<?> type, String name, Class<?>... args) throws Exception {
        Method m = type.getDeclaredMethod(name, args); m.setAccessible(true); return m;
    }
    private static void set(Object object, String name, Object value) throws Exception {
        Field f = object.getClass().getDeclaredField(name); f.setAccessible(true); f.set(object, value);
    }
    private static Object get(Object object, String name) throws Exception {
        Field f = object.getClass().getDeclaredField(name); f.setAccessible(true); return f.get(object);
    }
    private static Object request(Class<?> type) throws Exception {
        Class<?> channel = Class.forName("com.shannon.imsservice.network.NetworkAdaptor$AdaptorChannel");
        Constructor<?> c = type.getDeclaredConstructor(channel); c.setAccessible(true);
        return c.newInstance(new Object[]{null});
    }
    private static Object allocate(Class<?> type) throws Exception {
        // Bypass constructors that attach listeners or consult services; this does not test startup.
        Class<?> unsafe = Class.forName("sun.misc.Unsafe");
        for (Field f : unsafe.getDeclaredFields()) {
            if (Modifier.isStatic(f.getModifiers()) && f.getType() == unsafe) {
                f.setAccessible(true);
                return unsafe.getMethod("allocateInstance", Class.class).invoke(f.get(null), type);
            }
        }
        throw new AssertionError("test allocator unavailable");
    }
    private static byte[] pattern(int size, int seed) {
        byte[] b = new byte[size]; for (int i = 0; i < size; i++) b[i] = (byte)(seed + i * 37); return b;
    }
    private static String hex(byte[] b) {
        StringBuilder s = new StringBuilder();
        for (byte v : b) s.append(String.format(java.util.Locale.ROOT, "%02x", v & 255));
        return s.toString();
    }
    private static void putInt(byte[] b, int offset, int value) {
        for (int i = 0; i < 4; i++) b[offset + i] = (byte)(value >>> (i * 8));
    }
    private static void submit() throws Exception {
        Class<?> type = Class.forName(DATA + "RilReqSendSms");
        int caseId = 0;
        for (int scaLength : new int[]{0, 1, 12, 13}) {
            for (int tpduLength : new int[]{0, 1, 140, 244, 245}) {
                Object request = request(type);
                byte[] sca = pattern(scaLength, 0x81), tpdu = pattern(tpduLength, 0x80);
                type.getMethod("setSmscLen", int.class).invoke(request, scaLength);
                type.getMethod("setSmsc", byte[].class).invoke(request, (Object)sca);
                type.getMethod("setSmsLen", int.class).invoke(request, tpduLength);
                type.getMethod("setSmsData", byte[].class).invoke(request, (Object)tpdu);
                type.getMethod("setSmsType", int.class).invoke(request, 0x102);
                type.getMethod("setGeolocationSupport", int.class).invoke(request, -1);
                type.getMethod("setToken", int.class).invoke(request, 42);
                byte[] encoded = (byte[])type.getMethod("getBytes").invoke(request);
                require(encoded.length == 260, "fixed submit payload length");
                require((encoded[0] & 255) == Math.min(scaLength, 12), "SMSC cap");
                require((encoded[13] & 255) == Math.min(tpduLength, 244), "TPDU cap");
                for (int i = 0; i < 12; i++) require(encoded[1 + i] == (i < scaLength ? sca[i] : 0), "SMSC content/padding");
                for (int i = 0; i < 244; i++) require(encoded[14 + i] == (i < tpduLength ? tpdu[i] : 0), "TPDU content/padding");
                require((encoded[258] & 255) == 2 && (encoded[259] & 255) == 255, "scalar low bytes");
                type.getMethod("setToken", int.class).invoke(request, -99);
                require(Arrays.equals(encoded, (byte[])type.getMethod("getBytes").invoke(request)), "token excluded from payload");
                System.out.println("sms-transport\tsubmit\t" + caseId++ + "\t" + scaLength + "\t" + tpduLength + "\t" + hex(encoded));
            }
        }
    }
    private static void ack() throws Exception {
        Class<?> type = Class.forName(DATA + "RilReqSendSmsAck");
        int[] values = {Integer.MIN_VALUE, -1, 0, 1, Integer.MAX_VALUE};
        for (int i = 0; i < values.length; i++) {
            Object request = request(type);
            int tpid = new int[]{0, 255, 256, -1, -128}[i], cause = values[(i + 2) % values.length];
            type.getMethod("setResult", int.class).invoke(request, values[i]);
            type.getMethod("setMsgTpid", int.class).invoke(request, tpid);
            type.getMethod("setErrorCode", int.class).invoke(request, cause);
            byte[] encoded = (byte[])type.getMethod("getBytes").invoke(request);
            byte[] expected = new byte[9]; putInt(expected, 0, values[i]); expected[4] = (byte)tpid; putInt(expected, 5, cause);
            require(Arrays.equals(encoded, expected), "ack int/byte/int encoding");
            System.out.println("sms-transport\tack\t" + values[i] + "\t" + tpid + "\t" + cause + "\t" + hex(encoded));
        }
    }
    private static void parseResponse() throws Exception {
        Class<?> type = Class.forName(DATA + "RilReqSendSms");
        Method update = method(type, "update", byte[].class, int.class);
        for (int length : new int[]{0, 1, 127, 128, 244}) {
            Object request = request(type);
            byte[] input = new byte[253], ack = pattern(244, 0x81);
            putInt(input, 0, 0x1020304); input[4] = (byte)length;
            System.arraycopy(ack, 0, input, 5, ack.length); putInt(input, 249, -1);
            update.invoke(request, input, input.length);
            require(((Integer)get(request, "mMsgRef")) == 0x1020304, "response reference");
            require(Arrays.equals((byte[])get(request, "mAckPdu"), ack), "response fixed ack area");
            require(((Integer)get(request, "mErrorCode")) == -1, "response error");
            System.out.println("sms-transport\tresponse\t" + length + "\t" + get(request, "mAckPduLen") + "\tPASS");
        }
    }
    private static void parseIncoming() throws Exception {
        for (String name : new String[]{"RilIndNewSms", "RilIndNewSmsStatusReport"}) {
            Class<?> type = Class.forName(DATA + name);
            Method update = method(type, "update", byte[].class, int.class);
            for (int declared : new int[]{0, 1, 244, 245}) {
                for (boolean optional : new boolean[]{false, true}) {
                    Object indication = request(type);
                    byte[] input = new byte[optional ? 247 : 246];
                    input[0] = (byte)0x81; input[1] = (byte)declared;
                    byte[] area = pattern(244, 0x80); System.arraycopy(area, 0, input, 2, area.length);
                    if (optional) input[246] = 2;
                    update.invoke(indication, input, input.length);
                    require(((Integer)type.getMethod("getMsgTpid").invoke(indication)) == 129, "unsigned TPID");
                    int length = (Integer)type.getMethod("getTpduLength").invoke(indication);
                    require(length == Math.min(declared, 244), "incoming length cap");
                    require(Arrays.equals((byte[])type.getMethod("getTpdu").invoke(indication), Arrays.copyOf(area, length)), "incoming content");
                    System.out.println("sms-transport\tincoming\t" + name + "\t" + declared + "\t" + optional + "\t" + type.getMethod("getVerstat").invoke(indication) + "\tPASS");
                }
            }
        }
    }
    private static void localTrackers() throws Exception {
        Class<?> type = Class.forName(SMS + "ImsSms");
        Object owner = allocate(type); Map<Integer,Object> trackers = new ConcurrentHashMap<>();
        set(owner, "mTrackers", trackers); set(owner, "mSlotId", 0); set(owner, "mDiscardAck", -1);
        Class<?> localLog = Class.forName("android.util.LocalLog");
        set(owner, "mSmsHistoryLogger", localLog.getConstructor(int.class).newInstance(16));
        Method add = method(type, "addTracker", int.class), lookup = method(type, "getTracker", int.class);
        Object first = add.invoke(owner, 42), second = add.invoke(owner, 42);
        require(first != second && trackers.size() == 1 && lookup.invoke(owner, 42) == second, "same-token replacement");
        System.out.println("sms-transport\ttracker\ttoken-replacement\tPASS");
        Class<?> trackerType = Class.forName(SMS + "ImsSmsTracker");
        trackerType.getMethod("setMr", int.class).invoke(second, 7);
        Object third = add.invoke(owner, 43); trackerType.getMethod("setMr", int.class).invoke(third, 7);
        Object matched = method(type, "getTrackerByMr", int.class).invoke(owner, 7);
        require(matched == second || matched == third, "MR lookup returns one ambiguous match");
        System.out.println("sms-transport\ttracker\tmessage-reference-collision\tPASS");
        trackers.clear(); Object acknowledged = add.invoke(owner, 44);
        trackerType.getMethod("setTpId", int.class).invoke(acknowledged, 129);
        Method acknowledge = method(type, "acknowledgeGsmSmsReport", int.class, int.class, int.class);
        // The adaptor remains null. This is an intentional stop before any vendor dispatch.
        try {
            acknowledge.invoke(owner, 44, 7, 1); throw new AssertionError("null dispatch collaborator must stop the attempt");
        } catch (InvocationTargetException error) {
            require(error.getCause() instanceof NullPointerException, "controlled adaptor stop");
        }
        require(trackers.isEmpty(), "tracker removed before failed acknowledgement dispatch");
        acknowledge.invoke(owner, 44, 7, 1);
        require(trackers.isEmpty(), "duplicate report acknowledgement leaves no tracker");
        System.out.println("sms-transport\ttracker\tremove-before-failed-ack-no-local-retry\tPASS");
        Object discarded = add.invoke(owner, 45); trackerType.getMethod("setTpId", int.class).invoke(discarded, 130);
        set(owner, "mDiscardAck", 45);
        method(type, "acknowledgeGsmSms", int.class, int.class, int.class).invoke(owner, 45, 0, 1);
        require(trackers.isEmpty(), "discard-ack removes tracker and avoids null adaptor");
        System.out.println("sms-transport\ttracker\tdiscard-ack\tPASS");
    }
    private static void enums() throws Exception {
        Class<?> type = Class.forName("com.shannon.imsservice.network.oem.helper.ImsRilHelper$RilMessageId");
        for (Object value : (Object[])type.getMethod("values").invoke(null)) {
            String name = ((Enum<?>)value).name();
            if (name.contains("SMS")) System.out.println("sms-transport\tmessage-id\t" + name + "\t" + type.getMethod("getCode").invoke(value));
        }
    }
    public static void main(String[] args) throws Exception {
        submit(); ack(); parseResponse(); parseIncoming(); localTrackers(); enums();
        System.out.println("sms-transport\tcomplete\tPASS");
        System.exit(0);
    }
}
