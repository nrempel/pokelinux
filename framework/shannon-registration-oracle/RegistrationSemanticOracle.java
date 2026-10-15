package org.omarchy.audit;

import android.net.Uri;
import java.lang.reflect.Array;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.HashSet;
import java.util.Set;
import org.json.JSONArray;
import org.json.JSONObject;

/** Authored original message-to-captured-listener probe; no helper/channel dispatch. */
public final class RegistrationSemanticOracle {
    private static final String DATA = "com.shannon.imsservice.network.adaptor.data.RilIndRegistration";
    private static final String NETWORK = "com.shannon.imsservice.network.NetworkAdaptor";
    private static final String CHANNEL = NETWORK + "$AdaptorChannel";
    private static final String ADAPTOR = "com.shannon.imsservice.network.adaptor.RegistrationAdaptor";
    private static final String LISTENER = "com.shannon.imsservice.network.IImsConnectivityNetworkListener";
    private static final String MESSAGE_ID = "com.shannon.imsservice.network.oem.helper.ImsRilHelper$RilMessageId";
    private static final String APN = "com.shannon.imsservice.conn.ApnNetType";
    private static final String REASON = "com.shannon.imsservice.conn.RegistrationReason";
    private static final String APK_SHA256 =
        "8b017599727398e80df5ab618d648890dcdc34b5fe2ba71fb2331e51bd8b8879";
    private static final int MAX_CALLBACKS = 8;
    private static final int MAX_RECORD_BYTES = 16384;
    private static final int MAX_TOTAL_BYTES = 262144;
    private static final String[] FIELDS = {
        "mRegState", "mFeature", "mFeatureBitmask", "mStatusCode", "mUriLength",
        "mReasonLength", "mFeatureBitmaskExt", "mSipDelegateCapaBitmask", "mFeatureExt",
        "mUriReasonString"
    };
    private static final Set<String> emitted = new HashSet<>();
    private static final Set<String> allowedCallbacks = new HashSet<>();
    private static Class<?> dataType, networkType, channelType, adaptorType, listenerType, idType;
    private static Constructor<?> dataConstructor;
    private static Method allocateInstance, update, onMessage, getMessageId, getSlotId;
    private static Object allocator;
    private static int outputBytes, cases, owners;

