import QtQuick

// UI state only. The daemon rechecks authority and owns durable command results.
QtObject {
    id: root
    property int barrier: 0
    property var observation: null
    property bool observationLive: false
    property var pending: null
    property string lastEpoch: ""
    property double lastSequence: -1
    property string lastOutcome: ""
    property string lastReason: ""
    readonly property var value: observation ? observation.controls.value : null
    readonly property var call: value && value.calls.length === 1 ? value.calls[0] : null
    readonly property bool available: observationLive && !!observation && observation.controls.fresh === true
        && !!value && value.availability === "observed" && !!call
    readonly property bool authorized: !!value && value.controls_allowed === true
    readonly property int callState: call ? call.state : 0
    readonly property var actions: call ? call.actions : []
    readonly property double revision: value ? value.revision : 0
    readonly property bool busy: pending !== null
    readonly property bool uncertain: pending !== null && pending.status === "unknown"
    readonly property string pendingAction: pending ? pending.command.action : ""
    readonly property string contextKey: available ? JSON.stringify([
        observation.epoch, value.generation, value.revision, call.id, call.state]) : ""
    signal dispatchRequested(var request)

    function hex(value) { return typeof value === "string" && /^[0-9a-f]{32}$/.test(value); }
    function integer(value, minimum, maximum) {
        return typeof value === "number" && isFinite(value) && Math.floor(value) === value
            && value >= minimum && value <= maximum;
    }
    function clone(value) { return JSON.parse(JSON.stringify(value)); }
    function reset() {
        barrier++; observation=null; observationLive=false; pending=null; lastEpoch=""; lastSequence=-1;
        lastOutcome=""; lastReason="";
    }
    function expire() { barrier++; observationLive=false; }
    function validSnapshot(snapshot) {
        if (!snapshot || !hex(snapshot.epoch) || !integer(snapshot.sequence,0,9007199254740991)) return false;
        var lane=snapshot.controls;
        if (!lane || typeof lane.fresh !== "boolean") return false;
        var v=lane.value;
        if (v === null) return lane.fresh === false;
        if (!v || !hex(v.generation) || !integer(v.revision,1,9007199254740991)
            || typeof v.controls_allowed !== "boolean" || !Array.isArray(v.calls)
            || ["observed","unavailable"].indexOf(v.availability)<0 || v.calls.length>64) return false;
        for (var i=0;i<v.calls.length;i++) {
            var c=v.calls[i];
            if (!c || !integer(c.id,1,2147483647) || !integer(c.state,0,20) || !Array.isArray(c.actions)) return false;
            for (var j=0;j<c.actions.length;j++)
                if (["answer","reject","disconnect"].indexOf(c.actions[j])<0) return false;
        }
        return true;
    }
    function acceptSnapshot(reply, ticket) {
        if (ticket !== barrier) return false;
        // QML var signals can wrap nested JSON arrays as QVariant sequences.
        // Normalize before Array.isArray validation, not only before storage.
        try { reply=clone(reply); }
        catch (error) { expire(); return false; }
        if (!reply || reply.available !== true || !validSnapshot(reply.snapshot)) { expire(); return false; }
        var next=reply.snapshot;
        if (next.epoch === lastEpoch && next.sequence <= lastSequence) return false;
        if (lastEpoch !== "" && next.epoch !== lastEpoch) {
            barrier++;
            if (pending) { var unresolved=clone(pending); unresolved.status="unknown"; pending=unresolved; }
        }
        lastEpoch=next.epoch; lastSequence=next.sequence;
        observation=clone(next); observationLive=true;
        return true;
    }
    function requestAction(action, observedContext, identity) {
        if (!available || !authorized || busy || !hex(identity) || observedContext !== contextKey
            || actions.indexOf(action)<0) return false;
        if ((action === "answer" || action === "reject") ? callState !== 2
            : action !== "disconnect" || [1,3,4,9].indexOf(callState)<0) return false;
        var request={epoch:observation.epoch,id:identity,command:{generation:value.generation,
            call_id:call.id,expected_revision:value.revision,expected_state:call.state,action:action}};
        pending={epoch:request.epoch,id:identity,status:"pending",command:clone(request.command)};
        lastOutcome=""; lastReason="";
        expire(); // Every earlier observation process is now obsolete.
        dispatchRequested(clone(request));
        return true;
    }
    function acceptOperation(reply) {
        if (!pending || !reply || reply.epoch !== pending.epoch || reply.id !== pending.id) return false;
        var operation=reply.operation;
        if (reply.available === true && operation && operation.state === "finished") {
            var result=operation.result;
            if (!result || ["submitted","rejected","unknown"].indexOf(result.outcome)<0
                || typeof result.reason !== "string" || !/^[a-z_]{1,64}$/.test(result.reason)) return false;
            lastOutcome=result.outcome; lastReason=result.reason;
            if (result.outcome !== "unknown") { pending=null; expire(); return true; }
        } else if (reply.available === true && operation && operation.state === "pending") {
            return true;
        } else if (reply.available !== false && !(reply.available === true
                   && (operation === null || (operation && operation.state === "unknown")))) {
            return false;
        }
        var unresolved=clone(pending); unresolved.status="unknown"; pending=unresolved;
        expire();
        return true; // Only a later matching saved result can resolve this uncertainty.
    }
}
