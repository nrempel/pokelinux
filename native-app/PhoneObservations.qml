import QtQml

// Open-app observations only. Background ringing belongs to the native service.
QtObject {
    id: root
    required property QtObject session
    required property QtObject actions
    required property bool active
    property string nextKind: "controls"
    property bool initialized: false
    function poll() {
        if (!active || !session.ready || !actions.canObserve) return
        if (actions.observe(nextKind)) nextKind = nextKind === "controls" ? "dial_status" : "controls"
    }
    function restart() {
        nextKind = "controls"
        if (active) Qt.callLater(poll)
        else actions.invalidate()
    }
    onActiveChanged: if (initialized) restart()
    Component.onCompleted: { initialized = true; restart() }
    property Timer observations: Timer {
        interval: 500
        repeat: true
        running: root.active && root.session.ready && !root.actions.unresolved
        onTriggered: root.poll()
    }
    property Connections sessionChanges: Connections {
        target: root.session
        function onReadyChanged() { if (root.session.ready) root.restart() }
    }
}
