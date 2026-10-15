package org.omarchy.audit;

import android.os.HandlerThread;
import android.os.Looper;
import android.os.SystemClock;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.Calendar;
import java.util.GregorianCalendar;
import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;
import java.util.TimeZone;
import org.json.JSONArray;
import org.json.JSONObject;

/** Authored original add-PDN codec probe. No APN acquisition or request dispatch. */
public final class RegistrationAddPdnCodecOracle {
    private static final String NET = "com.shannon.imsservice.network.";
    private static final String CHANNEL = NET + "NetworkAdaptor$AdaptorChannel";
    private static final String ADAPTOR = NET + "adaptor.RegistrationAdaptor";
    private static final String ENTITY = "com.shannon.imsservice.conn.ApnEntity";
    private static final String REQUEST = NET + "adaptor.data.RilReqAddPdnInfo";
    private static final String SOLICITED = NET + "adaptor.SolicitedMessage";
    private static final String MESSAGE_ID = NET + "oem.helper.ImsRilHelper$RilMessageId";
    private static final String WRITER = NET + "oem.io.DataWriter";
    private static final String APK_SHA = "8b017599727398e80df5ab618d648890dcdc34b5fe2ba71fb2331e51bd8b8879";
    private static final Thread DRIVER = Thread.currentThread();
    private static final int CASES = 7, VECTOR_BOUND = 4096;
    private static final int RECORD_BOUND = 10, RECORD_BYTES = 32768, TOTAL_BYTES = 262144;
    private static long startedAt, caseUntil, cleanupUntil;
    private static int recordCount, outputBytes, caseCount, vectorCount;
    private static ClassLoader loader;
    private static Object unsafe, channel;
    private static Method allocate, getBytes, getMessageId, getCode, getImsNetType, getPdnNew, getTimestamp;
    private static Constructor<?> requestConstructor;
    private static Field originalThreadField, looperField, count4Field, count6Field, entityField, channelField;
    private static HandlerThread originalThread;
    private static Looper originalLooper;
    private static int originalTid;
    private static long originalJavaId;
    private static final LinkedHashMap<String, Class<?>> parameters = new LinkedHashMap<>();
    private static final LinkedHashMap<String, Method> setters = new LinkedHashMap<>();
    private static final JSONArray targetBindings = new JSONArray();

