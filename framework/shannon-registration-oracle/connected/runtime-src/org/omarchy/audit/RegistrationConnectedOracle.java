package org.omarchy.audit;

import android.os.HandlerThread;
import android.os.Looper;
import java.lang.reflect.Array;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.AbstractMap;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.Map;
import org.json.JSONArray;
import org.json.JSONObject;

/** Authored conditional original CONNECTED/admission probe. No wire or disconnected dispatch. */
public final class RegistrationConnectedOracle {
    private static final String CONN = "com.shannon.imsservice.conn.";
    private static final String NET = "com.shannon.imsservice.network.";
    private static final String OWNER = CONN + "ImsConnectivityMgr";
    private static final String ENTITY = CONN + "ApnEntity";
    private static final String APN = CONN + "ApnNetType";
    private static final String STATE = ENTITY + "$ImsServiceState";
    private static final String REASON = CONN + "RegistrationReason";
    private static final String REQUEST = NET + "adaptor.data.RilReqSetImsRegistrationState";
    private static final String SOLICITED = NET + "adaptor.SolicitedMessage";
    private static final String APK_SHA = "8b017599727398e80df5ab618d648890dcdc34b5fe2ba71fb2331e51bd8b8879";
    private static final int SLOT = 1;
    private static final String[] FLAGS = {
        "mIsAddPdnReady", "mIsReregistration", "mIsHoOngoing", "mIsRegistrationRetrying", "mIsPseudoReg",
        "mIsDisconnectedByNetwork", "mIsDisconnectedByUser", "mIsHoToWiFiReadySent",
        "mIsRequiredSendHoToWifiReady", "mIsVolteHoTriggered", "mReRegistrationRequired"
    };
    private static final Thread DRIVER = Thread.currentThread();
    private static long caseUntil, cleanupUntil;
    private static int recordCount, outputBytes, caseCount;
    private static ClassLoader loader;
    private static Object unsafe;
    private static Method allocate;
    private static Field originalThreadField, looperField;
    private static HandlerThread originalThread;
    private static Looper originalLooper;
    private static int originalTid;
    private static long originalJavaId;
    private static Graph current;
    private static String phase;
    private static final JSONArray targetBindings = new JSONArray();

