import QtQuick
import QtQuick.Controls
Rectangle {
    id: root
    required property QtObject theme
    required property QtObject permissionClient
    readonly property bool audioScope: permissionClient.scope === 1
    readonly property bool networkScope: permissionClient.scope === 2
    readonly property string permissionName: audioScope ? "Audio" : networkScope ? "Network control" : ""
    color: theme.background
    component ChoiceButton: Button {
        id: button
        height: 56
        font.family: root.theme.fontFamily
        font.pixelSize: root.theme.fontPixel("body", 16)
        contentItem: Text {
            text: button.text
            textFormat: Text.PlainText
            font: button.font
            color: button.down ? root.theme.ink : button.enabled ? root.theme.foreground : root.theme.muted
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
        background: Rectangle {
            color: button.down ? root.theme.accent : root.theme.surface
            border.color: button.activeFocus ? root.theme.accent : root.theme.border
            border.width: button.activeFocus ? 2 : 1
        }
    }
    Flickable {
        id: scroll
        anchors.fill: parent
        anchors.margins: root.theme.spacingPixel("panelPadding", 18)
        contentWidth: width
        contentHeight: column.implicitHeight
        clip: true
        ScrollBar.vertical: ScrollBar {}
        Column {
            id: column
            width: scroll.width
            spacing: root.theme.spacingPixel("panelGap", 14)
            Text {
                text: "Permissions"
                color: root.theme.foreground
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("heading", 24)
            }
            Text {
                text: root.audioScope ? "Sounds" : root.networkScope ? "Settings" : ""
                color: root.theme.foreground
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("title", 20)
            }
            Text {
                width: parent.width
                text: root.audioScope ? "Let Sounds play audio." : root.networkScope ? "Let Settings control the local connection." : ""
                wrapMode: Text.Wrap
                color: root.theme.foreground
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("body", 16)
            }
            Text {
                objectName: "savedPermission"
                text: root.permissionName && root.permissionClient.allowed === 1 ? root.permissionName + " allowed" : root.permissionName && root.permissionClient.allowed === 0 ? root.permissionName + " not allowed" : "Permission unavailable"
                width: parent.width
                wrapMode: Text.Wrap
                color: root.theme.foreground
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("body", 16)
            }
            ChoiceButton {
                objectName: "allowPermission"
                width: parent.width
                text: root.audioScope ? "Allow audio" : root.networkScope ? "Allow network control" : "Allow"
                enabled: root.permissionClient.canSet && root.permissionClient.allowed !== 1
                onClicked: root.permissionClient.choose(true)
            }
            ChoiceButton {
                objectName: "revokePermission"
                width: parent.width
                text: root.audioScope ? "Revoke audio" : root.networkScope ? "Revoke network control" : "Revoke"
                enabled: root.permissionClient.canSet && root.permissionClient.allowed !== 0
                onClicked: root.permissionClient.choose(false)
            }
            Text {
                width: parent.width
                text: root.permissionClient.unconfirmed ? "Couldn't confirm the last change." : root.audioScope ? "Start playback in Sounds after allowing audio." : root.networkScope ? "Enable the local connection in Settings after allowing control." : ""
                wrapMode: Text.Wrap
                color: root.theme.muted
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("caption", 12)
            }
        }
    }
}
