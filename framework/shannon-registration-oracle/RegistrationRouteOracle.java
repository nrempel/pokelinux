package org.omarchy.audit;

import android.os.Parcel;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import org.json.JSONArray;
import org.json.JSONObject;

/** Authored synthetic original raw dispatch/parser oracle. No services or semantic delivery. */
public final class RegistrationRouteOracle {
    private static final String HAL = "com.shannon.imsservice.network.oem.hal.";
    private static final String IND =
        "vendor.samsung_slsi.telephony.hardware.radioExternal.IOemSlsiRadioExternalInd";
    private static final String DATA = "com.shannon.imsservice.network.adaptor.data.RilIndRegistration";
    private static final String APK_SHA256 =
        "8b017599727398e80df5ab618d648890dcdc34b5fe2ba71fb2331e51bd8b8879";
    private static final int CANDIDATE_REGISTRATION_ID = 3012;
    private static final int MAX_PAYLOAD = 96;
    private static final int MAX_RECORD_BYTES = 8192;
    private static final int MAX_TOTAL_BYTES = 131072;
    private static final int MAX_RECORDS = 24;
    private static final String[] PARSER_FIELDS = {
        "mRegState", "mFeature", "mFeatureBitmask", "mStatusCode", "mUriLength",
        "mReasonLength", "mFeatureBitmaskExt", "mSipDelegateCapaBitmask", "mFeatureExt",
        "mUriReasonString"
    };
    private static final Set<String> emittedIds = new HashSet<>();
    private static int outputBytes;
    private static int routeCases;
    private static int parserCases;
    private static int registrationId;
    private static Class<?> registrationType;
    private static Constructor<?> registrationConstructor;
    private static Method registrationUpdate;

    private enum Delivery { ONE, FILTERED, NULL_PAYLOAD, TRAILING_PARCEL }

    private static void require(boolean ok, String label) {
        if (!ok) throw new AssertionError(label);
    }

