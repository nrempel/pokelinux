import QtQml

// Action state belongs to the app lifetime, not to a page or button delegate.
QtObject {
    id: root
    required property QtObject rpc
    required property QtObject session
    property string phase: "idle"
    property string outcome: ""
    property string intentId: ""
    property string intentEpoch: ""
    property string intentKind: ""
    property bool hadReceipt: false
    property bool acknowledged: false
    property var controlsSnapshot: null
    property var dialSnapshot: null
    property double controlsStarted: -1
    property double dialStarted: -1
    property int observationRevision: 0
    property var job: null
    readonly property bool unresolved: phase === "submitting" || phase === "pending" || phase === "unknown" || phase === "recovering"
    readonly property bool canObserve: session.ready && session.requestId === "" && !rpc.busy && !job && !unresolved
    readonly property bool canDial: canObserve && fresh("dial_status") && dialSnapshot.can_dial

    function clearObservation(kind) {
        if (kind === "controls") { controlsSnapshot = null; controlsStarted = -1 }
        else { dialSnapshot = null; dialStarted = -1 }
    }
    function invalidate() {
        observationRevision++
        clearObservation("controls")
        clearObservation("dial_status")
    }
    function fresh(kind) {
        const snapshot = kind === "controls" ? controlsSnapshot : dialSnapshot
        const started = kind === "controls" ? controlsStarted : dialStarted
        if (!snapshot || started < 0) return false
        const age = rpc.boottimeMs() - started
        return age >= 0 && age <= 2000
    }
    function issue(request, kind, extra) {
        const token = rpc.newId()
        job = {token: token, id: request.id, kind: kind, extra: extra}
        if (!rpc.request(session.socketPath, JSON.stringify(request) + "\n", token)) {
            job = null
            return false
        }
        return true
    }
    function observe(kind) {
        if (!canObserve || (kind !== "controls" && kind !== "dial_status")) return false
        if (!fresh(kind)) clearObservation(kind)
        const started = rpc.boottimeMs()
        if (started < 0) return false
        return issue({v: 1, id: rpc.newId(), op: kind}, kind, {
            started: started, revision: observationRevision,
            epoch: session.epoch, session: session.sessionToken, admission: session.admissionToken
        })
    }
    function control(callId, action) {
        if (!canObserve || !fresh("controls")
                || !controlsSnapshot.controls_allowed || controlsSnapshot.calls.length !== 1) return false
        const call = controlsSnapshot.calls[0]
        if (call.id !== callId || call.actions.indexOf(action) < 0) return false
        const states = {1: "dialing", 2: "ringing", 3: "held", 4: "active", 9: "connecting"}
        if (!states[call.state]) return false
        return submit({kind: "control", generation: controlsSnapshot.generation,
            call_id: call.id, revision: controlsSnapshot.revision, state: states[call.state], action: action})
    }
    function dial(number) {
        // Read the clock in the action path even if no QML binding/timer ran on resume.
        if (!canDial || !fresh("dial_status") || !/^\+?[0-9]{3,32}$/.test(number)) return false
        return submit({kind: "dial", generation: dialSnapshot.generation,
            offer: dialSnapshot.offer, number: number})
    }
    function submit(command) {
        if (!canObserve || !fresh(command.kind === "dial" ? "dial_status" : "controls")) return false
        // Claim the guard and copy all authority before starting an asynchronous job.
        phase = "submitting"
        outcome = ""
        intentId = rpc.newId()
        intentEpoch = session.epoch
        intentKind = command.kind
        hadReceipt = false
        acknowledged = false
        const request = {v: 1, id: intentId, op: "submit", epoch: intentEpoch,
            session: session.sessionToken, admission: session.admissionToken, command: command}
        invalidate()
        if (!issue(request, "submit", null)) {
            settle("not_sent")
            return false
        }
        return true
    }
    function acknowledge(key) {
        if (phase !== "unknown" || rpc.busy || job || session.epoch !== intentEpoch
                || !session.canAcknowledge || key !== session.recoveryKey
                || session.recoveryItems[0].id !== intentId) return false
        phase = "recovering"
        if (!session.acknowledge(key)) {
            settle("unknown")
            return false
        }
        return true
    }
    function recover() {
        if (rpc.busy || job || (phase !== "pending" && phase !== "unknown")) return false
        phase = "recovering"
        invalidate()
        if (!session.refresh()) {
            settle("unknown")
            return false
        }
        return true
    }
    function lookup() {
        if (rpc.busy || job || (phase !== "pending" && phase !== "unknown")) return false
        return issue({v: 1, id: rpc.newId(), op: "lookup", epoch: intentEpoch, target: intentId}, "lookup", null)
    }
    function settle(value) {
        outcome = value
        phase = value === "pending" ? "pending" : value === "unknown" ? "unknown" : "terminal"
    }
    property Timer expiry: Timer {
        interval: 250
        repeat: true
        running: root.controlsSnapshot !== null || root.dialSnapshot !== null
        onTriggered: {
            if (!root.fresh("controls")) root.clearObservation("controls")
            if (!root.fresh("dial_status")) root.clearObservation("dial_status")
        }
    }
    property Connections sessionChanges: Connections {
        target: root.session
        function onBarrierEstablished(epoch, admission) {
            if (root.phase !== "recovering") return
            if (epoch !== root.intentEpoch || admission === "") { root.settle("unknown"); return }
            for (const item of root.session.recoveryItems) {
                if (item.id === root.intentId) root.hadReceipt = true
            }
            if (!root.issue({v: 1, id: root.rpc.newId(), op: "lookup", epoch: root.intentEpoch,
                    target: root.intentId}, "recovery_lookup", null)) root.settle("unknown")
        }
        function onPhaseChanged() {
            if (root.phase === "recovering" && root.session.phase === "unavailable") root.settle("unknown")
        }
        function onReadyChanged() { root.invalidate() }
        function onEpochChanged() { root.invalidate() }
        function onSessionTokenChanged() { root.invalidate() }
        function onAdmissionTokenChanged() { root.invalidate() }
    }
    property Connections replies: Connections {
        target: root.rpc
        function onCompleted(status, bytes, token) {
            if (!root.job || token !== root.job.token) return
            const completed = root.job
            root.job = null
            if (status !== 0) {
                if (completed.kind === "controls" || completed.kind === "dial_status") root.clearObservation(completed.kind)
                if (completed.kind === "submit") root.settle(status === 2 ? "not_sent" : "unknown")
                else if (completed.kind === "recovery_lookup") root.settle("unknown")
                return
            }
            let reply
            try { reply = JSON.parse(String(bytes)) } catch (error) {
                if (completed.kind === "submit" || completed.kind === "recovery_lookup") root.settle("unknown")
                return
            }
            if (!reply || reply.id !== completed.id || !reply.result) {
                if (completed.kind === "submit" || completed.kind === "recovery_lookup") root.settle("unknown")
                return
            }
            const result = reply.result
            if (completed.kind === "submit") {
                root.hadReceipt = result.kind === "submission"
                root.settle(result.kind === "submission" ? result.outcome : "unknown")
            } else if (completed.kind === "lookup" || completed.kind === "recovery_lookup") {
                if (result.kind !== "receipt" || result.epoch !== root.intentEpoch || result.target !== root.intentId) {
                    if (completed.kind === "recovery_lookup") root.settle("unknown")
                    return
                }
                if (result.receipt) {
                    root.hadReceipt = true
                    root.acknowledged = result.receipt.acknowledged
                    root.settle(result.receipt.outcome)
                    if (root.acknowledged && result.receipt.outcome === "unknown") root.phase = "terminal"
                } else {
                    // Only the post-barrier read can exclude a delayed unadmitted
                    // command. A previously observed receipt must never vanish.
                    root.settle(completed.kind === "recovery_lookup" && !root.hadReceipt ? "not_sent" : "unknown")
                }
            } else if (result.kind === completed.kind) {
                const context = completed.extra
                const age = root.rpc.boottimeMs() - context.started
                if (!root.session.ready || context.revision !== root.observationRevision
                        || context.epoch !== root.session.epoch || context.session !== root.session.sessionToken
                        || context.admission !== root.session.admissionToken || age < 0 || age > 2000) return
                const other = completed.kind === "controls" ? root.dialSnapshot : root.controlsSnapshot
                if (other && other.generation !== result.snapshot.generation) root.invalidate()
                if (completed.kind === "controls") {
                    root.controlsStarted = context.started
                    root.controlsSnapshot = result.snapshot
                } else {
                    root.dialStarted = context.started
                    root.dialSnapshot = result.snapshot
                }
            }
        }
    }
}
