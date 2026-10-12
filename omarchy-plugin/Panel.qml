import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.Commons as Commons
import "qml"

Item {
    id: root
    property var shell: null
    property var service: null
    property bool opened: false
    readonly property var controller: service ? service.controller : null
    readonly property bool observedIdle: !!controller && controller.observationLive
        && !!controller.observation && controller.observation.controls.fresh
        && !!controller.value && controller.value.calls.length===0
    function status() {
        return JSON.stringify({opened:opened,available:card.available,callState:card.callState,
            busy:card.busy,uncertain:card.uncertain,title:card.title,
            canAcknowledge:card.canAcknowledge,recoveryKey:card.recoveryKey,
            recoveryCount:service ? service.recoveryItems.length : 0});
    }
    function open(payloadJson) { opened=true; }
    function close() { opened=false; }
    function requestClose() {
        if (shell) shell.hide("nbrempel.phone");
        else close();
    }
    PanelWindow {
        id: window
        visible: root.opened
        implicitWidth: Math.min(420, screen ? screen.width : 420)
        implicitHeight: content.implicitHeight+32
        color: "transparent"
        exclusiveZone: 0
        WlrLayershell.namespace: "nbrempel-phone"
        WlrLayershell.layer: WlrLayer.Top
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
        ColumnLayout {
            id: content
            anchors.fill: parent
            anchors.margins: 16
            spacing: 12
            Button {
                id: closeButton
                Layout.alignment: Qt.AlignRight
                text: "Close"
                implicitWidth: 88
                implicitHeight: 44
                background: Rectangle {
                    color: closeButton.down ? Commons.Color.muted : Commons.Color.menu.background
                    border.color: Commons.Color.menu.border
                    border.width: closeButton.activeFocus ? 2 : 1
                    radius: 4
                }
                contentItem: Text {
                    text: closeButton.text
                    color: Commons.Color.menu.text
                    font.pixelSize: 16
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                Accessible.name: "Close phone panel"
                onClicked: root.requestClose()
            }
            CallCard {
                id: card
                Layout.fillWidth: true
                available: root.observedIdle || (!!root.controller && root.controller.available)
                authorized: root.controller ? root.controller.authorized : false
                callState: root.controller ? root.controller.callState : 0
                actions: root.controller ? root.controller.actions : []
                busy: root.service ? root.service.busy : false
                uncertain: root.controller ? root.controller.uncertain : false
                pendingAction: root.service ? root.service.pendingAction : ""
                recoveryKey: root.service ? root.service.recoveryKey : ""
                recoveryMessage: root.service ? root.service.recoveryMessage : ""
                canAcknowledge: root.service ? root.service.canAcknowledge : false
                onAcknowledgmentRequested: function(key) { if (root.service) root.service.acknowledge(key); }
                contextKey: root.controller ? root.controller.contextKey : ""
                surfaceColor: Commons.Color.menu.background
                foregroundColor: Commons.Color.menu.text
                mutedColor: Commons.Color.muted
                borderColor: Commons.Color.menu.border
                onActionRequested: function(action, observedContext) {
                    if (root.service) root.service.requestAction(action,observedContext);
                }
            }
        }
    }
}
