import QtQml

// One instance at app scope. Observe host sleep; never request or inhibit it.
QtObject {
    id: root
    required property QtObject power
    required property QtObject session
    required property QtObject actions
    required property QtObject rpc
    property bool resumePending: false

    function update() {
        const blocked = !power.available || power.preparing
        session.suspended = blocked
        actions.invalidate()
        resumePending = !blocked
        if (!blocked) Qt.callLater(pump)
    }
    function pump() {
        if (!resumePending || !power.available || power.preparing || rpc.busy || actions.job) return
        // A completion may be queued after busy=false. Let it settle the intent
        // before selecting recovery; never replace or replay the command.
        if (actions.phase === "submitting" || actions.phase === "recovering") return
        const started = actions.unresolved ? actions.recover() : session.refresh()
        if (started) resumePending = false
    }
    property Timer receipts: Timer {
        interval: 500
        repeat: true
        running: root.power.available && !root.power.preparing && !root.session.suspended
            && (root.session.ready || root.session.needsRecovery) && root.actions.phase === "pending"
        onTriggered: {
            if (root.rpc.busy || root.actions.job || root.session.requestId !== "") return
            root.actions.lookup()
        }
    }
    property Timer reconnect: Timer {
        interval: 3000
        repeat: true
        running: root.power.available && !root.power.preparing
        onTriggered: {
            if (root.session.phase !== "unavailable") return
            root.resumePending = true
            root.pump()
        }
    }
    Component.onCompleted: update()
    property Connections powerChanges: Connections {
        target: root.power
        function onChanged() { root.update() }
    }
    property Connections completions: Connections {
        target: root.rpc
        function onCompleted(status, bytes, token) { Qt.callLater(root.pump) }
    }
    property Connections actionChanges: Connections {
        target: root.actions
        function onPhaseChanged() { Qt.callLater(root.pump) }
    }
}
