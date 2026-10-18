import QtQuick
import QtQuick.Controls
Rectangle {
    id: root
    required property QtObject theme
    required property QtObject settingsClient
    required property QtObject networkStatusClient
    required property QtObject connectivityClient
    color: theme.background
    component BrightnessButton: Button {
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
                text: "Settings"
                textFormat: Text.PlainText
                color: root.theme.foreground
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("heading", 24)
            }
            Text {
                text: "Display brightness"
                textFormat: Text.PlainText
                color: root.theme.foreground
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("title", 20)
            }
            Text {
                objectName: "confirmedBrightness"
                text: {
                    const value = root.settingsClient.confirmedPercent
                    return value === null || value === undefined ? "Brightness unavailable" : "Brightness: " + value + "%"
                }
                textFormat: Text.PlainText
                width: parent.width
                wrapMode: Text.Wrap
                color: root.theme.foreground
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("body", 16)
            }
            Text {
                objectName: "brightnessStatus"
                text: root.settingsClient.status
                textFormat: Text.PlainText
                width: parent.width
                wrapMode: Text.Wrap
                color: root.theme.muted
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("caption", 12)
            }
            Row {
                width: parent.width
                spacing: root.theme.spacingPixel("controlGap", 8)
                BrightnessButton {
                    objectName: "brightness25"
                    width: (parent.width - parent.spacing) / 2
                    text: "25%"
                    enabled: root.settingsClient.canSet
                    onClicked: root.settingsClient.setPercent(25)
                }
                BrightnessButton {
                    objectName: "brightness75"
                    width: (parent.width - parent.spacing) / 2
                    text: "75%"
                    enabled: root.settingsClient.canSet
                    onClicked: root.settingsClient.setPercent(75)
                }
            }
            Text {
                objectName: "connectivityTitle"
                text: "Local connection"
                textFormat: Text.PlainText
                color: root.theme.foreground
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("title", 20)
            }
            Text {
                objectName: "connectivityState"
                text: {
                    const value = root.connectivityClient.configured
                    return "Local connection: " + (value === null || value === undefined ? "Unavailable" : value ? "Enabled" : "Disabled")
                }
                textFormat: Text.PlainText
                width: parent.width
                wrapMode: Text.Wrap
                color: root.theme.foreground
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("body", 16)
            }
            Text {
                objectName: "connectivityStatus"
                text: root.connectivityClient.status
                textFormat: Text.PlainText
                width: parent.width
                wrapMode: Text.Wrap
                color: root.theme.muted
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("caption", 12)
            }
            Row {
                width: parent.width
                spacing: root.theme.spacingPixel("controlGap", 8)
                BrightnessButton {
                    objectName: "connectivityEnable"
                    width: (parent.width - parent.spacing) / 2
                    text: "Enable"
                    enabled: root.connectivityClient.canControl && root.connectivityClient.configured === false
                    onClicked: root.connectivityClient.setConnected(true)
                }
                BrightnessButton {
                    objectName: "connectivityDisable"
                    width: (parent.width - parent.spacing) / 2
                    text: "Disable"
                    enabled: root.connectivityClient.canControl && root.connectivityClient.configured === true
                    onClicked: root.connectivityClient.setConnected(false)
                }
            }
            Text {
                objectName: "connectivityCaption"
                text: "Connection state does not indicate internet access."
                textFormat: Text.PlainText
                width: parent.width
                wrapMode: Text.Wrap
                color: root.theme.muted
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("caption", 12)
            }
            Text {
                objectName: "localNetworkTitle"
                text: "Local network status"
                textFormat: Text.PlainText
                color: root.theme.foreground
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("title", 20)
            }
            Text {
                objectName: "localNetworkInterfaces"
                text: {
                    const value = root.networkStatusClient.localInterfaces
                    return "Network interfaces: " + (value === null || value === undefined ? "—" : value)
                }
                textFormat: Text.PlainText
                width: parent.width
                wrapMode: Text.Wrap
                color: root.theme.foreground
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("body", 16)
            }
            Text {
                objectName: "localNetworkAddresses"
                text: {
                    const value = root.networkStatusClient.addressRecords
                    return "Address records: " + (value === null || value === undefined ? "—" : value)
                }
                textFormat: Text.PlainText
                width: parent.width
                wrapMode: Text.Wrap
                color: root.theme.foreground
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("body", 16)
            }
            Text {
                objectName: "localNetworkCaption"
                text: "Counts do not indicate internet access."
                textFormat: Text.PlainText
                width: parent.width
                wrapMode: Text.Wrap
                color: root.theme.muted
                font.family: root.theme.fontFamily
                font.pixelSize: root.theme.fontPixel("caption", 12)
            }
        }
    }
}
