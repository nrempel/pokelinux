import QtQuick
import QtQuick.Layouts

// Presentation only. The service owns authority, admission and durable results.
Rectangle {
    id: root
    property bool available: false
    property bool authorized: false
    property int callState: 0
    property var actions: []
    property bool busy: false
    property bool uncertain: false
    property string pendingAction: ""
    property string recoveryKey: ""
    property string recoveryMessage: ""
    property bool canAcknowledge: false
    signal acknowledgmentRequested(string observedKey)
    // Controller binds this to service epoch, generation, call ID and revision.
    property string contextKey: ""
    property color surfaceColor: "#1b2026"
    property color foregroundColor: "#e6e9ef"
    property color mutedColor: "#b7bec9"
    property color borderColor: "#3c454f"
    property color answerColor: "#9ece6a"
    property color declineColor: "#f7768e"
    property color buttonInk: "#101315"
    signal actionRequested(string action, string observedContext)
    readonly property bool ringing: callState === 2
    readonly property bool canAct: available && authorized && !busy && !uncertain && contextKey.length > 0 && recoveryKey.length===0
    readonly property string title: recoveryKey.length>0 ? (available ? (ringing ? "Incoming call" : callState===4 ? "Call in progress" : "Phone") : "Phone")
        : uncertain ? "Call status unknown"
        : busy ? (pendingAction === "answer" ? "Answering…" : pendingAction === "reject" ? "Declining…" : "Ending call…")
        : !available ? "Phone unavailable"
        : ringing ? "Incoming call" : callState === 4 ? "Call in progress"
        : callState === 3 ? "Call on hold" : callState === 0 ? "No current call" : "Call"
    readonly property string detail: recoveryKey.length>0 ? recoveryMessage
        : uncertain ? "The command result could not be confirmed."
        : busy ? ""
        : !available ? "Waiting for fresh phone status."
        : !authorized ? "Call controls are disabled on Android."
        : ringing ? "Caller ID unavailable" : ""
    function permits(action) {
        if (!canAct || !Array.isArray(actions) || actions.indexOf(action) < 0) return false;
        if (action === "answer" || action === "reject") return ringing;
        return action === "disconnect" && [1, 3, 4, 9].indexOf(callState) >= 0;
    }
    function requestAction(action, observedContext) {
        if (observedContext === contextKey && permits(action)) actionRequested(action, observedContext);
    }
    implicitWidth: 380
    implicitHeight: body.implicitHeight + 48
    radius: 8
    color: surfaceColor
    border.width: 1
    border.color: borderColor
    ColumnLayout {
        id: body
        anchors.fill: parent
        anchors.margins: 24
        spacing: 16
        Text {
            Layout.fillWidth: true
            text: root.title
            textFormat: Text.PlainText
            color: root.foregroundColor
            font.pixelSize: 28
            font.weight: Font.DemiBold
            wrapMode: Text.Wrap
            Accessible.role: Accessible.StaticText
            Accessible.name: text
        }
        Text {
            Layout.fillWidth: true
            visible: text.length > 0
            text: root.detail
            textFormat: Text.PlainText
            color: root.mutedColor
            font.pixelSize: 18
            wrapMode: Text.Wrap
        }
        Item { Layout.preferredHeight: 16 }
        CallActionButton {
            objectName: "acknowledgeButton"
            Layout.fillWidth: true
            visible: root.recoveryKey.length>0
            enabled: root.canAcknowledge
            text: "Acknowledge uncertainty"
            Accessible.name: text
            actionColor: "#e0af68"
            inkColor: root.buttonInk
            contextKey: root.recoveryKey
            onActionTriggered: function(key) {
                if (root.canAcknowledge && key===root.recoveryKey) root.acknowledgmentRequested(key);
            }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 16
            visible: root.ringing && root.recoveryKey.length===0
            CallActionButton {
                objectName: "answerButton"
                Layout.fillWidth: true
                text: "Answer"
                Accessible.name: "Answer call"
                actionColor: root.answerColor
                inkColor: root.buttonInk
                enabled: root.permits("answer")
                contextKey: root.contextKey
                onActionTriggered: function(observedContext) { root.requestAction("answer", observedContext); }
            }
            CallActionButton {
                objectName: "declineButton"
                Layout.fillWidth: true
                text: "Decline"
                Accessible.name: "Decline call"
                actionColor: root.declineColor
                inkColor: root.buttonInk
                enabled: root.permits("reject")
                contextKey: root.contextKey
                onActionTriggered: function(observedContext) { root.requestAction("reject", observedContext); }
            }
        }
        CallActionButton {
            objectName: "endButton"
            Layout.fillWidth: true
            text: "End call"
            Accessible.name: "End call"
            visible: !root.ringing && root.callState !== 0 && root.recoveryKey.length===0
            actionColor: root.declineColor
            inkColor: root.buttonInk
            enabled: root.permits("disconnect")
            contextKey: root.contextKey
            onActionTriggered: function(observedContext) { root.requestAction("disconnect", observedContext); }
        }
    }
}
