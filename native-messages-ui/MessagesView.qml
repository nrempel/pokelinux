import QtQuick
import QtQuick.Controls
Rectangle {
 id: root
 required property QtObject theme
 required property QtObject messagesClient
 required property QtObject batteryClient
 color: theme.background
 component MessageButton: Button {
  id: button
  height: 48
  font.family: root.theme.fontFamily
  font.pixelSize: root.theme.fontPixel("body",16)
  contentItem: Text {
   text: button.text; textFormat: Text.PlainText
   color: button.down ? root.theme.ink : button.enabled ? root.theme.foreground : root.theme.muted
   font: button.font; verticalAlignment: Text.AlignVCenter; horizontalAlignment: Text.AlignHCenter
   elide: Text.ElideRight
  }
  background: Rectangle {
   color: button.down ? root.theme.accent : root.theme.surface
   border.color: button.activeFocus ? root.theme.accent : root.theme.border
   border.width: button.activeFocus ? 2 : 1
  }
 }
 Flickable {
  id: scroll
  anchors.fill: parent; anchors.margins: root.theme.spacingPixel("panelPadding",18)
  contentWidth: width; contentHeight: column.implicitHeight; clip: true
  ScrollBar.vertical: ScrollBar {}
  Column {
   id: column; width: scroll.width
   spacing: root.theme.spacingPixel("panelGap",14)
   Text { text: "Messages"; textFormat: Text.PlainText; color: root.theme.foreground; font.family: root.theme.fontFamily; font.pixelSize: root.theme.fontPixel("heading",24) }
   Text {
    objectName: "batteryStatus"
    text: {
     if (!root.batteryClient.available) return "Battery unavailable"
     const status = root.batteryClient.status
     const label = status === "charging" ? "Charging" : status === "discharging" ? "Discharging" : status === "full" ? "Full" : status === "not_charging" ? "Not charging" : "Status unknown"
     return root.batteryClient.capacityPercent + "% · " + label
    }
    textFormat: Text.PlainText
    color: root.theme.muted
    font.family: root.theme.fontFamily
    font.pixelSize: root.theme.fontPixel("caption",12)
   }
   Text { text: root.messagesClient.status; textFormat: Text.PlainText; width: parent.width; wrapMode: Text.Wrap; color: root.theme.muted; font.family: root.theme.fontFamily; font.pixelSize: root.theme.fontPixel("caption",12) }
   Repeater { model: root.messagesClient.conversations
    delegate: MessageButton { required property var modelData; width: column.width
     text: modelData.sender
     onClicked: root.messagesClient.select(modelData.id)
    }
   }
   Row { spacing: root.theme.spacingPixel("controlGap",8)
    MessageButton { text:"First chats"; onClicked: root.messagesClient.firstConversations() }
    MessageButton { text:"Next chats"; enabled:root.messagesClient.conversations.length>0; onClicked: root.messagesClient.nextConversations() }
   }
   Repeater { model: root.messagesClient.messages
    delegate: Rectangle { required property var modelData; width: column.width; height: body.implicitHeight+24; color: root.theme.surface; border.color: root.theme.border
     Text { id: body; objectName: "messageBody"; anchors.margins: 12; anchors.fill: parent; text: modelData.text; textFormat: Text.PlainText; wrapMode: Text.Wrap; color: root.theme.foreground; font.family: root.theme.fontFamily; font.pixelSize: root.theme.fontPixel("body",16) }
    }
   }
   Row { spacing: root.theme.spacingPixel("controlGap",8)
    MessageButton { text:"First page"; onClicked: root.messagesClient.firstPage() }
    MessageButton { text:"Next page"; enabled:root.messagesClient.messages.length>0; onClicked: root.messagesClient.nextPage() }
   }
  }
 }
}