    private static Object nullable(Object value) {
        return value == null ? JSONObject.NULL : value;
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

    private static Object vector(byte[] bytes) throws Exception {
        if (bytes == null) return JSONObject.NULL;
        require(bytes.length <= MAX_PAYLOAD, "payload vector bound");
        JSONArray out = new JSONArray();
        for (byte value : bytes) out.put(value & 255);
        return out;
    }

    private static void emit(String id, JSONObject out) throws Exception {
        require(emittedIds.size() < MAX_RECORDS && emittedIds.add(id), "record count/unique ID");
        out.put("schema", 1).put("case_id", id);
        String line = out.toString();
        int size = line.getBytes(StandardCharsets.UTF_8).length + 1;
        require(size <= MAX_RECORD_BYTES && outputBytes + size <= MAX_TOTAL_BYTES, "JSON output bound");
        outputBytes += size;
        System.out.println(line);
        System.out.flush();
        require(!System.out.checkError(), "stdout write/flush");
    }

    private static final class Owner {
        final Object adapter;
        final Object stub;
        final Method dispatch;
        final int configuredSlot;
        final List<Object> captured = new ArrayList<>();
        final List<String> callbackMethods = new ArrayList<>();
        int callbackInvocations;

        Owner(int slot) throws Exception {
            configuredSlot = slot;
            Class<?> listener = Class.forName(HAL + "IOemSlsiRadioExternalIndListener");
            Object capture = Proxy.newProxyInstance(listener.getClassLoader(), new Class<?>[]{listener},
                (proxy, method, args) -> {
                    if (method.getDeclaringClass() == Object.class) {
                        if (method.getName().equals("toString")) return "RegistrationRouteCapture";
                        if (method.getName().equals("hashCode")) return System.identityHashCode(proxy);
                        if (method.getName().equals("equals")) return proxy == args[0];
                        throw new AssertionError("unexpected Object method");
                    }
                    callbackInvocations++;
                    require(callbackInvocations <= 2, "callback count bound");
                    callbackMethods.add(method.getName());
                    require(method.getName().equals("processUnsolicited"), "captured listener method");
                    require(args != null && args.length == 1 && args[0] != null, "captured listener argument");
                    captured.add(args[0]);
                    // Deliberate stop: never forward to OemRIL/helper/channel/onMessage.
                    return null;
                });
            Class<?> adapterType = Class.forName(HAL + "aidl.OemSlsiRadioExternalIndAidl");
            adapter = adapterType.getConstructor(int.class, listener).newInstance(slot, capture);
            stub = adapterType.getMethod("getStub").invoke(adapter);
            require(stub != null, "original concrete indication Stub");
            dispatch = Class.forName(IND + "$Stub").getMethod(
                "onTransact", int.class, Parcel.class, Parcel.class, int.class);
        }
    }

    private static int integer(Object object, String getter) throws Exception {
        Object value = object.getClass().getMethod(getter).invoke(object);
        require(value instanceof Integer, "original int getter " + getter);
        return (Integer)value;
    }

    private static JSONObject parserSnapshot(Object instance) throws Exception {
        JSONObject out = new JSONObject();
        for (String name : PARSER_FIELDS) {
            Field field = registrationType.getDeclaredField(name);
            field.setAccessible(true);
            Object value = field.get(instance);
            if (value instanceof String) require(((String)value).length() <= 256, "synthetic parser text bound");
            out.put(name, nullable(value));
        }
        return out;
    }

    private static JSONObject parseEntireDelivered(byte[] delivered, String policy, boolean valid)
        throws Exception {
        // Fresh original instance, null channel; only update/field reads, never onMessage.
        Object instance = registrationConstructor.newInstance(new Object[]{null});
        JSONObject out = new JSONObject().put("input_bytes", vector(delivered))
            .put("selected_length", delivered.length).put("entire_delivered_array", true)
            .put("native_policy_expectation", policy).put("native_decoder_executed", false)
            .put("before", parserSnapshot(instance));
        Throwable thrown = null;
        try {
            registrationUpdate.invoke(instance, delivered, delivered.length);
        } catch (InvocationTargetException error) {
            thrown = target(error);
        }
        JSONObject after = parserSnapshot(instance);
        out.put("escaped_exception", exception(thrown)).put("returned_normally", thrown == null)
            .put("after", after).put("semantic_onMessage_called", false);
        parserCases++;
        if (valid) {
            // Record before asserting; these scalar inputs carry no semantic state meaning.
            out.put("authored_complete_scalars_match", thrown == null
                && after.getInt("mRegState") == 255 && after.getInt("mFeature") == 128
                && after.getInt("mFeatureBitmask") == Integer.MIN_VALUE
                && after.getInt("mStatusCode") == 65535);
        }
        // Malformed cases record partial/default original state and any escaping exception.
        // No native acceptance is inferred from the original returning normally.
        return out;
    }

    private static void route(String id, int configuredSlot, int incomingSlot, int messageId,
        byte[] inputBytes, Delivery delivery, String policy, boolean parse, boolean valid) throws Exception {
        require(inputBytes == null || inputBytes.length <= MAX_PAYLOAD, "authored input bound");
        Owner owner = new Owner(configuredSlot);
        require(owner.callbackInvocations == 0 && owner.captured.isEmpty(), "fresh capture owner");
        Object handled = null;
        Throwable thrown = null;
        Parcel input = Parcel.obtain();
        try {
            input.writeInterfaceToken(IND);
            input.writeInt(messageId);
            input.writeInt(incomingSlot);
            input.writeByteArray(inputBytes);
            if (delivery == Delivery.TRAILING_PARCEL) input.writeInt(0x13579bdf);
            input.setDataPosition(0);
            try {
                // In-process execution of original generated dispatcher, not a driver/sender test.
                handled = owner.dispatch.invoke(owner.stub, 1, input, null, 1);
            } catch (InvocationTargetException error) {
                thrown = target(error);
            }
        } finally {
            input.recycle();
        }
        JSONObject out = new JSONObject().put("record", "raw_dispatch")
            .put("configured_slot", configuredSlot).put("incoming_slot", incomingSlot)
            .put("input_message_id", messageId).put("registration_message_id", registrationId)
            .put("input_bytes", vector(inputBytes)).put("input_length", inputBytes == null ? -1 : inputBytes.length)
            .put("authored_transaction_code", 1).put("authored_onTransact_flags", 1)
            .put("extra_parcel_int", delivery == Delivery.TRAILING_PARCEL)
            .put("dispatch_handled", nullable(handled)).put("dispatch_exception", exception(thrown))
            .put("callback_invocations", owner.callbackInvocations)
            .put("callback_methods", new JSONArray(owner.callbackMethods))
            .put("captured_count", owner.captured.size()).put("stub_class", owner.stub.getClass().getName())
            .put("native_route_selector_matches", incomingSlot == configuredSlot && messageId == registrationId)
            .put("native_policy_expectation", policy).put("native_decoder_executed", false)
            .put("downstream_dispatch_called", false).put("semantic_onMessage_called", false);
        boolean metadataMatches = false, bytesMatch = false, lengthMatches = false;
        boolean completeParserMatches = !parse || !valid;
        if (owner.captured.size() == 1) {
            Object result = owner.captured.get(0);
            byte[] delivered = (byte[])result.getClass().getMethod("getData").invoke(result);
            int reportedLength = integer(result, "getDataLength");
            int actualId = integer(result, "getId");
            int actualSlot = integer(result, "getChannel");
            JSONObject captured = new JSONObject().put("class", result.getClass().getName())
                .put("message_id", actualId).put("slot", actualSlot)
                .put("type", integer(result, "getType")).put("serial", integer(result, "getSerial"))
                .put("status", integer(result, "getStatus")).put("reported_data_length", reportedLength)
                .put("bytes", vector(delivered)).put("array_length", delivered == null ? -1 : delivered.length);
            out.put("captured", captured);
            if (delivery == Delivery.ONE) {
                metadataMatches = actualId == messageId && actualSlot == incomingSlot;
                bytesMatch = delivered != null && Arrays.equals(delivered, inputBytes);
                lengthMatches = delivered != null && reportedLength == delivered.length
                    && delivered.length == inputBytes.length;
                captured.put("authored_routing_metadata_matches", metadataMatches)
                    .put("authored_full_bytes_match", bytesMatch).put("authored_full_length_matches", lengthMatches);
                if (parse && delivered != null) {
                    require(messageId == registrationId && incomingSlot == configuredSlot,
                        "parser selected only by the authored exact registration route");
                    JSONObject parser = parseEntireDelivered(delivered, policy, valid);
                    out.put("parser", parser);
                    if (valid) completeParserMatches = parser.getBoolean("authored_complete_scalars_match");
                }
            }
        } else {
            out.put("captured", JSONObject.NULL);
        }
        routeCases++;
        // Emit actual observations before contract assertions, retaining anomalous facts on failure.
        emit(id, out);
        if (delivery == Delivery.ONE) {
            require(thrown == null && Boolean.TRUE.equals(handled), "positive original raw dispatch handled");
            require(owner.callbackInvocations == 1 && owner.captured.size() == 1,
                "one actual original listener invocation and capture");
            require(metadataMatches && bytesMatch && lengthMatches, "original raw routing/full-byte contract");
            require(completeParserMatches, "complete original parser scalar contract");
        } else {
            require(owner.callbackInvocations == 0 && owner.captured.isEmpty(), "no listener entry for refused input");
            if (delivery == Delivery.FILTERED) {
                require(thrown == null && Boolean.TRUE.equals(handled), "wrong-slot callback handled without delivery");
            } else if (delivery == Delivery.TRAILING_PARCEL) {
                require(thrown != null, "extra parcel data rejected before listener entry");
            }
            // Null-payload exception class/handled behavior are observations, not guessed expectations.
        }
    }

    private static byte[] packet(boolean extended) {
        byte[] uri = extended ? "sip:route".getBytes(StandardCharsets.US_ASCII) : new byte[0];
        byte[] reason = extended ? "ok".getBytes(StandardCharsets.US_ASCII) : new byte[0];
        ByteBuffer out = ByteBuffer.allocate(12 + uri.length + reason.length + (extended ? 14 : 0))
            .order(ByteOrder.LITTLE_ENDIAN);
        out.put((byte)255).put((byte)128).putInt(Integer.MIN_VALUE).putShort((short)65535);
        out.putShort((short)uri.length).putShort((short)reason.length).put(uri).put(reason);
        if (extended) out.putInt(0x11223344).putLong(0x0102030405060708L).putShort((short)0x3344);
        return out.array();
    }

    private static byte[] trailing(byte[] value) {
        byte[] out = Arrays.copyOf(value, value.length + 1);
        out[value.length] = 0x7e;
        return out;
    }

    private static void run() throws Exception {
        ClassLoader loader = RegistrationRouteOracle.class.getClassLoader();
        registrationType = Class.forName(DATA, false, loader);
        Class<?> channel = Class.forName("com.shannon.imsservice.network.NetworkAdaptor$AdaptorChannel", false, loader);
        registrationConstructor = registrationType.getConstructor(channel);
        registrationUpdate = registrationType.getMethod("update", byte[].class, int.class);
        Object original = registrationConstructor.newInstance(new Object[]{null});
        Object originalId = registrationType.getMethod("getMessageId").invoke(original);
        registrationId = (Integer)originalId.getClass().getMethod("getCode").invoke(originalId);
        emit("registration-id-association", new JSONObject().put("record", "association")
            .put("original_class", registrationType.getName()).put("constructor_channel", JSONObject.NULL)
            .put("actual_enum_name", ((Enum<?>)originalId).name())
            .put("actual_wire_code", registrationId).put("observed_enum_ordinal_not_used_for_routing", ((Enum<?>)originalId).ordinal())
            .put("static_candidate_wire_code", CANDIDATE_REGISTRATION_ID)
            .put("candidate_matches_actual", registrationId == CANDIDATE_REGISTRATION_ID)
            .put("expected_original_apk_sha256", APK_SHA256).put("apk_hash_checked_by_this_oracle", false)
            .put("external_runner_must_pin_apk_and_actual_process", true)
            .put("original_parser_prior_scope", "121 probes:37 admitted complete ranges/84 native-policy refusals; not rerun here")
            .put("semantic_state_meaning", "unknown").put("semantic_onMessage_called", false));
        require(registrationId == CANDIDATE_REGISTRATION_ID, "static registration code candidate mismatch");
        require(((Enum<?>)originalId).name().equals("RILC_UNSOL_AIMS_REGISTRATION"), "registration enum association");
        byte[] minimal = packet(false), full = packet(true);
        route("slot0-complete-minimal", 0, 0, registrationId, minimal, Delivery.ONE, "accept-wire-only", true, true);
        route("slot0-complete-extended", 0, 0, registrationId, full, Delivery.ONE, "accept-wire-only", true, true);
        route("slot1-complete-extended", 1, 1, registrationId, full, Delivery.ONE, "accept-wire-only", true, true);
        route("slot1-to-configured0", 0, 1, registrationId, full, Delivery.FILTERED, "ignore-wrong-slot", false, false);
        route("slot0-to-configured1", 1, 0, registrationId, full, Delivery.FILTERED, "ignore-wrong-slot", false, false);
        route("negative-slot-to-configured0", 0, -1, registrationId, full, Delivery.FILTERED, "ignore-wrong-slot", false, false);
        route("adjacent-message-below", 0, 0, registrationId - 1, full, Delivery.ONE, "ignore-nonregistration-id", false, false);
        route("adjacent-message-above", 0, 0, registrationId + 1, full, Delivery.ONE, "ignore-nonregistration-id", false, false);
        route("unknown-message-id", 0, 0, -1234567, full, Delivery.ONE, "ignore-nonregistration-id", false, false);
        route("empty-array", 0, 0, registrationId, new byte[0], Delivery.ONE, "reject-native-Truncated", true, false);
        route("null-array-matching-slot", 0, 0, registrationId, null, Delivery.NULL_PAYLOAD, "reject-null-transport", false, false);
        route("null-array-wrong-slot", 0, 1, registrationId, null, Delivery.FILTERED, "ignore-wrong-slot", false, false);
        route("truncated-header", 0, 0, registrationId, Arrays.copyOf(full, 11), Delivery.ONE, "reject-native-Truncated", true, false);
        route("truncated-body", 0, 0, registrationId, Arrays.copyOf(full, 22), Delivery.ONE, "reject-native-Truncated", true, false);
        route("trailing-after-minimal", 0, 0, registrationId, trailing(minimal), Delivery.ONE, "reject-native-TrailingData", true, false);
        route("trailing-after-extended", 0, 0, registrationId, trailing(full), Delivery.ONE, "reject-native-TrailingData", true, false);
        route("extra-parcel-int", 0, 0, registrationId, full, Delivery.TRAILING_PARCEL, "reject-parcel-trailing-data", false, false);
        require(routeCases == 17 && parserCases == 8, "exact authored case accounting");
        emit("complete", new JSONObject().put("record", "completion").put("passed", true)
            .put("route_cases", routeCases).put("parser_cases", parserCases).put("prior_decoder_scope_changed", false)
            .put("actual_original_calls", "AIDL raw onTransact/code1, captured listener, result getters, registration constructor/ID/update/field reads")
            .put("in_process_only", true).put("cross_process_binder_qualified", false)
            .put("factory_producer_or_reset_qualified", false).put("native_decoder_executed", false)
            .put("OemRIL_helper_channel_dispatch_called", false).put("semantic_onMessage_called", false)
            .put("NetworkAdaptor_constructed", false).put("services_or_modem_called", false)
            .put("phone_accessed", false).put("auth_or_PAM_called", false));
    }

    public static void main(String[] args) {
        try {
            require(args.length == 0, "no runtime mode or endpoint selectors");
            run();
            System.exit(0);
        } catch (Throwable error) {
            try {
                emit("oracle-failure", new JSONObject().put("record", "failure").put("passed", false)
                    .put("escaped_exception", exception(error)).put("completed_route_cases", routeCases)
                    .put("completed_parser_cases", parserCases));
            } catch (Throwable outputFailure) {
                // stdout failure remains a nonzero process outcome; no unbounded stack dump.
            }
            System.exit(1);
        }
    }
}