    private static void require(boolean condition, String label) {
        if (!condition) throw new AssertionError(label);
    }
    private static long now() { return SystemClock.elapsedRealtimeNanos(); }
    private static void before() {
        require(Thread.currentThread() == DRIVER && now() < caseUntil, "original BOOTTIME case cutoff");
    }
    private static void beforeCleanup() {
        require(now() < cleanupUntil, "original BOOTTIME infrastructure cleanup cutoff");
    }
    private static Class<?> type(String name) throws Exception { return Class.forName(name, false, loader); }
    private static Field field(Class<?> type, String name) throws Exception {
        Field field = type.getDeclaredField(name);
        field.setAccessible(true);
        return field;
    }
    private static Throwable target(Throwable value) {
        while (value instanceof InvocationTargetException && value.getCause() != null) value = value.getCause();
        return value;
    }
    private static Object failure(Throwable error) throws Exception {
        if (error == null) return JSONObject.NULL;
        Throwable actual = target(error);
        String message = actual.getMessage();
        return new JSONObject().put("class", actual.getClass().getName())
            .put("message", message == null ? JSONObject.NULL : message.substring(0, Math.min(512, message.length())))
            .put("message_truncated", message != null && message.length() > 512);
    }
    private static String hex(byte[] value) {
        require(value != null && value.length <= VECTOR_BOUND, "bounded byte vector");
        char[] result = new char[value.length * 2];
        final String digits = "0123456789abcdef";
        for (int i = 0; i < value.length; i++) {
            int item = value[i] & 255;
            result[i * 2] = digits.charAt(item >>> 4);
            result[i * 2 + 1] = digits.charAt(item & 15);
        }
        return new String(result);
    }
    private static void emit(String id, JSONObject record) throws Exception {
        require(recordCount++ < RECORD_BOUND, "record count bound");
        String text = record.put("schema", 1).put("case_id", id).toString();
        int size = text.getBytes(StandardCharsets.UTF_8).length + 1;
        require(size <= RECORD_BYTES && outputBytes + size <= TOTAL_BYTES, "authored JSON byte bound");
        outputBytes += size;
        System.out.println(text);
        System.out.flush();
        require(!System.out.checkError(), "stdout write/flush");
    }
    private static Method bind(String className, String name, Class<?> result, Class<?>... args) throws Exception {
        Class<?> declaring = type(className);
        Method method = declaring.getDeclaredMethod(name, args);
        require(method.getDeclaringClass() == declaring && declaring.getClassLoader() == loader
            && method.getReturnType() == result, "exact original method declaration and loader");
        JSONArray types = new JSONArray();
        for (Class<?> arg : args) types.put(arg.getName());
        targetBindings.put(new JSONObject().put("declaring_class", className).put("method", name)
            .put("parameter_types", types).put("return_type", result.getName()).put("same_loader", true));
        return method;
    }
    private static void parameter(String name, Class<?> type) throws Exception {
        require(parameters.put(name, type) == null, "unique original setter");
        setters.put(name, bind(REQUEST, name, void.class, type));
    }
    private static Object calendarJSON(Calendar value) throws Exception {
        if (value == null) return JSONObject.NULL;
        return new JSONObject().put("timezone", value.getTimeZone().getID())
            .put("epoch_millis_dec", Long.toString(value.getTimeInMillis()))
            .put("year", value.get(Calendar.YEAR)).put("month_1_based", value.get(Calendar.MONTH) + 1)
            .put("day", value.get(Calendar.DAY_OF_MONTH)).put("hour_24", value.get(Calendar.HOUR_OF_DAY))
            .put("minute", value.get(Calendar.MINUTE)).put("second", value.get(Calendar.SECOND))
            .put("millisecond", value.get(Calendar.MILLISECOND))
            .put("zone_offset_millis", value.get(Calendar.ZONE_OFFSET)).put("dst_offset_millis", value.get(Calendar.DST_OFFSET));
    }
    private static Object inputValue(Object value) throws Exception {
        if (value == null) return JSONObject.NULL;
        if (value instanceof Integer || value instanceof String) return value;
        if (value instanceof Long) return Long.toString((Long)value);
        if (value instanceof byte[]) return hex((byte[])value);
        if (value instanceof String[]) {
            JSONArray result = new JSONArray();
            for (String item : (String[])value) result.put(item);
            return result;
        }
        if (value instanceof Calendar) return calendarJSON((Calendar)value);
        throw new AssertionError("unexpected authored input type");
    }
    private static Calendar fixedCalendar(boolean last) {
        GregorianCalendar value = new GregorianCalendar(TimeZone.getTimeZone(last ? "GMT-03:30" : "GMT+05:30"), Locale.ROOT);
        value.setLenient(false);
        value.clear();
        if (last) value.set(2020, Calendar.JANUARY, 2, 3, 4, 5);
        else value.set(2024, Calendar.FEBRUARY, 29, 23, 58, 59);
        value.set(Calendar.MILLISECOND, last ? 123 : 456);
        value.getTimeInMillis();
        return value;
    }
    private static String[] pcscf(int count, boolean v6) {
        require(count >= 0 && count <= 9, "finite authored PCSCF count");
        String[] result = new String[count];
        for (int i = 0; i < count; i++) result[i] = v6
            ? "2001:db8::" + Integer.toHexString(100 + i) : "192.0.2." + (100 + i);
        return result;
    }
    private static void numericAddress(String value, boolean v6) {
        require(value != null && value.length() <= 64, "explicit bounded numeric address");
        if (v6) {
            require(value.indexOf(':') >= 0 && value.matches("[0-9a-fA-F:]+"), "numeric IPv6 fixture only");
        } else {
            String[] parts = value.split("\\.", -1);
            require(parts.length == 4, "numeric IPv4 shape");
            for (String part : parts) require(part.matches("[0-9]{1,3}") && Integer.parseInt(part) <= 255,
                "numeric IPv4 octet");
        }
    }
    private static final class Input {
        final String id;
        final LinkedHashMap<String, Object> values = new LinkedHashMap<>();
        final Calendar timestamp;
        final int imsNetType;
        Input(String id, int imsNetType, int count4, int count6, boolean absentOptions) {
            this.id = id;
            this.imsNetType = imsNetType;
            timestamp = absentOptions ? null : fixedCalendar(false);
            values.put("setPdnType", 0);
            values.put("setIpType", 3);
            values.put("setImsPdnIpAddress4", "192.0.2.10");
            values.put("setImsPdnIpAddress6", "2001:db8::10");
            values.put("setHdInternetEnabled", 1);
            values.put("setOperatorId", 7);
            values.put("setRegStyle", 2);
            values.put("setPdnNew", 1);
            values.put("setRequireVolteAutoConfig", 0);
            values.put("setRequireRcsAutoConfig", 1);
            values.put("setOfferingType", 0x1234);
            values.put("setRegType", 4);
            values.put("setInitType", 0);
            values.put("setImsNetType", imsNetType);
            values.put("setWifiMacAddress", absentOptions ? null : new byte[]{2, 0, 0, 0, 0, 1});
            values.put("setPcscfIpAddress4List", pcscf(count4, false));
            values.put("setPcscfIpAddress4Num", count4);
            values.put("setPcscfIpAddress6List", pcscf(count6, true));
            values.put("setPcscfIpAddress6Num", count6);
            values.put("setPrimaryDnsIpAddress4", "192.0.2.53");
            values.put("setSecDnsIpAddress4", "198.51.100.53");
            values.put("setPrimaryDnsIpAddress6", "2001:db8::53");
            values.put("setSecDnsIpAddress6", "2001:db8:1::53");
            values.put("setLastAccessNetwork", 7);
            values.put("setLastAccessNetworkTimestamp", absentOptions ? null : fixedCalendar(true));
            values.put("setTimestamp", timestamp);
            values.put("setTacOrLac", 0x12345678);
            values.put("setIntTypeCellId", 0x01020304);
            values.put("setRemainingEmergRegTimerValue", 0x01020304);
            values.put("setNetworkMccMnc", absentOptions ? null : "310260");
            values.put("setCountryCode", absentOptions ? null : "US");
            values.put("setMtu", 1500);
            values.put("setRcsRegisteredCapabilityMask", 0x10203040);
            values.put("setVendorSpecificCapabilityMask", 0x1234);
            values.put("setExtendedRcsRegisteredCapabilityMask", 0x55667788);
            values.put("setVowifiMdnE911Aid", absentOptions ? null : "ORACLE");
            values.put("setBandVal", 0x01020304);
            values.put("setLongTypeCellId", 0x0102030405060708L);
            values.put("setNetworkInterfaceIndex", 0x00010203);
            values.put("setSipDelegateCapability", 0x1112131415161718L);
        }
        JSONObject json() throws Exception {
            JSONObject result = new JSONObject();
            for (Map.Entry<String, Object> item : values.entrySet()) result.put(item.getKey(),
                new JSONObject().put("java_type", parameters.get(item.getKey()).getName())
                    .put("value", inputValue(item.getValue())));
            return result;
        }
        void validate() {
            require(values.keySet().equals(parameters.keySet()), "complete finite original setter inputs");
            for (String key : new String[]{"setImsPdnIpAddress4", "setPrimaryDnsIpAddress4", "setSecDnsIpAddress4"})
                numericAddress((String)values.get(key), false);
            for (String key : new String[]{"setImsPdnIpAddress6", "setPrimaryDnsIpAddress6", "setSecDnsIpAddress6"})
                numericAddress((String)values.get(key), true);
            for (String value : (String[])values.get("setPcscfIpAddress4List")) numericAddress(value, false);
            for (String value : (String[])values.get("setPcscfIpAddress6List")) numericAddress(value, true);
            require(((String[])values.get("setPcscfIpAddress4List")).length == (Integer)values.get("setPcscfIpAddress4Num")
                && ((String[])values.get("setPcscfIpAddress6List")).length == (Integer)values.get("setPcscfIpAddress6Num"),
                "authored arrays match authored counts before original setter behavior");
        }
    }
    private static void probe(Input input) throws Exception {
        before();
        input.validate();
        JSONObject authored = input.json(), getters = new JSONObject(), stored = new JSONObject();
        Object id = null;
        byte[] bytes = null;
        Throwable escaped = null;
        String stage = "constructor";
        long invocationAt = 0, returnedAt = 0;
        try {
            Object request = requestConstructor.newInstance(new Object[]{channel, null});
            before();
            require(request.getClass() == type(REQUEST) && entityField.get(request) == null
                && channelField.get(request) == channel, "original constructor/channel and explicit null APN binding");
            stage = "setters";
            for (Map.Entry<String, Object> item : input.values.entrySet()) {
                before();
                setters.get(item.getKey()).invoke(request, new Object[]{item.getValue()});
                before();
            }
            stage = "getters";
            id = getMessageId.invoke(request);
            require(id instanceof Enum<?> && ((Enum<?>)id).getDeclaringClass() == type(MESSAGE_ID), "actual original message enum");
            Object actualTimestamp = getTimestamp.invoke(request);
            getters.put("message_enum", ((Enum<?>)id).name()).put("message_code", getCode.invoke(id))
                .put("ims_net_type", getImsNetType.invoke(request)).put("pdn_new", getPdnNew.invoke(request))
                .put("timestamp_identity", actualTimestamp == input.timestamp)
                .put("timestamp", calendarJSON((Calendar)actualTimestamp));
            require(getters.getInt("ims_net_type") == input.imsNetType && getters.getInt("pdn_new") == 1
                && actualTimestamp == input.timestamp, "source-bound original direct getter inputs");
            before();
            stored.put("source", "reflective original fields after public setters; no count getter is claimed")
                .put("pcscf_ipv4_count", count4Field.getInt(request)).put("pcscf_ipv6_count", count6Field.getInt(request));
            stage = "getBytes";
            before();
            invocationAt = now();
            try { bytes = (byte[])getBytes.invoke(request); }
            finally { returnedAt = now(); }
            before();
            require(bytes != null && bytes.length <= VECTOR_BOUND, "original bounded vector result");
            vectorCount++;
            stage = "complete";
        } catch (Throwable error) {
            escaped = target(error);
            escaped.printStackTrace(System.err); // Preserve complete caught diagnostics beside bounded JSON metadata.
            System.err.flush();
            require(!System.err.checkError(), "stderr diagnostic write/flush");
        }
        before();
        caseCount++;
        emit(input.id, new JSONObject().put("record", "codec").put("inputs", authored)
            .put("constructor_bypass", false).put("channel_constructor_bypass", true).put("apn_entity", JSONObject.NULL)
            .put("getters", getters).put("actual_stored_counts_after_setters", stored)
            .put("last_stage", stage).put("normal_completion", escaped == null).put("escaped_exception", failure(escaped))
            .put("invoked_at_boottime_ns", invocationAt).put("returned_at_boottime_ns", returnedAt)
            .put("length", bytes == null ? JSONObject.NULL : bytes.length)
            .put("hex", bytes == null || bytes.length > VECTOR_BOUND ? JSONObject.NULL : hex(bytes))
            .put("source_expected_vector", false).put("submitted", false).put("onMessage_called", false));
    }
    private static Looper observedLooper() throws Exception {
        // Same reviewed monitor/read pattern as the prior authored admission oracle.
        beforeCleanup();
        Looper value;
        synchronized (originalThread) { value = (Looper)looperField.get(originalThread); }
        beforeCleanup();
        return value;
    }
    private static JSONObject threadIdentity() throws Exception {
        return new JSONObject().put("source", SOLICITED + ".mHandlerThread")
            .put("java_id", originalJavaId).put("name", originalThread.getName()).put("tid", originalTid)
            .put("static_field_identity", originalThreadField.get(null) == originalThread)
            .put("looper_identity", originalLooper != null && observedLooper() == originalLooper)
            .put("looper_thread_identity", originalLooper != null && originalLooper.getThread() == originalThread)
            .put("alive", originalThread.isAlive()).put("proc_task_present", originalTid > 0
                && Files.exists(Paths.get("/proc/self/task/" + originalTid)));
    }
    private static void setup() throws Exception {
        before();
        loader = RegistrationAddPdnCodecOracle.class.getClassLoader();
        for (String name : new String[]{REQUEST, SOLICITED, CHANNEL, ADAPTOR, ENTITY, WRITER, MESSAGE_ID})
            require(type(name).getClassLoader() == loader, "original target loader binding");
        require(type(REQUEST).getSuperclass() == type(SOLICITED) && type(ADAPTOR).getSuperclass() == type(CHANNEL),
            "original request/channel superclass bindings");
        Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
        unsafe = field(unsafeType, "theUnsafe").get(null);
        allocate = unsafeType.getMethod("allocateInstance", Class.class);
        requestConstructor = type(REQUEST).getConstructor(type(CHANNEL), type(ENTITY));
        require(requestConstructor.getDeclaringClass() == type(REQUEST), "original public request constructor");
        getBytes = bind(REQUEST, "getBytes", byte[].class);
        getMessageId = bind(SOLICITED, "getMessageId", type(MESSAGE_ID));
        getCode = bind(MESSAGE_ID, "getCode", int.class);
        getImsNetType = bind(REQUEST, "getImsNetType", int.class);
        getPdnNew = bind(REQUEST, "getPdnNew", int.class);
        getTimestamp = bind(REQUEST, "getTimestamp", Calendar.class);
        for (String name : new String[]{"getAdditionalField", "getBandInfoArray", "getIntTypeTacOrLacArray",
                "getLongTypeCellIdArray", "getNetworkInterfaceIndexArray", "getSipDelegateCapabilityArray"})
            bind(REQUEST, name, byte[].class); // Binding only; original getBytes owns helper calls.
        bind(REQUEST, "getAdditionalFieldLength", int.class);
        for (String name : new String[]{"setPdnType", "setIpType", "setHdInternetEnabled", "setOperatorId", "setRegStyle",
                "setPdnNew", "setRequireVolteAutoConfig", "setRequireRcsAutoConfig", "setOfferingType", "setRegType",
                "setInitType", "setImsNetType", "setPcscfIpAddress4Num", "setPcscfIpAddress6Num", "setLastAccessNetwork",
                "setTacOrLac", "setIntTypeCellId", "setRemainingEmergRegTimerValue", "setMtu",
                "setRcsRegisteredCapabilityMask", "setVendorSpecificCapabilityMask", "setExtendedRcsRegisteredCapabilityMask",
                "setBandVal", "setNetworkInterfaceIndex"}) parameter(name, int.class);
        for (String name : new String[]{"setLongTypeCellId", "setSipDelegateCapability"}) parameter(name, long.class);
        for (String name : new String[]{"setImsPdnIpAddress4", "setImsPdnIpAddress6", "setPrimaryDnsIpAddress4",
                "setSecDnsIpAddress4", "setPrimaryDnsIpAddress6", "setSecDnsIpAddress6", "setNetworkMccMnc",
                "setCountryCode", "setVowifiMdnE911Aid"}) parameter(name, String.class);
        parameter("setPcscfIpAddress4List", String[].class);
        parameter("setPcscfIpAddress6List", String[].class);
        parameter("setLastAccessNetworkTimestamp", Calendar.class);
        parameter("setTimestamp", Calendar.class);
        parameter("setWifiMacAddress", byte[].class);
        require(parameters.size() == 40 && setters.size() == 40, "exact finite public setter inventory");
        for (String name : new String[]{"writeByte", "writeShort", "writeInt"}) bind(WRITER, name, void.class, int.class);
        bind(WRITER, "writeShort", void.class, short.class);
        bind(WRITER, "writeBytes", void.class, byte[].class);
        bind(WRITER, "writeBytes", void.class, byte[].class, int.class, int.class);
        bind(WRITER, "writeFixedByteArray", void.class, String.class, int.class, int.class);
        bind(WRITER, "toByteArray", byte[].class);
        require(targetBindings.length() == 61, "complete exact original method binding inventory");
        count4Field = field(type(REQUEST), "mPcscfIpAddress4Num");
        count6Field = field(type(REQUEST), "mPcscfIpAddress6Num");
        entityField = field(type(REQUEST), "mApnEntity");
        channelField = field(type(SOLICITED), "mNetChannel");
        // Prepare every reflective access before deliberately initializing original live infrastructure.
        originalThreadField = field(type(SOLICITED), "mHandlerThread");
        looperField = field(HandlerThread.class, "mLooper");
        Class.forName(SOLICITED, true, loader);
        Object retained = originalThreadField.get(null);
        require(retained instanceof HandlerThread, "original static HandlerThread");
        originalThread = (HandlerThread)retained;
        originalJavaId = originalThread.getId();
        while (observedLooper() == null) {
            before();
            require(originalThread.isAlive(), "original HandlerThread terminated during initialization");
            Thread.sleep(1);
        }
        originalLooper = observedLooper();
        originalTid = originalThread.getThreadId();
        require(originalThread.isAlive() && originalTid > 0 && originalLooper.getThread() == originalThread,
            "original thread/Looper/TID binding");
        channel = allocate.invoke(unsafe, type(ADAPTOR));
        require(channel.getClass() == type(ADAPTOR)
            && field(type(CHANNEL), "mNetwork").get(channel) == null
            && field(type(CHANNEL), "mUnsolList").get(channel) == null,
            "original allocated channel has no helper or unsolicited dispatch table");
        before();
    }
    private static void closeThread() throws Exception {
        if (originalThread == null) throw new AssertionError("original infrastructure was not retained");
        // Reused custody pattern: setup errors after retention still reach quit/join.
        Throwable captureError = null;
        JSONObject before = new JSONObject(), after = new JSONObject();
        boolean quit = false, quitRequested = false, joinCalled = false;
        long quitAt = 0, joinedAt = 0, taskAbsentAt = 0;
        try {
            while (originalThread.isAlive() && observedLooper() == null) {
                require(now() < cleanupUntil, "original cleanup cutoff awaiting original Looper");
                Thread.sleep(1);
            }
            if (originalLooper == null) originalLooper = observedLooper();
            if (originalTid <= 0) originalTid = originalThread.getThreadId();
            before = threadIdentity();
        } catch (Throwable error) { captureError = target(error); }
        try {
            quitAt = now();
            require(quitAt < cleanupUntil, "original cleanup cutoff before quit");
            if (originalLooper != null) {
                quitRequested = true;
                quit = originalThread.quitSafely();
            }
        } catch (Throwable error) { if (captureError == null) captureError = target(error); }
        try {
            long left = cleanupUntil - now();
            require(left > 0, "original cleanup cutoff before join");
            joinCalled = true;
            originalThread.join(left / 1000000L, (int)(left % 1000000L));
            joinedAt = now(); // Immediate actual join-return observation in BOOTTIME.
            while (originalTid > 0 && Files.exists(Paths.get("/proc/self/task/" + originalTid))
                    && now() < cleanupUntil) Thread.sleep(1);
            if (originalTid > 0 && !Files.exists(Paths.get("/proc/self/task/" + originalTid))) taskAbsentAt = now();
            after = threadIdentity();
        } catch (Throwable error) { if (captureError == null) captureError = target(error); }
        if (captureError != null) {
            captureError.printStackTrace(System.err);
            System.err.flush();
            require(!System.err.checkError(), "thread diagnostic write/flush");
        }
        emit("original-thread-closure", new JSONObject().put("record", "thread_closure")
            .put("before", before).put("after", after).put("quit_requested", quitRequested).put("quit_accepted", quit)
            .put("quit_at_ns", quitAt).put("actual_join_called", joinCalled).put("joined_at_ns", joinedAt)
            .put("task_absence_observed_at_ns", taskAbsentAt).put("capture_exception", failure(captureError))
            .put("clock", "CLOCK_BOOTTIME/SystemClock.elapsedRealtimeNanos")
            .put("original_case_until_ns", caseUntil).put("original_cleanup_until_ns", cleanupUntil));
        require(captureError == null && quitRequested && quit && joinCalled && joinedAt < cleanupUntil
            && taskAbsentAt > 0 && taskAbsentAt < cleanupUntil && !originalThread.isAlive()
            && originalTid > 0 && !Files.exists(Paths.get("/proc/self/task/" + originalTid)),
            "actual original thread quit/join closure");
    }
    public static void main(String[] args) throws Throwable {
        require(args.length == 0, "fixed no-argument oracle");
        startedAt = now();
        caseUntil = startedAt + 20000000000L;
        cleanupUntil = startedAt + 25000000000L;
        Throwable failed = null;
        try {
            setup();
            emit("add-pdn-association", new JSONObject().put("record", "association").put("expected_original_apk_sha256", APK_SHA)
                .put("runtime_DEX_allowlist_required_externally", true).put("original_types_only_from_APK_required_externally", true)
                .put("target_bindings", targetBindings).put("setter_count", setters.size())
                .put("original_handler_thread", threadIdentity()).put("clock", "CLOCK_BOOTTIME/SystemClock.elapsedRealtimeNanos")
                .put("started_at_ns", startedAt).put("original_case_until_ns", caseUntil).put("original_cleanup_until_ns", cleanupUntil)
                .put("original_request_constructor", true).put("channel_constructor_bypass", true).put("apn_entity", JSONObject.NULL)
                .put("numeric_addresses_only", true).put("original_response_dispatch_called", false).put("submitted", false));
            probe(new Input("numeric-v4-v6-pcscf1-wire0-fixed-time", 0, 1, 1, false));
            probe(new Input("empty-pcscf-wire2-fixed-time", 2, 0, 0, false));
            probe(new Input("empty-pcscf-wire2-null-optionals", 2, 0, 0, true));
            probe(new Input("pcscf-v4-count8", 0, 8, 0, false));
            probe(new Input("pcscf-v4-count9", 0, 9, 0, false));
            probe(new Input("pcscf-v6-count8", 0, 0, 8, false));
            probe(new Input("pcscf-v6-count9", 0, 0, 9, false));
            require(caseCount == CASES, "complete finite codec case set");
        } catch (Throwable error) { failed = target(error); }
        finally {
            try { closeThread(); }
            catch (Throwable error) {
                if (failed == null) failed = target(error);
                else failed.addSuppressed(target(error));
            }
        }
        if (failed != null) throw failed;
        beforeCleanup();
        emit("add-pdn-summary", new JSONObject().put("record", "summary").put("cases", caseCount)
            .put("bounded_vectors", vectorCount).put("all_cases_produced_vectors", vectorCount == CASES)
            .put("authored_JSON_records_including_summary", RECORD_BOUND).put("original_thread_closed", true)
            .put("source_expected_vectors", false).put("native_encoder_executed", false)
            .put("apn_acquisition_qualified", false)
            .put("remote_outcome_qualified", false).put("response_dispatch_called", false).put("phone_accessed", false));
        require(now() < cleanupUntil && vectorCount == CASES, "complete bounded vectors under original BOOTTIME cutoff");
    }
}
