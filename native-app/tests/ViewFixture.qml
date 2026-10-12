import QtQuick
import QtQuick.Controls
import ".."
Item {
    id: root
    width: 390; height: 844
    property QtObject injectedSession: null
    property QtObject injectedActions: null
    property QtObject injectedTheme: null
    signal actionSubmitted(string action)
    signal recoveryRequested(string operation, string key)
    property alias fixtureSession: fixtureSession
    property alias fixtureActions: fixtureModel
    property alias fixtureTheme: fixturePalette
    QtObject {
        id: fixtureSession
        property bool ready: true
        property bool needsRecovery: false
        property bool canAcknowledge: false
        property string phase: "ready"
        property string epoch: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        property string sessionToken: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        property string admissionToken: "cccccccccccccccccccccccccccccccc"
        property string recoveryKey: ""
        property string requestId: ""
        property bool suspended: false
        property var recoveryItems: []
        function acknowledge(key) { root.recoveryRequested("session_ack", key); return true }
        function refresh() { root.recoveryRequested("refresh", ""); return true }
    }
    QtObject {
        id: fixtureModel
        property bool canObserve: true
        property bool canDial: false
        property bool unresolved: false
        property bool acknowledged: false
        property string phase: "idle"
        property string outcome: ""
        property string intentId: ""
        property string intentEpoch: ""
        property var job: null
        function acknowledge(key) { root.recoveryRequested("intent_ack", key); return true }
        function recover() { root.recoveryRequested("recover", ""); return true }
        property var controlsSnapshot: ({generation: "dddddddddddddddddddddddddddddddd", revision: 7, controls_allowed: true, calls: [{id: 1, state: 2, actions: ["answer", "reject"]}]})
        property var dialSnapshot: null
        function fresh(kind) { return true }
        function control(id, action) { root.actionSubmitted(action); return true }
        function dial(number) { root.actionSubmitted("dial"); return true }
    }
    QtObject {
        id: fixturePalette
        property color background: "#1b1d24"
        property color surface: "#252832"
        property color foreground: "#e8e3d5"
        property color muted: "#b4b0a6"
        property color border: "#454852"
        property color accent: "#e0af68"
        property color urgent: "#f7768e"
        property color positive: "#9ece6a"
        property color ink: "#161821"
        property string fontFamily: "DejaVu Sans"
        property string monoFamily: "DejaVu Sans Mono"
        property real fontScale: 1
    }
    Rectangle { anchors.fill: parent; color: fixturePalette.background }
    PhoneView { anchors.fill: parent; anchors.bottomMargin: 28; session: root.injectedSession || fixtureSession; actions: root.injectedActions || fixtureModel; theme: root.injectedTheme || fixturePalette }
    Label {
        anchors.bottom: parent.bottom; anchors.horizontalCenter: parent.horizontalCenter
        height: 28; text: "Synthetic call · native QML test"; color: fixturePalette.muted; font.pixelSize: 12
    }
}
