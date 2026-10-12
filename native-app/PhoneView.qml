import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Presentation consumes native app state; it never dispatches outside actions.
Item {
    id: root
    required property QtObject session
    required property QtObject actions
    required property QtObject theme
    property bool keypadVisible: false
    readonly property var snapshot: actions.controlsSnapshot
    readonly property var call: snapshot && snapshot.calls.length === 1 ? snapshot.calls[0] : null
    readonly property string callKey: call ? JSON.stringify([session.epoch, session.sessionToken,
        session.admissionToken, snapshot.generation, snapshot.revision, call.id, call.state]) : ""
    readonly property string callTitle: !snapshot ? "Call status unavailable"
        : !call ? (snapshot.calls.length ? "Multiple calls" : "No observed call")
        : call.state === 2 ? "Incoming call" : call.state === 4 ? "Call in progress"
        : call.state === 3 ? "Call on hold" : call.state === 7 ? "Call ended" : "Connecting call"
    readonly property bool needsReview: session.needsRecovery
        || (actions.unresolved && actions.outcome === "unknown")
    readonly property bool canAcknowledgeReview: session.canAcknowledge && (!actions.unresolved
        || (actions.intentEpoch === session.epoch && session.recoveryItems.length === 1
            && session.recoveryItems[0].id === actions.intentId))
    readonly property string reviewContext: JSON.stringify([session.epoch, session.sessionToken,
        session.recoveryKey, actions.intentEpoch, actions.intentId, canAcknowledgeReview])
    function review() {
        if (canAcknowledgeReview) {
            if (actions.unresolved) actions.acknowledge(session.recoveryKey)
            else session.acknowledge(session.recoveryKey)
        } else if (actions.unresolved) actions.recover()
        else session.refresh()
    }
    function permits(action) {
        return call && actions.canObserve && actions.fresh("controls")
            && snapshot.controls_allowed && call.actions.indexOf(action) >= 0
    }
    function insertDigit(digit) {
        const at = number.selectionStart !== number.selectionEnd ? number.selectionStart : number.cursorPosition
        if (number.selectionStart !== number.selectionEnd) number.remove(number.selectionStart, number.selectionEnd)
        number.insert(at, digit)
        number.forceActiveFocus()
    }
    Rectangle { anchors.fill: parent; color: root.theme.background }
    ScrollView {
        anchors.fill: parent; anchors.margins: 20
        contentWidth: availableWidth
        clip: true
        ColumnLayout {
            width: parent.width
            spacing: 20
            Label {
                text: "Phone"; color: root.theme.foreground
                font.family: root.theme.monoFamily; font.pixelSize: Math.max(1, Math.round(30 * root.theme.fontScale)); font.bold: true
                Layout.fillWidth: true
            }
            Label {
                text: root.session.ready ? "Bridge connected" : root.session.needsRecovery ? "Review required" : "Bridge unavailable"
                color: root.theme.muted; font.family: root.theme.fontFamily; font.pixelSize: Math.max(1, Math.round(15 * root.theme.fontScale))
                Layout.fillWidth: true; wrapMode: Text.Wrap
            }
            Frame {
                Layout.fillWidth: true; padding: 20
                background: Rectangle { color: root.theme.surface; radius: 4; border.color: root.theme.border }
                ColumnLayout {
                    anchors.fill: parent; spacing: 16
                    Label {
                        text: root.callTitle; color: root.theme.foreground
                        font.family: root.theme.fontFamily; font.pixelSize: Math.max(1, Math.round(25 * root.theme.fontScale)); font.bold: true
                        Layout.fillWidth: true; wrapMode: Text.Wrap
                    }
                    Label {
                        text: "Caller ID unavailable"; visible: root.call !== null
                        color: root.theme.muted; font.family: root.theme.fontFamily; font.pixelSize: Math.max(1, Math.round(16 * root.theme.fontScale))
                        Layout.fillWidth: true; wrapMode: Text.Wrap
                    }
                    Label {
                        visible: root.actions.unresolved || root.session.needsRecovery
                        text: root.actions.outcome === "unknown" || root.session.needsRecovery
                            ? "An earlier action may have taken effect. Review its result before continuing."
                            : "Waiting for the command result…"
                        color: root.theme.accent; font.family: root.theme.fontFamily; font.pixelSize: Math.max(1, Math.round(16 * root.theme.fontScale))
                        Layout.fillWidth: true; wrapMode: Text.Wrap
                    }
                    IntentButton {
                        objectName: "recoveryButton"; theme: root.theme
                        text: root.canAcknowledgeReview ? "Acknowledge uncertainty" : "Check result"
                        Layout.fillWidth: true; visible: root.needsReview
                        enabled: !root.session.suspended && root.session.requestId === "" && !root.actions.job
                            && root.actions.phase !== "submitting" && root.actions.phase !== "recovering"
                        fill: theme.accent; ink: theme.ink; contextKey: root.reviewContext
                        onInvoked: root.review()
                    }
                    IntentButton {
                        objectName: "answerButton"; theme: root.theme
                        text: "Answer"; fill: theme.positive; ink: theme.ink
                        Layout.fillWidth: true; visible: root.call && root.call.state === 2
                        enabled: root.permits("answer"); contextKey: root.callKey
                        onInvoked: root.actions.control(root.call.id, "answer")
                    }
                    IntentButton {
                        objectName: "declineButton"; theme: root.theme
                        text: "Decline"; fill: theme.urgent; ink: theme.ink
                        Layout.fillWidth: true; visible: root.call && root.call.state === 2
                        enabled: root.permits("reject"); contextKey: root.callKey
                        onInvoked: root.actions.control(root.call.id, "reject")
                    }
                    IntentButton {
                        objectName: "endButton"; theme: root.theme
                        text: "End call"; fill: theme.urgent; ink: theme.ink
                        Layout.fillWidth: true; visible: root.call && [1,3,4,9].indexOf(root.call.state) >= 0
                        enabled: root.permits("disconnect"); contextKey: root.callKey
                        onInvoked: root.actions.control(root.call.id, "disconnect")
                    }
                }
            }
            ColumnLayout {
                Layout.fillWidth: true; spacing: 12
                Label { text: "Number"; color: root.theme.foreground; font.family: root.theme.fontFamily; font.pixelSize: Math.max(1, Math.round(16 * root.theme.fontScale)) }
                TextField {
                    id: number; objectName: "numberField"
                    Layout.fillWidth: true; implicitHeight: 56
                    font.family: root.theme.monoFamily; font.pixelSize: Math.max(1, Math.round(24 * root.theme.fontScale)); color: root.theme.foreground
                    Accessible.name: "Phone number"
                    placeholderText: "Phone number"; placeholderTextColor: root.theme.muted
                    inputMethodHints: Qt.ImhDialableCharactersOnly
                    maximumLength: 33; selectByMouse: true
                    validator: RegularExpressionValidator { regularExpression: /\+?[0-9]{3,32}/ }
                    background: Rectangle { color: root.theme.surface; radius: 4; border.color: number.activeFocus ? root.theme.accent : root.theme.border }
                }
                RowLayout {
                    Layout.fillWidth: true; spacing: 12
                    IntentButton {
                        theme: root.theme; text: root.keypadVisible ? "Hide keypad" : "Keypad"
                        contextKey: "keypad"; Layout.fillWidth: true
                        onInvoked: root.keypadVisible = !root.keypadVisible
                    }
                    IntentButton {
                        objectName: "dialButton"; theme: root.theme; text: "Call"
                        fill: theme.accent; ink: theme.ink; Layout.fillWidth: true
                        enabled: root.actions.canDial && number.acceptableInput
                        contextKey: JSON.stringify([root.session.admissionToken, root.actions.dialSnapshot, number.text])
                        onInvoked: root.actions.dial(number.text)
                    }
                }
                GridLayout {
                    visible: root.keypadVisible; columns: 3; rowSpacing: 8; columnSpacing: 8
                    Layout.fillWidth: true
                    Repeater {
                        model: ["1","2","3","4","5","6","7","8","9","+","0","Delete"]
                        IntentButton {
                            required property string modelData
                            theme: root.theme; text: modelData; contextKey: "digit:" + modelData
                            Layout.fillWidth: true
                            onInvoked: {
                                if (modelData !== "Delete") root.insertDigit(modelData)
                                else if (number.selectionStart !== number.selectionEnd) number.remove(number.selectionStart, number.selectionEnd)
                                else if (number.cursorPosition > 0) number.remove(number.cursorPosition - 1, number.cursorPosition)
                            }
                        }
                    }
                }
            }
        }
    }
}