    private static void require(boolean ok, String label) {
        if (!ok) throw new AssertionError(label);
    }
    private static Throwable target(Throwable error) {
        while (error instanceof InvocationTargetException && error.getCause() != null) {
            error = error.getCause();
        }
        return error;
    }
    private static Object exception(Throwable error) throws Exception {
        if (error == null) return JSONObject.NULL;
        String name = target(error).getClass().getName();
        require(name.length() <= 256, "exception class bound");
        return new JSONObject().put("class", name);
    }
    private static Object nullable(Object value) {
        return value == null ? JSONObject.NULL : value;
    }
    private static void emit(String id, JSONObject record) throws Exception {
        require(emitted.size() < 14 && emitted.add(id), "unique bounded record count");
        record.put("schema", 1).put("case_id", id);
        String text = record.toString();
        int length = text.getBytes(StandardCharsets.UTF_8).length + 1;
        require(length <= MAX_RECORD_BYTES && outputBytes + length <= MAX_TOTAL_BYTES,
            "authored JSON output bound");
        outputBytes += length;
        System.out.println(text);
        System.out.flush();
        require(!System.out.checkError(), "stdout write/flush");
    }
    private static String typeDescriptor(Class<?> type) {
        if (type.isArray()) return type.getName().replace('.', '/');
        if (!type.isPrimitive()) return "L" + type.getName().replace('.', '/') + ";";
        if (type == void.class) return "V";
        if (type == int.class) return "I";
        if (type == long.class) return "J";
        if (type == boolean.class) return "Z";
        if (type == byte.class) return "B";
        if (type == short.class) return "S";
        if (type == char.class) return "C";
        if (type == float.class) return "F";
        if (type == double.class) return "D";
        throw new AssertionError("unknown primitive type");
    }
    private static String descriptor(Method method) {
        StringBuilder text = new StringBuilder(method.getName()).append('(');
        for (Class<?> type : method.getParameterTypes()) text.append(typeDescriptor(type));
        return text.append(')').append(typeDescriptor(method.getReturnType())).toString();
    }
    private static Object optionalIntGetter(Object object, String getter) throws Exception {
        Method method;
        try {
            method = object.getClass().getMethod(getter);
        } catch (NoSuchMethodException absent) {
            return JSONObject.NULL;
        }
        require(method.getReturnType() == int.class, "enum int getter shape");
        return method.invoke(object);
    }
    private static Object value(Object object, int depth) throws Exception {
        require(depth <= 2, "callback value nesting bound");
        if (object == null) return JSONObject.NULL;
        if (object instanceof Boolean || object instanceof Integer || object instanceof Long
            || object instanceof Short || object instanceof Byte) return object;
        if (object instanceof String) {
            require(((String)object).length() <= 256, "callback string bound");
            return new JSONObject().put("kind", "string").put("value", object);
        }
        if (object instanceof Uri) {
            String text = object.toString();
            require(text.length() <= 256, "callback URI bound");
            return new JSONObject().put("kind", "uri").put("value", text);
        }
        if (object instanceof Enum<?>) {
            Enum<?> item = (Enum<?>)object;
            String type = item.getDeclaringClass().getName();
            require(type.equals(APN) || type.equals(REASON) || type.equals(DATA + "$RegState"),
                "callback enum type");
            return new JSONObject().put("kind", "enum").put("class", type).put("name", item.name())
                .put("getCode", optionalIntGetter(object, "getCode"))
                .put("getCpReason", optionalIntGetter(object, "getCpReason"));
        }
        if (object.getClass().isArray()) {
            int count = Array.getLength(object);
            require(count <= 8, "callback array bound");
            JSONArray items = new JSONArray();
            for (int i = 0; i < count; i++) items.put(value(Array.get(object, i), depth + 1));
            return new JSONObject().put("kind", "array").put("class", object.getClass().getName())
                .put("items", items);
        }
        throw new IllegalStateException("unexpected callback value class");
    }
    private static final class Capture {
        JSONArray calls = new JSONArray();
        String failure;
        boolean duringOnMessage;
        int outsideOnMessage;
        void begin() {
            calls = new JSONArray();
            failure = null;
            outsideOnMessage = 0;
            duringOnMessage = false;
        }
        Object invoke(Object proxy, Method method, Object[] args) throws Throwable {
            if (method.getDeclaringClass() == Object.class) {
                if (method.getName().equals("toString")) return "RegistrationSemanticCapture";
                if (method.getName().equals("hashCode")) return System.identityHashCode(proxy);
                if (method.getName().equals("equals")) return proxy == args[0];
                throw new AssertionError("unexpected Object method");
            }
            try {
                require(calls.length() < MAX_CALLBACKS, "callback count bound");
                String signature = descriptor(method);
                JSONObject call = new JSONObject().put("index", calls.length())
                    .put("method", method.getName()).put("descriptor", signature)
                    .put("during_original_onMessage", duringOnMessage);
                calls.put(call); // Retain an entry even if later shape/value capture fails.
                if (!duringOnMessage) outsideOnMessage++;
                require(duringOnMessage && method.getDeclaringClass() == listenerType
                    && method.getReturnType() == void.class && allowedCallbacks.contains(signature),
                    "unexpected listener entry or signature");
                Class<?>[] types = method.getParameterTypes();
                int count = args == null ? 0 : args.length;
                require(count == types.length, "callback argument count");
                JSONArray arguments = new JSONArray();
                for (int i = 0; i < count; i++) {
                    arguments.put(new JSONObject().put("declared_type", types[i].getName())
                        .put("value", value(args[i], 0)));
                }
                call.put("arguments", arguments);
                return null; // Capture only: never forward to a concrete service listener.
            } catch (Throwable error) {
                if (failure == null) failure = target(error).getClass().getName();
                // Original onMessage may catch Exception; the sticky failure still refuses qualification.
                throw new IllegalStateException("capture refused unexpected callback shape", error);
            }
        }
    }
    private static Field field(Class<?> type, String name) throws Exception {
        Field field = type.getDeclaredField(name);
        field.setAccessible(true);
        return field;
    }
    private static JSONObject fields(Object message) throws Exception {
        JSONObject result = new JSONObject();
        for (String name : FIELDS) {
            Object item = field(dataType, name).get(message);
            require(item == null || item instanceof Integer || item instanceof Long || item instanceof String,
                "original field capture type");
            if (item instanceof String) require(((String)item).length() <= 256, "original field text bound");
            result.put(name, nullable(item));
        }
        return result;
    }
    private static final class Owner {
        final int number, slot;
        final Object network, channel, message, messageId;
        final Capture capture = new Capture();
        Owner(int selectedSlot) throws Exception {
            require(selectedSlot == 0 || selectedSlot == 1, "authored explicit slot");
            number = ++owners;
            slot = selectedSlot;
            // Deliberate constructor bypass; original class initialization may still execute.
            network = allocateInstance.invoke(allocator, networkType);
            channel = allocateInstance.invoke(allocator, adaptorType);
            require(channelType.isInstance(channel), "original concrete channel subtype");
            require(field(channelType, "mUnsolList").get(channel) == null
                && field(channelType, "mNetwork").get(channel) == null
                && field(channelType, "mStateLister").get(channel) == null,
                "no initialized helper/table/state listener");
            field(networkType, "mSlotId").setInt(network, slot);
            channelType.getMethod("setNetAdaptor", networkType).invoke(channel, network);
            Object listener = Proxy.newProxyInstance(listenerType.getClassLoader(),
                new Class<?>[]{listenerType}, capture::invoke);
            networkType.getMethod("setConnectivityListener", listenerType).invoke(network, listener);
            require(channelType.getMethod("getNetAdaptor").invoke(channel) == network
                && networkType.getMethod("getConnectivityListener").invoke(network) == listener
                && ((Integer)getSlotId.invoke(channel)) == slot, "original setters and slot binding");
            message = dataConstructor.newInstance(channel); // Only the original data/base constructors.
            messageId = getMessageId.invoke(message);
            require(messageId != null && messageId.getClass() == idType && messageId instanceof Enum<?>
                && ((Integer)idType.getMethod("getCode").invoke(messageId)) == 3012,
                "actual original registration message association");
            require(capture.calls.length() == 0 && capture.failure == null,
                "no constructor/setter callback entry");
        }
    }
    private static byte[] packet(int state) {
        require(state >= 0 && state <= 255, "authored state byte");
        return ByteBuffer.allocate(12).order(ByteOrder.LITTLE_ENDIAN)
            .put((byte)state).put((byte)0).putInt(0).putShort((short)0)
            .putShort((short)0).putShort((short)0).array();
    }
    private static JSONArray bytes(byte[] input) {
        require(input.length == 12, "complete authored minimal packet");
        JSONArray array = new JSONArray();
        for (byte item : input) array.put(item & 255);
        return array;
    }
    private static void probe(String id, Owner owner, int state, boolean reused) throws Exception {
        owner.capture.begin();
        byte[] input = packet(state);
        JSONObject before = fields(owner.message);
        Throwable updateError = null, messageError = null;
        try { update.invoke(owner.message, input, input.length); }
        catch (InvocationTargetException error) { updateError = target(error); }
        JSONObject afterUpdate = fields(owner.message);
        boolean messageCalled = false;
        if (updateError == null) {
            messageCalled = true;
            owner.capture.duringOnMessage = true;
            try { onMessage.invoke(owner.message, owner.messageId); }
            catch (InvocationTargetException error) { messageError = target(error); }
            finally { owner.capture.duringOnMessage = false; }
        }
        JSONObject afterMessage = fields(owner.message);
        boolean parsed = updateError == null && afterUpdate.getInt("mRegState") == state
            && afterUpdate.getInt("mFeature") == 0 && afterUpdate.getInt("mFeatureBitmask") == 0
            && afterUpdate.getInt("mStatusCode") == 0 && afterUpdate.getInt("mUriLength") == 0
            && afterUpdate.getInt("mReasonLength") == 0;
        cases++;
        emit(id, new JSONObject().put("record", "message_listener_probe").put("owner", owner.number)
            .put("configured_slot", owner.slot).put("actual_channel_slot", getSlotId.invoke(owner.channel))
            .put("reused_message", reused).put("input_state_wire", state).put("input_bytes", bytes(input))
            .put("selected_length", input.length).put("constructor_bypass", true)
            .put("before_update", before).put("after_update", afterUpdate).put("after_onMessage", afterMessage)
            .put("update_exception", exception(updateError)).put("onMessage_called", messageCalled)
            .put("onMessage_exception", exception(messageError)).put("proxy_capture_failure", nullable(owner.capture.failure))
            .put("callbacks_outside_onMessage", owner.capture.outsideOnMessage)
            .put("callbacks", owner.capture.calls).put("parsed_authored_scalars_match", parsed)
            .put("concrete_listener_called", false).put("helper_or_channel_dispatch_called", false));
        // Only input/association/capture-shape invariants. Callback meanings/count/order are observed.
        require(parsed && messageCalled && messageError == null && owner.capture.failure == null
            && owner.capture.outsideOnMessage == 0, "original probe/capture shape qualification");
        require(((Integer)getSlotId.invoke(owner.channel)) == owner.slot
            && field(channelType, "mUnsolList").get(owner.channel) == null,
            "slot/table unchanged after direct message boundary");
    }
    private static void setup() throws Exception {
        ClassLoader loader = RegistrationSemanticOracle.class.getClassLoader();
        dataType = Class.forName(DATA, false, loader);
        networkType = Class.forName(NETWORK, false, loader);
        channelType = Class.forName(CHANNEL, false, loader);
        adaptorType = Class.forName(ADAPTOR, false, loader);
        listenerType = Class.forName(LISTENER, false, loader);
        idType = Class.forName(MESSAGE_ID, false, loader);
        Class<?> apn = Class.forName(APN, false, loader);
        Class<?> reason = Class.forName(REASON, false, loader);
        Class<?> regState = Class.forName(DATA + "$RegState", false, loader);
        dataConstructor = dataType.getConstructor(channelType);
        update = dataType.getMethod("update", byte[].class, int.class);
        onMessage = dataType.getMethod("onMessage", idType);
        getMessageId = dataType.getMethod("getMessageId");
        getSlotId = channelType.getMethod("getSlotId");
        require(update.getReturnType() == void.class && onMessage.getReturnType() == void.class
            && getMessageId.getReturnType() == idType && getSlotId.getReturnType() == int.class
            && listenerType.isInterface(), "original reflection signature contract");
        Method[] methods = {
            listenerType.getMethod("registrationAssociatedUriChanged", Uri[].class),
            listenerType.getMethod("registrationConnected", apn, reason),
            listenerType.getMethod("registrationConnected", apn, reason, boolean.class),
            listenerType.getMethod("registrationDisconnected", apn, reason, int.class, String.class),
            listenerType.getMethod("registrationFeatureCapabilityChanged", apn, int.class, int.class,
                int.class, long.class, regState, int.class)
        };
        for (Method method : methods) {
            require(method.getReturnType() == void.class, "original listener void signature");
            allowedCallbacks.add(descriptor(method));
        }
        require(allowedCallbacks.size() == 5, "exact overload-aware callback shapes");
        Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
        Field singleton = field(unsafeType, "theUnsafe");
        allocator = singleton.get(null);
        allocateInstance = unsafeType.getMethod("allocateInstance", Class.class);
    }
    private static void run() throws Exception {
        setup();
        Owner first = new Owner(0);
        JSONArray shapes = new JSONArray();
        for (String shape : new java.util.TreeSet<String>(allowedCallbacks)) shapes.put(shape);
        emit("semantic-association", new JSONObject().put("record", "association")
            .put("message_class", DATA).put("actual_message_enum", ((Enum<?>)first.messageId).name())
            .put("actual_message_code", idType.getMethod("getCode").invoke(first.messageId))
            .put("original_onMessage_argument", "actual getMessageId enum; not a new message-ID filter test")
            .put("listener_interface", LISTENER).put("allowed_callback_descriptors", shapes)
            .put("constructor_bypass", true).put("bypassed_types", new JSONArray(new String[]{NETWORK, ADAPTOR}))
            .put("data_constructor_original", true).put("original_class_initializers_may_execute", true)
            .put("expected_original_apk_sha256", APK_SHA256).put("apk_hash_checked_by_this_oracle", false)
            .put("external_runner_must_pin_apk_source_and_actual_process", true)
            .put("helper_open_called", false).put("helper_or_channel_dispatch_called", false)
            .put("concrete_listener_called", false).put("vendor_native_load_called_by_oracle", false)
            .put("callback_semantic_expectations_asserted", false));
        for (int slot = 0; slot <= 1; slot++) {
            Owner transition = slot == 0 ? first : new Owner(slot);
            probe("slot-" + slot + "-reused-state-2", transition, 2, false);
            probe("slot-" + slot + "-reused-state-1", transition, 1, true);
            for (int state : new int[]{1, 2, 255, 3}) {
                probe("slot-" + slot + "-fresh-state-" + state, new Owner(slot), state, false);
            }
        }
        require(cases == 12 && owners == 10 && emitted.size() == 13, "complete finite case set");
        emit("semantic-summary", new JSONObject().put("record", "summary").put("probe_cases", cases)
            .put("original_data_instances", owners).put("authored_JSON_records_including_summary", 14)
            .put("constructor_bypass", true).put("harness_invariants_passed", true)
            .put("callback_semantic_expectations_asserted", false)
            .put("scope", "original direct registration message-to-captured-listener only")
            .put("full_channel_helper_or_IMS_qualification", false).put("phone_accessed", false));
    }
    public static void main(String[] args) throws Exception {
        require(args.length == 0, "fixed no-argument oracle");
        run();
    }
}