    private static void require(boolean condition, String label) {
        if (!condition) throw new AssertionError(label);
    }
    private static void before() { require(System.nanoTime() < caseUntil, "original case cutoff"); }
    private static Class<?> type(String name) throws Exception { return Class.forName(name, false, loader); }
    private static Field field(Class<?> type, String name) throws Exception {
        Field field = type.getDeclaredField(name);
        field.setAccessible(true);
        return field;
    }
    private static Object constant(String type, String name) throws Exception {
        return RegistrationConnectedOracle.type(type).getField(name).get(null);
    }
    private static Object fresh(String name) throws Exception { return allocate.invoke(unsafe, type(name)); }
    private static Object call(Object target, String name) throws Exception {
        return target.getClass().getMethod(name).invoke(target);
    }
    private static Throwable target(Throwable value) {
        while (value instanceof InvocationTargetException && value.getCause() != null) value = value.getCause();
        return value;
    }
    private static Object failure(Throwable error) throws Exception {
        return error == null ? JSONObject.NULL : new JSONObject().put("class", target(error).getClass().getName());
    }
    private static String enumName(Object value) {
        require(value instanceof Enum<?>, "actual original enum");
        return ((Enum<?>)value).name();
    }
    private static Object encoded(Object value) throws Exception {
        if (value == null) return JSONObject.NULL;
        if (value instanceof Boolean || value instanceof Integer || value instanceof Long) return value;
        require(value instanceof Enum<?>, "peripheral argument is primitive or original enum");
        Enum<?> item = (Enum<?>)value;
        require(item.getDeclaringClass().getClassLoader() == loader, "original enum runtime loader");
        return new JSONObject().put("class", item.getDeclaringClass().getName()).put("name", item.name());
    }
    private static void emit(String id, JSONObject record) throws Exception {
        require(recordCount++ < 10, "record count bound");
        String text = record.put("schema", 1).put("case_id", id).toString();
        int size = text.getBytes(StandardCharsets.UTF_8).length + 1;
        require(size <= 32768 && outputBytes + size <= 262144, "authored JSON bound");
        outputBytes += size;
        System.out.println(text);
        System.out.flush();
        require(!System.out.checkError(), "stdout write/flush");
    }
    /** The only three peripheral override categories; called by distinct authored subclasses. */
    public static void effect(String name, Object... arguments) {
        try {
            require(current != null && Thread.currentThread() == DRIVER, "owned synchronous peripheral capture");
            before();
            if (name.equals("owner.isRcsEnabled")) {
                require(arguments.length == 0, "RCS cut arity");
            } else {
                require("connected".equals(phase), "peripheral effect only during connected handler");
                if (name.equals("owner.triggerRegistering")) {
                    require(arguments.length == 1 && arguments[0] instanceof Enum<?>
                        && ((Enum<?>)arguments[0]).getDeclaringClass().getName().equals(OWNER + "$ImsTriggerReason"),
                        "trigger capture type");
                } else if (name.equals("calls.updateAllCalls")) {
                    require(arguments.length == 1 && arguments[0] == current.apn, "call update APN binding");
                } else if (name.equals("calls.updateHandoverStateForCall")) {
                    require(arguments.length == 4 && arguments[0] instanceof Integer
                        && arguments[1] instanceof Integer && arguments[2] instanceof Boolean
                        && arguments[3] == current.apn, "call handover capture shape");
                } else throw new AssertionError("unexpected peripheral override");
            }
            require(current.effects.length() < 64, "effect count bound");
            JSONArray args = new JSONArray();
            for (Object argument : arguments) args.put(encoded(argument));
            current.effects.put(new JSONObject().put("index", current.effects.length()).put("phase", phase)
                .put("method", name).put("arguments", args));
        } catch (Throwable error) {
            if (current != null && current.captureFailure == null) current.captureFailure = target(error).getClass().getName();
            throw new IllegalStateException("peripheral capture refused", error);
        }
    }
    private static final class Singleton {
        final Object object;
        final Map<Object, Object> map;
        @SuppressWarnings("unchecked")
        Singleton(String name, String fieldName, Object object) throws Exception {
            this.object = object;
            Object value = field(type(name), fieldName).get(null);
            require(value instanceof Map<?, ?>, "original singleton map type");
            map = (Map<Object, Object>)value;
            require(map.isEmpty(), "fresh original singleton map required");
            map.put(SLOT, object);
        }
        void verify(String name) throws Exception {
            require(map.size() == 1 && map.get(SLOT) == object
                && type(name).getMethod("getInstance", int.class).invoke(null, SLOT) == object,
                "original singleton selected-slot identity");
        }
        void close() { require(map.remove(SLOT) == object && map.isEmpty(), "restore original empty singleton map"); }
    }
    private static final class Graph implements AutoCloseable {
        final Object owner, manager, stack, calls, connListener, regListener, adaptor, network, entity, cell, apn;
        final ArrayList<Singleton> singletons = new ArrayList<>();
        final HashMap<Object, Object> requests = new HashMap<>();
        final JSONArray effects = new JSONArray(), helperRequests = new JSONArray();
        final boolean entityPresent, requestPresent;
        String captureFailure;
        Graph(boolean hasEntity, boolean hasRequest) throws Exception {
            boolean initialized = false;
            try {
            entityPresent = hasEntity;
            requestPresent = hasRequest;
            owner = fresh(CONN + "OracleConnectivityMgr");
            calls = fresh("com.shannon.imsservice.call.OracleCallManager");
            connListener = fresh(OWNER + "$ConnListener");
            regListener = fresh(OWNER + "$RegistrationListener");
            stack = fresh(CONN + "StackControl");
            adaptor = fresh(NET + "adaptor.RegistrationAdaptor");
            network = fresh(NET + "NetworkAdaptor");
            cell = type(CONN + "rule.CellConnectivity").getConstructor().newInstance();
            apn = constant(APN, "APN_IMS");
            entity = hasEntity ? fresh(ENTITY) : null;
            field(type(OWNER), "mSlotId").setInt(owner, SLOT);
            field(type(OWNER), "mPsfbStatus").set(owner, constant(OWNER + "$PsfbStatus", "UNKNOWN"));
            field(type(OWNER), "mPsfbTargetRat").set(owner, constant(CONN + "RadioAccessTech", "RAT_UNKNOWN"));
            field(type(OWNER + "$ConnListener"), "this$0").set(connListener, owner);
            field(type(OWNER + "$RegistrationListener"), "this$0").set(regListener, owner);
            manager = type(CONN + "ApnManager").getConstructor(int.class, type(OWNER + "$ConnListener"))
                .newInstance(SLOT, connListener);
            field(type(OWNER), "mApnManager").set(owner, manager);
            field(type(OWNER), "mStackControl").set(owner, stack);
            field(type(OWNER), "mCallManager").set(owner, calls);
            field(type(CONN + "StackControl"), "mSlotId").setInt(stack, SLOT);
            field(type(CONN + "StackControl"), "mSlotIdForImsStack").setInt(stack, SLOT);
            if (hasRequest) requests.put(apn, new AbstractMap.SimpleEntry<Integer, Integer>(SLOT, 0));
            field(type(CONN + "StackControl"), "mSavedApnList").set(stack, requests);
            field(type(NET + "NetworkAdaptor"), "mSlotId").setInt(network, SLOT);
            type(NET + "NetworkAdaptor$AdaptorChannel").getMethod("setNetAdaptor", type(NET + "NetworkAdaptor"))
                .invoke(adaptor, network);
            Class<?> helperType = type(NET + "NetworkAdaptor$INetworkHelper");
            Object helper = Proxy.newProxyInstance(loader, new Class<?>[]{helperType}, (proxy, method, args) -> {
                if (method.getDeclaringClass() == Object.class) {
                    if (method.getName().equals("toString")) return "ConnectedHelperCapture";
                    if (method.getName().equals("hashCode")) return System.identityHashCode(proxy);
                    if (method.getName().equals("equals")) return proxy == args[0];
                }
                try {
                    require(current == this && "connected".equals(phase) && Thread.currentThread() == DRIVER,
                        "helper capture current original handler only");
                    before();
                    require(method.getName().equals("sendRequest") && method.getReturnType() == void.class
                        && method.getParameterTypes().length == 1
                        && method.getParameterTypes()[0] == type(SOLICITED)
                        && args != null && args.length == 1 && args[0] != null
                        && args[0].getClass() == type(REQUEST), "only original registration-state request is captured");
                    Object request = args[0];
                    require(field(type(SOLICITED), "mNetChannel").get(request) == adaptor, "request original channel identity");
                    require(helperRequests.length() < 4, "request capture bound");
                    Object id = call(request, "getMessageId");
                    helperRequests.put(new JSONObject().put("index", helperRequests.length()).put("class", REQUEST)
                        .put("message_enum", enumName(id)).put("message_code", call(id, "getCode"))
                        .put("registration_state", call(request, "getRegistrationState"))
                        .put("registered_rat", call(request, "getRegisteredRat"))
                        .put("selected_slot", SLOT).put("forwarded", false));
                    return null;
                } catch (Throwable error) {
                    if (captureFailure == null) captureFailure = target(error).getClass().getName();
                    throw new IllegalStateException("unexpected original helper call", error);
                }
            });
            field(type(NET + "NetworkAdaptor$AdaptorChannel"), "mNetwork").set(adaptor, helper);
            Object adaptors = Array.newInstance(type(NET + "adaptor.RegistrationAdaptor"), 2);
            Array.set(adaptors, SLOT, adaptor);
            field(type(CONN + "StackControl"), "mRegistrationAdaptor").set(stack, adaptors);
            if (entity != null) {
                field(type(ENTITY), "mApnType").set(entity, apn);
                field(type(ENTITY), "mConnectivity").set(entity, cell);
                field(type(ENTITY), "mServiceState").set(entity, constant(STATE, "IMS_DEREGISTERED"));
                field(type(ENTITY), "mConnListener").set(entity, connListener);
                field(type(ENTITY), "mRegistrationStateCallbacks").set(entity, new ArrayList<Object>());
                field(type(ENTITY), "mApnEventCallbacks").set(entity, new ArrayList<Object>());
                field(type(ENTITY), "mConnectionState").set(entity, constant(ENTITY + "$ApnConnectionState", "APN_DISCONNECTED"));
                field(type(ENTITY), "mImsApnConnectionState").set(entity, constant(ENTITY + "$ApnConnectionState", "APN_DISCONNECTED"));
                field(type(ENTITY), "mHandoverSourceRat").set(entity, constant(CONN + "RadioAccessTech", "RAT_UNKNOWN"));
                for (String name : FLAGS) field(type(ENTITY), name).setBoolean(entity, false);
                field(type(ENTITY), "mInitType").setInt(entity, 0);
                Object apns = field(type(CONN + "ApnManager"), "mApns").get(manager);
                require(apns instanceof Map<?, ?> && ((Map<?, ?>)apns).isEmpty(), "original fresh APN table");
                Map.class.getMethod("put", Object.class, Object.class).invoke(apns, apn, entity);
            }
            singletons.add(new Singleton(OWNER, "sInstances", owner));
            singletons.add(new Singleton(CONN + "StackControl", "me", stack));
            singletons.add(new Singleton("com.shannon.imsservice.call.ImsCallManager", "me", calls));
            for (int n = 0; n < singletons.size(); n++) singletons.get(n).verify(new String[]{OWNER,
                CONN + "StackControl", "com.shannon.imsservice.call.ImsCallManager"}[n]);
            require(type(CONN + "rule.CellConnectivity").getField("mRegState").getInt(cell) == 0,
                "explicit disconnected original cellular marker");
            require(((Integer)call(adaptor, "getSlotId")) == SLOT
                && field(type(NET + "NetworkAdaptor$AdaptorChannel"), "mUnsolList").get(adaptor) == null,
                "original slot bound; no channel table dispatch");
            initialized = true;
            } finally {
                if (!initialized) close();
            }
        }
        boolean gate() throws Exception {
            Method method = type(OWNER + "$RegistrationListener").getDeclaredMethod("isExistPreviousRequest", type(ENTITY), type(APN));
            method.setAccessible(true);
            return (Boolean)method.invoke(regListener, entity, apn);
        }
        JSONObject snapshot() throws Exception {
            Object actualEntity = type(CONN + "ApnManager").getMethod("getApnEntity", type(APN)).invoke(manager, apn);
            require(actualEntity == entity, "original APN lookup identity");
            JSONObject flags = new JSONObject();
            if (entity != null) for (String name : FLAGS) flags.put(name, field(type(ENTITY), name).getBoolean(entity));
            return new JSONObject().put("selected_slot", SLOT).put("entity_present", entity != null)
                .put("request_membership", requests.containsKey(apn)).put("request_map_size", requests.size())
                .put("request_prior", requestPresent ? new JSONArray(new int[]{SLOT, 0}) : JSONObject.NULL)
                .put("apn", enumName(apn)).put("entity_state", entity == null ? JSONObject.NULL : enumName(call(entity, "getServiceState")))
                .put("manager_stored_state", enumName(field(type(CONN + "ApnManager"), "mCurRegState").get(manager)))
                .put("manager_apn_state", enumName(type(CONN + "ApnManager").getMethod("getRegistrationState", type(APN)).invoke(manager, apn)))
                .put("cell_registered", call(owner, "isCellRegistered")).put("wifi_registered", call(owner, "isWiFiRegistered"))
                .put("ims_registered", call(owner, "isImsRegistered"))
                .put("cell_marker_state", type(CONN + "rule.CellConnectivity").getField("mRegState").getInt(cell))
                .put("cell_physically_connected", call(cell, "isConnected")).put("wifi_entity_present", false)
                .put("entity_flags", flags).put("registration_observers", entity == null ? 0 :
                    ((ArrayList<?>)field(type(ENTITY), "mRegistrationStateCallbacks").get(entity)).size())
                .put("handover_source", entity == null ? JSONObject.NULL : enumName(field(type(ENTITY), "mHandoverSourceRat").get(entity)))
                .put("owner_psfb_status", field(type(OWNER), "mPsfbStatus").get(owner) == null ? JSONObject.NULL : enumName(field(type(OWNER), "mPsfbStatus").get(owner)))
                .put("owner_psfb_target", field(type(OWNER), "mPsfbTargetRat").get(owner) == null ? JSONObject.NULL : enumName(field(type(OWNER), "mPsfbTargetRat").get(owner)));
        }
        public void close() {
            Throwable failed = null;
            for (int n = singletons.size() - 1; n >= 0; n--) {
                try { singletons.get(n).close(); }
                catch (Throwable error) { if (failed == null) failed = error; }
            }
            if (failed != null) throw new AssertionError("original singleton restoration failed", failed);
        }
    }
    private static void probe(String id, boolean hasEntity, boolean hasRequest, boolean connected) throws Exception {
        before();
        try (Graph graph = new Graph(hasEntity, hasRequest)) {
            current = graph;
            phase = "before";
            JSONObject prior = graph.snapshot();
            require(prior.getBoolean("entity_present") == hasEntity
                && prior.getBoolean("request_membership") == hasRequest
                && prior.getString("manager_stored_state").equals("IMS_DEREGISTERED")
                && prior.getString("manager_apn_state").equals("IMS_DEREGISTERED")
                && !prior.getBoolean("cell_registered") && !prior.getBoolean("wifi_registered")
                && !prior.getBoolean("ims_registered") && !prior.getBoolean("cell_physically_connected")
                && prior.getInt("cell_marker_state") == 0 && prior.getInt("registration_observers") == 0,
                "explicit initial conditional priors");
            if (hasEntity) {
                require(prior.getString("entity_state").equals("IMS_DEREGISTERED"), "initial original entity state");
                for (String name : FLAGS) require(!prior.getJSONObject("entity_flags").getBoolean(name), "explicit initial entity flag");
            }
            phase = "admission";
            boolean gate = graph.gate();
            Throwable escaped = null;
            if (connected) {
                phase = "connected";
                try {
                    type(OWNER + "$RegistrationListener").getMethod("registrationConnected", type(APN), type(REASON))
                        .invoke(graph.regListener, graph.apn, constant(REASON, "IMS_SERVICE_REGISTER_SUCCESS"));
                } catch (InvocationTargetException error) { escaped = target(error); }
            }
            phase = "after";
            JSONObject after = graph.snapshot();
            before();
            caseCount++;
            emit(id, new JSONObject().put("record", connected ? "connected" : "admission")
                .put("constructor_bypass", true).put("entity_prior", hasEntity).put("request_prior", hasRequest)
                .put("actual_admission", gate).put("connected_called", connected).put("before", prior).put("after", after)
                .put("escaped_exception", failure(escaped)).put("normal_completion", escaped == null)
                .put("capture_failure", graph.captureFailure == null ? JSONObject.NULL : graph.captureFailure)
                .put("effects", graph.effects).put("helper_requests", graph.helperRequests)
                .put("disconnected_called", false).put("full_wire_callback_chain_called", false));
            require(gate == (hasEntity && hasRequest) && escaped == null && graph.captureFailure == null,
                "full original admission/connected completion");
            boolean admitted = connected && gate;
            require(after.getString("manager_stored_state").equals(admitted ? "IMS_REGISTERED" : "IMS_DEREGISTERED")
                && after.getString("manager_apn_state").equals(admitted ? "IMS_REGISTERED" : "IMS_DEREGISTERED")
                && after.getBoolean("cell_registered") == admitted && !after.getBoolean("wifi_registered")
                && after.getBoolean("ims_registered") == admitted, "original stored state and inherited predicates");
            require(!after.getBoolean("cell_physically_connected") && after.getInt("cell_marker_state") == 0,
                "no cellular acquisition inferred");
            if (hasEntity) require(after.getString("entity_state").equals(admitted ? "IMS_REGISTERED" : "IMS_DEREGISTERED")
                && !after.getJSONObject("entity_flags").getBoolean("mIsAddPdnReady"), "original entity state and explicit PDN-ready prior");
            require(graph.helperRequests.length() == (admitted ? 1 : 0), "original notification reaches capture exactly when admitted");
            int triggers = 0, updates = 0;
            for (int n = 0; n < graph.effects.length(); n++) {
                String name = graph.effects.getJSONObject(n).getString("method");
                if (name.equals("owner.triggerRegistering")) triggers++;
                if (name.equals("calls.updateAllCalls")) updates++;
                require(!name.equals("calls.updateHandoverStateForCall"), "RAT_UNKNOWN has no handover effect");
            }
            require(triggers == (admitted ? 1 : 0) && updates == (admitted ? 1 : 0), "complete admitted continuation and refused controls");
            if (!admitted) require(prior.toString().equals(after.toString()), "refused/admission-only case preserves all captured priors");
            if (admitted) {
                JSONObject request = graph.helperRequests.getJSONObject(0);
                require(request.getInt("registration_state") == 1 && request.getInt("registered_rat") == 1
                    && request.getString("message_enum").equals("RILC_REQ_SET_IMS_REGISTRATION_STATE")
                    && request.getInt("message_code") == 48,
                    "actual original conditional cellular registration-state request");
            }
        } finally { current = null; phase = null; }
    }
    private static Looper observedLooper() throws Exception {
        // HandlerThread publishes mLooper and notifies under this same monitor.
        // This critical section reads one field; no getLooper wait is introduced.
        beforeCleanup();
        Looper value;
        synchronized (originalThread) { value = (Looper)looperField.get(originalThread); }
        beforeCleanup();
        return value;
    }
    private static void beforeCleanup() {
        require(System.nanoTime() < cleanupUntil, "original infrastructure cleanup cutoff");
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
    private static void bindTarget(Class<?> query, Class<?> declaring, String name,
            Class<?> result, Class<?>... parameters) throws Exception {
        Method method = name.equals("isExistPreviousRequest")
            ? query.getDeclaredMethod(name, parameters) : query.getMethod(name, parameters);
        require(method.getDeclaringClass() == declaring && declaring.getClassLoader() == loader
            && method.getReturnType() == result, "exact original target declaration and loader");
        JSONArray types = new JSONArray();
        for (Class<?> parameter : parameters) types.put(parameter.getName());
        targetBindings.put(new JSONObject().put("declaring_class", declaring.getName()).put("method", name)
            .put("parameter_types", types).put("return_type", result.getName()).put("same_loader", true));
    }
    private static void setup() throws Exception {
        loader = RegistrationConnectedOracle.class.getClassLoader();
        Class<?> unsafeType = Class.forName("sun.misc.Unsafe");
        unsafe = field(unsafeType, "theUnsafe").get(null);
        allocate = unsafeType.getMethod("allocateInstance", Class.class);
        Class<?> owner = type(OWNER), calls = type("com.shannon.imsservice.call.ImsCallManager");
        require(type(CONN + "OracleConnectivityMgr").getSuperclass() == owner
            && type("com.shannon.imsservice.call.OracleCallManager").getSuperclass() == calls,
            "authored peripheral subclasses extend original targets");
        for (String name : new String[]{OWNER, CONN + "ApnManager", ENTITY, CONN + "StackControl",
                OWNER + "$ConnListener", OWNER + "$RegistrationListener", NET + "adaptor.RegistrationAdaptor",
                CONN + "OracleConnectivityMgr", "com.shannon.imsservice.call.OracleCallManager"}) {
            require(type(name).getClassLoader() == loader, "same runtime classloader and package access");
        }
        Class<?> listener = type(OWNER + "$RegistrationListener"), apn = type(APN), entity = type(ENTITY);
        Class<?> manager = type(CONN + "ApnManager"), state = type(STATE), reason = type(REASON);
        bindTarget(listener, listener, "isExistPreviousRequest", boolean.class, entity, apn);
        bindTarget(listener, listener, "registrationConnected", void.class, apn, reason);
        bindTarget(listener, listener, "registrationConnected", void.class, apn, reason, boolean.class);
        bindTarget(manager, manager, "getApnEntity", entity, apn);
        bindTarget(manager, manager, "getRegistrationState", state, apn);
        bindTarget(entity, entity, "getServiceState", state);
        bindTarget(entity, entity, "getConnectivity", type(CONN + "rule.RatConnectivity"));
        Class<?> cell = type(CONN + "rule.CellConnectivity");
        bindTarget(cell, cell, "isConnected", boolean.class);
        for (String name : new String[]{"isCellRegistered", "isWiFiRegistered", "isImsRegistered"}) {
            bindTarget(type(CONN + "OracleConnectivityMgr"), owner, name, boolean.class);
        }
        bindTarget(type(SOLICITED), type(SOLICITED), "getMessageId", type(NET + "oem.helper.ImsRilHelper$RilMessageId"));
        bindTarget(type(REQUEST), type(REQUEST), "getRegistrationState", int.class);
        bindTarget(type(REQUEST), type(REQUEST), "getRegisteredRat", int.class);
        Class<?> helper = type(NET + "NetworkAdaptor$INetworkHelper");
        bindTarget(helper, helper, "sendRequest", void.class, type(SOLICITED));
        require(targetBindings.length() == 15, "complete original target binding list");
        // Prepare every reflective access before deliberately initializing this original live infrastructure.
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
    }
    private static void closeThread() throws Exception {
        if (originalThread == null) throw new AssertionError("original infrastructure was not retained");
        // A setup error after retaining the thread must not prevent quit/join.
        Throwable captureError = null;
        JSONObject before = new JSONObject(), after = new JSONObject();
        boolean quit = false, quitRequested = false, joinCalled = false;
        long quitAt = 0, joinedAt = 0, taskAbsentAt = 0;
        try {
            while (originalThread.isAlive() && observedLooper() == null) {
                require(System.nanoTime() < cleanupUntil, "original cleanup cutoff awaiting original Looper");
                Thread.sleep(1);
            }
            if (originalLooper == null) originalLooper = observedLooper();
            if (originalTid <= 0) originalTid = originalThread.getThreadId();
            before = threadIdentity();
        } catch (Throwable error) { captureError = target(error); }
        try {
            quitAt = System.nanoTime();
            require(quitAt < cleanupUntil, "original cleanup cutoff before quit");
            if (originalLooper != null) {
                quitRequested = true;
                quit = originalThread.quitSafely();
            }
        } catch (Throwable error) { if (captureError == null) captureError = target(error); }
        try {
            long left = cleanupUntil - System.nanoTime();
            require(left > 0, "original cleanup cutoff before join");
            joinCalled = true;
            originalThread.join(left / 1000000L, (int)(left % 1000000L));
            joinedAt = System.nanoTime(); // Immediate actual join return observation.
            while (originalTid > 0 && Files.exists(Paths.get("/proc/self/task/" + originalTid))
                    && System.nanoTime() < cleanupUntil) Thread.sleep(1);
            if (originalTid > 0 && !Files.exists(Paths.get("/proc/self/task/" + originalTid))) taskAbsentAt = System.nanoTime();
            after = threadIdentity();
        } catch (Throwable error) { if (captureError == null) captureError = target(error); }
        emit("original-thread-closure", new JSONObject().put("record", "thread_closure")
            .put("before", before).put("after", after).put("quit_requested", quitRequested).put("quit_accepted", quit)
            .put("quit_at_ns", quitAt).put("actual_join_called", joinCalled).put("joined_at_ns", joinedAt)
            .put("task_absence_observed_at_ns", taskAbsentAt)
            .put("capture_exception", failure(captureError))
            .put("original_case_until_ns", caseUntil).put("original_cleanup_until_ns", cleanupUntil));
        require(captureError == null && quitRequested && quit && joinCalled && joinedAt < cleanupUntil
            && taskAbsentAt > 0 && taskAbsentAt < cleanupUntil && !originalThread.isAlive()
            && originalTid > 0 && !Files.exists(Paths.get("/proc/self/task/" + originalTid)),
            "actual original thread quit/join closure");
    }
    public static void main(String[] args) throws Throwable {
        require(args.length == 0, "fixed no-argument oracle");
        caseUntil = System.nanoTime() + 20000000000L;
        cleanupUntil = caseUntil + 5000000000L;
        Throwable failed = null;
        try {
            setup();
            emit("connected-association", new JSONObject().put("record", "association").put("selected_slot", SLOT)
                .put("constructor_bypass", true).put("expected_original_apk_sha256", APK_SHA)
                .put("runtime_DEX_allowlist_required_externally", true).put("original_types_only_from_APK_required_externally", true)
                .put("same_actual_classloader_verified", true).put("original_target_bindings_checked", true)
                .put("original_target_bindings", targetBindings).put("original_handler_thread", threadIdentity())
                .put("original_case_until_ns", caseUntil).put("original_cleanup_until_ns", cleanupUntil)
                .put("connected_only", true).put("disconnected_loss_qualified", false).put("physical_connectivity_qualified", false));
            for (boolean entity : new boolean[]{false, true}) for (boolean request : new boolean[]{false, true}) {
                probe("admission-entity-" + (entity ? 1 : 0) + "-request-" + (request ? 1 : 0), entity, request, false);
            }
            probe("connected-no-entity", false, true, true);
            probe("connected-no-request", true, false, true);
            probe("connected-admitted", true, true, true);
            require(caseCount == 7, "complete original finite case set");
        } catch (Throwable error) { failed = target(error); }
        finally {
            try { closeThread(); }
            catch (Throwable error) { if (failed == null) failed = target(error); }
        }
        if (failed != null) throw failed;
        emit("connected-summary", new JSONObject().put("record", "summary").put("cases", caseCount)
            .put("admission_cases", 4).put("connected_cases", 3).put("authored_JSON_records_including_summary", 10)
            .put("harness_invariants_passed", true).put("original_thread_closed", true)
            .put("connected_only", true).put("disconnected_loss_qualified", false)
            .put("full_wire_callback_chain_qualified", false).put("phone_accessed", false));
        require(System.nanoTime() < cleanupUntil, "original oracle completion cutoff");
    }
}
