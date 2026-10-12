import QtQuick
import QtQuick.Controls

Button {
    id: root
    property color actionColor: "#9ece6a"
    property color inkColor: "#101315"
    property color focusColor: "#ffffff"
    property string contextKey: ""
    property string pressedContext: ""
    signal actionTriggered(string observedContext)
    onPressed: pressedContext = contextKey
    onCanceled: pressedContext = ""
    onClicked: {
        var captured = pressedContext;
        pressedContext = "";
        actionTriggered(captured);
    }
    implicitHeight: 64
    implicitWidth: 144
    focusPolicy: Qt.StrongFocus
    Accessible.role: Accessible.Button
    contentItem: Text {
        text: root.text
        textFormat: Text.PlainText
        color: root.inkColor
        font.pixelSize: 20
        font.weight: Font.DemiBold
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }
    background: Rectangle {
        radius: 4
        color: root.down ? Qt.darker(root.actionColor, 1.18) : root.actionColor
        border.width: root.activeFocus ? 3 : 0
        border.color: root.focusColor
        opacity: root.enabled ? 1 : 0.4
    }
}
