import QtQuick
import QtQuick.Controls
Rectangle {
 id: root
 required property QtObject theme
 required property QtObject audioClient
 color: theme.background
 component SoundButton: Button {
  id: button
  height: 56
  font.family: root.theme.fontFamily
  font.pixelSize: root.theme.fontPixel("body",16)
  contentItem: Text {
   text: button.text; textFormat: Text.PlainText; font: button.font
   color: button.down ? root.theme.ink : button.enabled ? root.theme.foreground : root.theme.muted
   verticalAlignment: Text.AlignVCenter; horizontalAlignment: Text.AlignHCenter; elide: Text.ElideRight
  }
  background: Rectangle { color: button.down ? root.theme.accent : root.theme.surface; border.color: button.activeFocus ? root.theme.accent : root.theme.border; border.width: button.activeFocus ? 2 : 1 }
 }
 Flickable {
  id: scroll
  anchors.fill: parent; anchors.margins: root.theme.spacingPixel("panelPadding",18)
  contentWidth: width; contentHeight: column.implicitHeight; clip: true
  ScrollBar.vertical: ScrollBar {}
  Column {
   id: column; width: scroll.width; spacing: root.theme.spacingPixel("panelGap",14)
   Text { text: "Sounds"; textFormat: Text.PlainText; color: root.theme.foreground; font.family: root.theme.fontFamily; font.pixelSize: root.theme.fontPixel("heading",24) }
   Text { text: "Media output"; textFormat: Text.PlainText; color: root.theme.foreground; font.family: root.theme.fontFamily; font.pixelSize: root.theme.fontPixel("subtitle",18) }
   Text { text: root.audioClient.status; textFormat: Text.PlainText; width: parent.width; wrapMode: Text.Wrap; color: root.theme.muted; font.family: root.theme.fontFamily; font.pixelSize: root.theme.fontPixel("body",16) }
   Repeater { model: root.audioClient.routes
    delegate: SoundButton { required property var modelData; width: column.width
     objectName: "route_"+modelData.id
     text: modelData.label+(root.audioClient.applied===modelData.id ? " · Applied" : root.audioClient.requested===modelData.id ? " · Requested" : "")
     enabled: root.audioClient.canChoose
     onClicked: root.audioClient.select(modelData.id)
    }
   }
   SoundButton { width: column.width; text: "Reconnect"; visible: root.audioClient.canReconnect; onClicked: root.audioClient.reconnect() }
  }
 }
}
