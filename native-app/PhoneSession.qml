import QtQml

// Current-protocol app session. Reads never replay or replace a command.
QtObject {
    id: root
    required property QtObject rpc
    required property string socketPath
    signal barrierEstablished(string epoch, string admission)
    property bool suspended: false
    onSuspendedChanged: if (suspended) unavailable()
    property string phase: "unavailable"
    property string epoch: ""
    property string sessionToken: ""
    property string admissionToken: ""
    property var recoveryItems: []
    property string requestId: ""
    property bool checkingHealth: false
    property string acknowledgementTarget: ""
    readonly property bool ready: phase === "ready" && !suspended
    readonly property bool needsRecovery: phase === "recovery" && !suspended
    readonly property bool canAcknowledge: needsRecovery && !rpc.busy && requestId === ""
        && recoveryItems.length === 1 && recoveryItems[0].outcome === "unknown"
    readonly property string recoveryKey: needsRecovery && recoveryItems.length === 1
        ? JSON.stringify([epoch, recoveryItems[0].id, recoveryItems[0].outcome]) : ""

    function unavailable() {
        phase = "unavailable"
        requestId = ""
        checkingHealth = false
        epoch = ""
        sessionToken = ""
        admissionToken = ""
        acknowledgementTarget = ""
    }
    function send(op, extra, target) {
        const identity = rpc.newId()
        const request = {v: 1, id: identity, op: op}
        if (extra) request.epoch = extra
        if (op === "fence") request.session = sessionToken
        if (target) request.target = target
        requestId = identity
        if (!rpc.request(socketPath, JSON.stringify(request) + "\n", identity)) {
            unavailable()
            return false
        }
        return true
    }
    function checkHealth() {
        if (suspended || rpc.busy || requestId !== "" || (!ready && !needsRecovery)) return false
        checkingHealth = true
        return send("hello", "")
    }
    function refresh() {
        if (suspended || rpc.busy || requestId !== "") return false
        admissionToken = ""
        phase = "hello"
        return send("hello", "")
    }
    function acknowledge(observedKey) {
        if (!canAcknowledge || observedKey !== recoveryKey) return false
        acknowledgementTarget = recoveryItems[0].id
        phase = "acknowledging"
        return send("acknowledge", epoch, acknowledgementTarget)
    }
    property Timer health: Timer {
        interval: 2000
        repeat: true
        running: root.ready || root.needsRecovery
        onTriggered: root.checkHealth()
    }
    property Connections replies: Connections {
        target: root.rpc
        function onCompleted(status, bytes, token) {
            if (root.requestId === "" || token !== root.requestId) return
            if (status !== 0) { root.unavailable(); return }
            let reply
            try { reply = JSON.parse(String(bytes)) }
            catch (error) { root.unavailable(); return }
            if (!reply || reply.id !== root.requestId) { root.unavailable(); return }
            root.requestId = ""
            if (root.checkingHealth) {
                root.checkingHealth = false
                if (!reply.result || reply.result.kind !== "hello" || reply.result.epoch !== root.epoch
                        || reply.result.session !== root.sessionToken) root.unavailable()
            } else if (root.phase === "hello" && reply.result.kind === "hello") {
                root.epoch = reply.result.epoch
                root.sessionToken = reply.result.session
                root.phase = "establishing_barrier"
                root.send("fence", root.epoch)
            } else if (root.phase === "establishing_barrier" && reply.result.kind === "fence"
                       && reply.result.epoch === root.epoch
                       && reply.result.session === root.sessionToken) {
                root.admissionToken = reply.result.admission
                root.recoveryItems = reply.result.items
                root.phase = root.recoveryItems.length === 0 ? "ready" : "recovery"
                root.barrierEstablished(root.epoch, root.admissionToken)
            } else if (root.phase === "acknowledging" && reply.result.kind === "acknowledged"
                       && reply.result.epoch === root.epoch
                       && reply.result.target === root.acknowledgementTarget) {
                root.acknowledgementTarget = ""
                root.phase = "establishing_barrier"
                root.send("fence", root.epoch)
            } else {
                root.unavailable()
            }
        }
    }
}
