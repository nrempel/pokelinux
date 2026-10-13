import QtQuick
import QtQuick.Controls
Button {
    id: root
    required property QtObject theme
    property color fill: theme.surface
    property color ink: theme.foreground
    property string contextKey: ""
    property string pressedContext: ""
    signal invoked()
    implicitHeight: 56
    implicitWidth: 120
    font.family: theme.fontFamily
    font.pixelSize: theme.fontPixel ? theme.fontPixel("title",17) : Math.max(1, Math.round(17 * theme.fontScale))
    font.weight: Font.DemiBold
    Accessible.name: text
    onPressed: pressedContext = contextKey
    onCanceled: pressedContext = ""
    onContextKeyChanged: pressedContext = ""
    onClicked: {
        if (enabled && contextKey !== "" && pressedContext === contextKey) invoked()
        pressedContext = ""
    }
    background: Rectangle {
        color: root.down ? Qt.darker(root.fill, 1.15) : root.fill
        opacity: root.enabled ? 1 : 0.45
        radius: 4
        border.width: root.activeFocus ? 2 : 1
        border.color: root.activeFocus ? root.theme.foreground : root.theme.border
    }
    contentItem: Text {
        text: root.text; font: root.font; color: root.ink
        opacity: root.enabled ? 1 : 0.55
        horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
        textFormat: Text.PlainText
    }
}
